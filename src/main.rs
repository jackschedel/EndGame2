use hashbrown::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::str::SplitWhitespace;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{fmt, thread};

#[derive(Debug, Copy, Clone, PartialEq)]
enum Color {
    Black,
    White,
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Piece {
    Pawn(Color),
    Knight(Color),
    Bishop(Color),
    Rook(Color),
    Queen(Color),
    King(Color),
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum HalfmoveFlag {
    KnightPromotion,
    BishopPromotion,
    RookPromotion,
    QueenPromotion,
    Castle,
    EnPassant,
    DoublePawnMove,
}

#[derive(PartialEq, Clone, Copy)]
struct HalfMove {
    from: u8,
    to: u8,
    flag: Option<HalfmoveFlag>,
    is_capture: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct ColorCastlingRights {
    kingside: bool,
    queenside: bool,
}

#[derive(Clone)]
struct PieceSet {
    all: HashSet<u8>,
    white: HashSet<u8>,
    black: HashSet<u8>,
    white_king: u8,
    black_king: u8,
}

#[derive(Clone, Debug, PartialEq)]
struct CastlingRights {
    black: ColorCastlingRights,
    white: ColorCastlingRights,
}

#[derive(Clone)]
struct Position {
    board: [Option<Piece>; 64],
    piece_set: PieceSet,
    move_next: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<u8>,
    halfmove_clock: u16,
    fullmove_number: u16,
}

#[derive(Clone)]
struct PositionTree {
    nodes: Vec<Vec<PositionTreeNode>>,
    position: Position,
    depth: usize,
    leaf_size: usize,
}

#[derive(Clone)]
struct PositionTreeNode {
    // parent, top_parent, and halfmove for root don't matter.
    // using option here would be inefficient
    parent: usize,
    children: Option<(usize, usize)>,
    halfmove: HalfMove,
    score: i32,
}

impl Color {
    fn opposite(&self) -> Color {
        match *self {
            Color::Black => Color::White,
            Color::White => Color::Black,
        }
    }
}

impl Piece {
    fn get_color(&self) -> Color {
        match self {
            Piece::Pawn(color)
            | Piece::Knight(color)
            | Piece::Bishop(color)
            | Piece::Rook(color)
            | Piece::Queen(color)
            | Piece::King(color) => *color,
        }
    }

    fn get_cp_val(&self) -> u16 {
        match self {
            Piece::Pawn(_) => 100,
            Piece::Knight(_) => 320,
            Piece::Bishop(_) => 290,
            Piece::Rook(_) => 490,
            Piece::Queen(_) => 900,
            Piece::King(_) => 60000,
        }
    }
}

impl fmt::Debug for HalfMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.flag == None {
            return write!(f, "[{} {}]", int_to_coord(self.from), int_to_coord(self.to));
        } else {
            return write!(
                f,
                "[{:?} {} {}]",
                self.flag.as_ref().unwrap(),
                int_to_coord(self.from),
                int_to_coord(self.to)
            );
        }
    }
}

impl HalfMove {
    fn move_to_coords(&self) -> String {
        let promotion_str;

        match self.flag {
            Some(HalfmoveFlag::QueenPromotion) => promotion_str = "q",
            Some(HalfmoveFlag::RookPromotion) => promotion_str = "r",
            Some(HalfmoveFlag::KnightPromotion) => promotion_str = "k",
            Some(HalfmoveFlag::BishopPromotion) => promotion_str = "b",

            _ => promotion_str = "",
        }

        if self.flag == Some(HalfmoveFlag::Castle) {
            if self.from == 4 {
                if self.to == 0 {
                    return "e1c1".to_string();
                } else {
                    return "e1g1".to_string();
                }
            } else {
                if self.to == 56 {
                    return "e8c8".to_string();
                } else {
                    return "e8g8".to_string();
                }
            }
        }

        return format!(
            "{}{}{}",
            int_to_coord(self.from),
            int_to_coord(self.to),
            promotion_str
        );
    }
}

impl fmt::Debug for PieceSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut all_string = String::from("All:");
        let mut sorted_all: Vec<u8> = self.all.iter().cloned().collect();
        sorted_all.sort_unstable();

        for i in sorted_all {
            all_string += " ";
            all_string += &int_to_coord(i);
        }

        let mut white_string = String::from("White:");
        let mut sorted_white: Vec<u8> = self.white.iter().cloned().collect();
        sorted_white.sort_unstable();

        for i in sorted_white {
            white_string += " ";
            white_string += &int_to_coord(i);
        }

        let mut black_string = String::from("Black:");
        let mut sorted_black: Vec<u8> = self.black.iter().cloned().collect();
        sorted_black.sort_unstable();

        for i in sorted_black {
            black_string += " ";
            black_string += &int_to_coord(i);
        }

        return write!(f, "{}\n{}\n{}", all_string, white_string, black_string);
    }
}

impl PieceSet {
    fn remove_index(&mut self, index: u8, color: Color) {
        self.all.remove(&index);

        if color == Color::Black {
            self.black.remove(&index);
        } else {
            self.white.remove(&index);
        }
    }

    fn add_index(&mut self, index: u8, color: Color) {
        self.all.insert(index);

        if color == Color::Black {
            self.black.insert(index);
        } else {
            self.white.insert(index);
        }
    }

    fn add_index_or_color_swap(&mut self, index: u8, color: Color) {
        self.all.insert(index);

        if color == Color::Black {
            self.black.insert(index);
            self.white.remove(&index);
        } else {
            self.white.insert(index);
            self.black.remove(&index);
        }
    }
}

impl PositionTree {
    fn from_pos(position: Position) -> Self {
        Self {
            position,
            nodes: vec![vec![PositionTreeNode::root_node()]],
            depth: 0,
            leaf_size: 0,
        }
    }

    fn print_tree(&self) {
        for i in 0..self.nodes.len() {
            println!();
            println!("Depth: {}", i);
            for j in 0..self.nodes[i].len() {
                self.nodes[i][j].print_node();
                if j != self.nodes[i].len() - 1 {
                    print!("| ");
                }
            }
            println!();
        }
        println!();
    }

    fn gen_children(&mut self, depth: usize, index: usize) {
        let mut position = self.position.clone();

        let mut trace = vec![];
        let mut cur_depth = depth;
        let mut cur_index = index;
        while cur_depth > 0 {
            trace.push(cur_index);
            cur_index = self.nodes[cur_depth][cur_index].parent;

            cur_depth -= 1;
        }
        trace.reverse();

        cur_depth = 1;
        for i in 0..trace.len() {
            execute_halfmove(&mut position, self.nodes[cur_depth][trace[i]].halfmove);
            cur_depth += 1;
        }

        let moves = gen_possible(&mut position);

        self.leaf_size += moves.len();

        if !moves.is_empty() {
            if self.nodes.len() <= depth + 1 {
                self.nodes.push(vec![]);
            }

            let orig_len = self.nodes[depth + 1].len();
            for i in 0..moves.len() {
                let child_node = PositionTreeNode {
                    parent: index,
                    halfmove: moves[i].to_owned(),
                    children: None,
                    score: 0,
                };
                self.nodes[depth + 1].push(child_node);
            }

            let child_range = Some((orig_len, orig_len + moves.len() - 1));
            self.nodes[depth][index].children = child_range;
        }
    }

    fn increase_depth(&mut self) -> usize {
        if self.nodes.len() == 0 {
            return 0;
        }

        for i in 0..self.nodes[self.depth].len() {
            if self.nodes[self.depth][i].children.is_some() {
                continue;
            }

            self.gen_children(self.depth, i);
        }
        self.depth += 1;

        return self.nodes[self.depth].len();
    }
}

impl PositionTreeNode {
    fn root_node() -> Self {
        Self {
            parent: 0,
            halfmove: HalfMove {
                from: 63,
                to: 63,
                flag: None,
                is_capture: false,
            },
            children: None,
            score: 0,
        }
    }

    fn print_node(&self) {
        if self.halfmove.to == 63 && self.halfmove.from == 63 {
            print!("root");
        } else {
            print!("{}", self.halfmove.move_to_coords());
        }
    }
}

impl Position {
    fn gen_hash(&self) -> u64 {
        let mut hash: u64 = 0;

        #[rustfmt::skip]
        let psrn_table: [u64; 781] = [6050961064690644123, 15385182941806993281, 1474049585344358660, 6851573923483025534, 13899087919403525125, 8758650992845187116, 1831239503027593786, 13701660087018851169, 18335348291191493899, 4402234053541100678, 14757096522167036102, 13009140431848805653, 1292898825854068034, 4884307846020727494, 13857947210706460393, 1626896879833203751, 6038445616195308722, 6720134536466369422, 4497292822882533224, 12369361321546040904, 14712685727521284085, 1608341193440387084, 4094586736089739280, 2072304564850959527, 4091162237664628960, 15417717071469061328, 158710210446366970, 10118476861800698006, 16261210225467785938, 3509118041234889229, 6369150832245265647, 16079384263440389010, 11115231651891558388, 4646006308786422360, 18110725773482173731, 5657782342379456300, 11143381484293096337, 7487773842973491479, 6751517840915511657, 14929942954797253082, 1901957234508141725, 8921907195207315801, 17463714160121970869, 12245751322195944246, 10654386101703818407, 3931494334593277793, 17115885933089799525, 8502883217534375488, 745914388038295655, 1034741315093060365, 9433678509952610578, 1098536606267845662, 13213316387606432785, 3350954517876542623, 11207000871705408100, 10414442641064136232, 16749912713695375096, 16740481193746264268, 15559897978749864387, 10170635327641382168, 13139853202089369670, 14022649397309221013, 11247166396668734960, 14500993388554649383, 10234231535682188861, 16082651711303738385, 13240344989764749555, 15761415548747030129, 13200626097523845685, 10158384463413159211, 12082793007871671521, 7053088165737182306, 17910572772996987755, 634551525556320577, 8715072720248632882, 16645249778365519939, 17071269256303802149, 13000434989816980991, 3266080034350421129, 10387188012931609076, 12909971265520579520, 2232707469466001278, 12247075673661908260, 2073603714481317363, 901131989421222986, 17687777256174267121, 10628670673870316880, 10335258412280339222, 13252625444758210862, 13244768822050161111, 11902193789785886843, 4557300638221084616, 15723110200581411395, 2002686390970716135, 3679706203300853541, 8465679685848505392, 15629865532713611859, 2252635975746926934, 13176514338201280970, 18323437376244292447, 3052078822704129486, 14668753997257336776, 7484590864270466728, 9116309183190979995, 17775487882875822414, 773122596006458714, 17641850471318846102, 15107524460097819202, 7418208085589646254, 8561007928848469504, 15315171626020440806, 5431303807153869368, 18338711474901845704, 15830389477933775901, 943683194046629764, 12756255220308303970, 15818202731076003553, 17099604802677736889, 4050058657376309133, 17788592446092713641, 12125532480504028469, 7346811925904984991, 16743490842944433249, 12915895388335722275, 13798990182043546430, 10651826920167390121, 17969822964227303393, 4544407742359086458, 10681790818208387649, 14722634122293894088, 7546947474351786735, 3105849400590956960, 8113431633459910815, 16846295435437074212, 2884719721752435755, 11434748994118687031, 8592015634217955360, 12804867188244779916, 11198362185301661234, 11893160967421070986, 5713328646749870157, 3376500401699912934, 12396827523520345466, 16163415384865807273, 10631825877706701086, 18362438055956926458, 7091289048407828922, 2601597000512142188, 5934351981512336591, 18009871113071078878, 5067636467652884776, 13664982911664380293, 11250428350774470275, 15195462104258779713, 3761708893439855811, 5714505373559617613, 16070201332416855208, 7836116975836822007, 6610470649036680618, 18340614937879377979, 16747532071404513809, 13866875191998180171, 2046326353399532111, 14152787502496138315, 17862055635526226878, 3935530809429155555, 14407604056361705041, 17819032531253250211, 7012195161138792524, 843324294862535766, 11284107948253343080, 1749165026438999140, 17365438740212629834, 529057808325496683, 11364771066596107837, 11856258599114527383, 4316973369925240629, 5243288441161619140, 894022035255586177, 16853695020805006493, 10797222682704016790, 5858313985552783408, 10237723180844500384, 15304820458373535844, 17850530461622689681, 14894060435840074976, 14427026045903430902, 104617213228060690, 7640074872228573677, 7573980051921992697, 10305090662346373726, 18307325185753646832, 3253083594076551494, 12756449958142110556, 7986408859512743752, 4976782687715554697, 6758736852197655040, 1033181489679567150, 14155585553909016816, 4249394446353065408, 17942940693848032142, 3535312454936521939, 13154155077310235819, 6615990194370558678, 11838970440518365616, 17082754182448501336, 8146609427596499162, 2225872567139137754, 5812928537890751298, 7002225902229134612, 15122223306994340390, 8811643324484140341, 4240177335615464473, 1263622195699005784, 4937788903724975379, 8710761994859176931, 11579355844267439659, 14762877258348145194, 9340761068251229970, 2965544404013391364, 7474830457601152485, 17004451485192980313, 836167104639626113, 16192472709886055895, 5567772969564176462, 14711296339676478873, 9064257581222141987, 289450938860923833, 9437036928624368577, 17319187955177794104, 14705260853599714953, 2938102596797146997, 13053614869271975353, 7811262463056009475, 10471781887007966218, 6318402700082491738, 12859742873462550346, 13581778329009260002, 2191427603160772933, 2912587536796309376, 12190681911391377435, 2121662344245551616, 11940356828758808627, 8579633679480549070, 17748750722896289810, 9922686718031707817, 9890729722693482208, 1738413465528204104, 12106772477101032553, 10343326210733605168, 10521792142915609879, 9133206837523597081, 14228057140258989546, 16629749403701501368, 8057453397540486664, 14771587299335891728, 12542639368862350092, 17033018684229091182, 14299417385609581513, 299895395448337771, 18261141907208659512, 58435901761234140, 1029815525973352126, 16667980257426781041, 10364293774554972990, 14118057326965178932, 13217797211731137055, 7331073934442150546, 13516155712980895236, 3849197493611392794, 6311397283325561707, 5734118818395547438, 4867368830777807010, 9287369375107932908, 2926380459256882904, 18359136274642055492, 6157115134594143556, 15083371154181254693, 14506803485078401988, 10100223926074614734, 17009608990384185248, 13503764453345526380, 8209605655417046357, 10908528342113814552, 5270672473694595866, 5227971298844608744, 2079841133548231047, 9716184702400726114, 16198418592916683571, 3228342983974782177, 14635980218870688079, 13550371517618278327, 12669594339150634112, 11591207933534184271, 9564796019405425199, 2501218974170272794, 1327476418968706882, 3168866119130897463, 8425176289155694011, 4253645703623642439, 4012572788319560961, 14748287885898588380, 15913389759861721802, 9004133984019784852, 5915021421986852130, 5629928874286919288, 4221326977905064881, 2510275727066081252, 8126081520560038169, 5696364608254310546, 10956502371156347231, 3256132070556573989, 16347019016654603140, 5002654120378261107, 2093733417751210425, 18015440295385245672, 5699852561200492883, 10706954589778002309, 3296275905849577026, 2965778812108887194, 4418827907814509781, 11190035921018846823, 4212926398119039131, 2172920485357587036, 5417674759529146084, 12559822789700806847, 6420030248204950146, 11556884813285663168, 17078599768159079822, 9457541948057297374, 5294206209553005609, 10417300360929566980, 16196327365681227323, 6395469077661940900, 4708532786143680622, 10654194123371921563, 9651553495607851035, 15014301726845382732, 12035066491922951630, 12561246240444678516, 11978111492276933879, 990166153752483250, 17569815533005884963, 10194498563663234464, 10928768676372136124, 14796581717184468331, 9723175147088108129, 5810018124754208806, 15176267457803663891, 10020041885928112913, 18310674336842914861, 6397752648784716519, 17225786258546997877, 1789197968025353863, 10403293684791961098, 18144238680550519661, 4576865157808586296, 12574838769490753335, 7897053966609216911, 17669716723430272262, 16887123826806941351, 14567216391692586257, 14148853342514078053, 15543561537766863720, 13179732114149938262, 15548110259070525182, 8241184539042406975, 12886167617719501434, 2668442484504456534, 5528737578750256550, 8045724552511222249, 14233549524182091382, 14454773653496152899, 4803382709611342105, 14104305995325012156, 9233416359162608765, 10896765243687396087, 3888622613535254020, 5204363573751428905, 15550077342098247025, 14057215280926617785, 6429067651734432800, 12145175219210066021, 3871151583257216929, 14382153126479391325, 3503136348951471139, 17074632318198699960, 18277337232990677676, 2438793295253393259, 14188276604425453834, 17190081648445444068, 14901372647638775549, 10575384267303610410, 12463714381430437920, 5296503864634704402, 1085506994095541018, 17711931477255281454, 10029194223911757044, 10755199144959386844, 8671868823094321814, 5983006676130798566, 10792592475280434339, 722608211743985546, 14482132779275271146, 9415512828898525574, 16956058083295549808, 9209857238564496465, 12683605268868586743, 2955776406012433258, 16029235202218082952, 6579160785242750161, 504295306752149147, 9341624862273318372, 15375265198034115277, 6994898638369110070, 309797721354564726, 12429410516424851772, 5192024237253378865, 3912399787570959755, 12541234326966170226, 18441632327004496392, 7346203468976882923, 17593945557702212252, 15367442556011108555, 18217099153608305021, 10157165767144106959, 14018728927678812016, 18099686645005791370, 15136980239802015388, 7047521305623726125, 7575245649510417331, 14278619717007843644, 556011385191822492, 11381450268477688895, 14606319637689027024, 7080222843433955438, 5535489633773271511, 12090789406220893065, 14588818283718185151, 15370484225886308308, 12506301389557425466, 14865276370451418685, 8307888451349003948, 2861458479835086804, 5979069397180909905, 3140261739536988441, 5512756738929686408, 62084764907834261, 15807114778163996394, 2514213451484910157, 12101977943332277088, 5754338443349951926, 11202526598411612289, 1941284846376634320, 13676463015082195127, 13512152708144120784, 15967285827171943566, 1414500093148047241, 12815445217859919773, 11408657165942473469, 13896534001351748553, 12170732773640396882, 13528711356234590625, 13396280905236298091, 526652414431385131, 18204997071569618430, 4672075988794117180, 10712277614075303886, 10462100441247111006, 15579071806953301277, 15286269530449908638, 5479544935618236438, 7078561675539836269, 14897271087535510231, 6663607476483075550, 10975108262709842260, 5164218779845057244, 3026027211361889997, 10372550805396296371, 5511181984466209111, 4615344310383006121, 14765022018300614504, 7941633078349736364, 2336972229937914156, 15572100879945226778, 4252302396980455049, 12550177359955319593, 1459460872050639652, 17262569865062661057, 1539903875688234572, 17611947439799518453, 2703027075660991994, 11637679138689401581, 3064316072640710608, 1680909683069808723, 3607511184654591057, 4108173343691407894, 17379562958700858326, 302916588307766784, 3080575744190946064, 14618492782227969892, 7410866293301888883, 15107711256805849837, 4154183358968633778, 228080548283573131, 4117428293408729535, 20393270934047095, 6924832010164006882, 9829266407182768870, 1479756068945379597, 13132187458871599966, 8408723953761582692, 7925131402231319271, 4163854595303398243, 11230101227039602264, 16993193842701891622, 15444679853316663011, 3781919890769373844, 11182705188793031493, 5892311539960805112, 17158673965144059034, 16226450487359544767, 3857937074244810267, 10290970525402511515, 4090612527962514610, 15705494108227347854, 11713886306567047904, 11839618259637189525, 8015231900599896429, 14318494365807990907, 17066719705795494095, 1191101778471427856, 18366858155298147659, 9909682530008047655, 18103868884984506862, 18272414527621650028, 8396690449634257845, 541665888372703491, 8880466152303936336, 7116327037981094726, 14787688634394995663, 10394250631058185299, 10941901494326388747, 2555388952390999332, 3758236094703160891, 16576737194033338957, 14366906953111661451, 1903270876738280544, 10324021488998625612, 18353689077818956100, 8991522840099717154, 2421074737169331519, 9169793787044604812, 18260962835765091438, 4114111187649682384, 12816926656461667356, 17481819214090174809, 10131753959629909294, 12546401621311663568, 7179244263615447903, 3726159482382699804, 4915138607684722647, 17168907241619308384, 12339912791745187348, 1707583925986994553, 14011204057319936567, 10794690787627844528, 10695852063656574836, 3197783774491593781, 7298933884713059525, 6328633030694775205, 10766434409434719553, 10091956128572215514, 3455431069320366557, 392899140943852740, 4786988958218928946, 17290118779266618583, 9569754117035606215, 9608745232397807979, 5317990318280560256, 1174821456301900773, 9629429049860584332, 16050528676160605532, 8553649108826978033, 9401175273538018431, 16154633128515230798, 1905181735190354887, 14357904420278363879, 8896250678213174483, 4757364172887264470, 534375847936343030, 4168770809732413905, 5319475466728698669, 16027717470825227410, 16290862895133357951, 7585575570172258495, 7376450908955157397, 105693702558868570, 5867124974882586974, 10358598132073172602, 12687742061092614126, 10033659482067303133, 6046794178657639025, 5682077375508511604, 16085410403491061574, 2221968166217602761, 14054121017115688694, 9999403329710406488, 14173309310608438308, 2213865887714781468, 10095989436830741386, 6831589881469508820, 2210779362904169838, 17870372339043947522, 926838333319636069, 14155028843993162273, 3789607254882896617, 11944486143104258955, 7877247516668015598, 13403475833651114537, 6804136318190869572, 15941814127752619716, 2870163589488692723, 3380428415866679688, 17869614886487731048, 12478570456242254441, 15953546942867752514, 14008141154075371973, 3261688501716887849, 8115949271760655919, 4123026970301930621, 6389926672597484019, 1434651534731442627, 17520271334962967152, 16176207172243961455, 12191313098201911592, 11829326229021738049, 3441526450024664451, 14672768068246754822, 6091054180467864913, 17310674220118407423, 10133704405879849808, 5245870133028354084, 5699111173180951384, 424903395538216595, 7043252863064238904, 15030900898582281980, 6047080229948242541, 6673848437141978636, 7619119509346272531, 1176862265596481243, 1432562585694455670, 14207256064924225720, 17177014215747555751, 12404913789792397349, 17142808236991882077, 12206497933940300112, 17239638110422550477, 9541260587948853653, 5382239951718353022, 12461479961770469628, 5378179547293175807, 17788513785887631264, 2769065464121114364, 6698553467183667369, 12881128031963500700, 15757831429553553394, 13546128785342036562, 12785217889406033884, 5788314480727360675, 2793556930718848067, 2569518059303078779, 9235865686928466768, 8559980265714462943, 1268367044108754402, 10691882615180890276, 15531362923319305157, 3713025523581636531, 17821743846441991997, 4052839474685653212, 1964950709534779841, 3077048493964259228, 4680455791302938987, 14635728993302465994, 16761081135007356997, 14169913184783073434, 15297891832366291834, 2318207228039675382, 8504436602692556662, 8742349526436801363, 6126332799630915405, 5684354393344322870, 786183764801356732, 2343749936100637379, 720083360215038549, 5698685623915037082, 5915927667934393073, 9653509432946646798, 7079450852067684930, 2873528879681144878, 5558725876601302241, 7122421005083450743, 9720837126712108722, 7042772586139178077, 9659765980907602557, 617947098950154900, 3427189771032661006, 17611098782518188137, 11842008121454083047, 12090422274619625801, 13027146231092701682, 7150380630802500542, 76383145629883242, 18080576578711996702, 16356958352804286010, 14114746357020113352, 7921345840959732225, 17062772333595287544, 1260272922934697060, 5106096742451363382, 16075734719455612351, 10285794434717851630, 13399089060204000538, 8989588679388842206, 17101036744433399257, 1870648155382229838, 4497126874072672417, 3938922443378817162, 2927351814943280270, 3808821898427060335, 13872502583271862467, 5070605930349356387, 11972956721874186286, 8204290494894192656, 7230281892289845417, 18275285539786715214, 2404658561333529903, 16200640225189980110, 1810535742598390686, 749579642403451083, 9381688544530888032, 2833403535692394632, 4291075055163480910, 13897310516919681668, 18124882455222455711, 4399037140000442103, 16790244923650890342, 11248815251785723945, 8950265283055727281, 14064316622141232227, 3486167182041002958, 3686193929606109177, 2507146866769039965, 17198954785903697242, 17081810208716216052, 3983765481446896246, 7931770154753963032, 11893182668119123647, 12793958946298266810, 8401299987260453643, 2010613517693662606, 1665411773551417479, 11537634155796626245, 17933021902018037060, 15450537488547765217, 5339631745645945879, 12343722746092515604, 1624170935175840137, 6367934948056314691, 9093462352226564605, 5970002736843134976, 16103184750985353063, 15981753300871893582, 7557999814210827344, 5975213922167227474, 9964613871441776749, 9541798040899160189, 9138840133348875391, 1714696712793392765, 4519285853735943465, 442643889793963538];

        for i in 0..64 {
            match self.board[i] {
                Some(Piece::Pawn(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i]);
                }
                Some(Piece::Pawn(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + 64]);
                }
                Some(Piece::Knight(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 2)]);
                }
                Some(Piece::Knight(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 3)]);
                }
                Some(Piece::Bishop(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 4)]);
                }
                Some(Piece::Bishop(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 5)]);
                }
                Some(Piece::Rook(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 6)]);
                }
                Some(Piece::Rook(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 7)]);
                }
                Some(Piece::Queen(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 8)]);
                }
                Some(Piece::Queen(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 9)]);
                }
                Some(Piece::King(Color::White)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 10)]);
                }
                Some(Piece::King(Color::Black)) => {
                    hash = hash.wrapping_add(psrn_table[i + (64 * 11)]);
                }
                None => {}
            }
        }

        if self.move_next == Color::Black {
            hash = hash.wrapping_add(768);
        }
        if self.castling_rights.white.kingside {
            hash = hash.wrapping_add(769);
        }
        if self.castling_rights.white.queenside {
            hash = hash.wrapping_add(770);
        }
        if self.castling_rights.black.kingside {
            hash = hash.wrapping_add(771);
        }
        if self.castling_rights.black.queenside {
            hash = hash.wrapping_add(772);
        }

        if self.en_passant_target != None {
            hash = hash.wrapping_add(773 + (self.en_passant_target.unwrap() % 8) as u64);
        }

        return hash;
    }

    fn to_fen(&self) -> String {
        let mut fen = String::new();

        let mut index: u8 = 64;
        let mut blank_count: u8;

        for i in 0..8 {
            index -= 8;
            blank_count = 0;

            if self.board[index as usize] == None {
                blank_count += 1;
            } else {
                if blank_count != 0 {
                    fen += &format!("{}", blank_count);
                    blank_count = 0;
                }
                fen += &format!("{}", piece_to_char(self.board[index as usize], false));
            }

            for _ in 0..7 {
                index += 1;
                if self.board[index as usize] == None {
                    blank_count += 1;
                } else {
                    if blank_count != 0 {
                        fen += &format!("{}", blank_count);
                        blank_count = 0;
                    }
                    fen += &format!("{}", piece_to_char(self.board[index as usize], false));
                }
            }

            if blank_count != 0 {
                fen += &format!("{}", blank_count);
            }

            if i != 7 {
                fen += "/";
            }
            index -= 7;
        }

        if self.move_next == Color::Black {
            fen += " b";
        } else {
            fen += " w";
        }

        fen += " ";

        if self.castling_rights
            == (CastlingRights {
                black: ColorCastlingRights {
                    kingside: false,
                    queenside: false,
                },
                white: ColorCastlingRights {
                    kingside: false,
                    queenside: false,
                },
            })
        {
            fen += "-";
        } else {
            if self.castling_rights.white.kingside {
                fen += "K";
            }
            if self.castling_rights.white.queenside {
                fen += "Q";
            }
            if self.castling_rights.black.kingside {
                fen += "k";
            }
            if self.castling_rights.black.queenside {
                fen += "q";
            }
        }

        fen += " ";

        if self.en_passant_target == None {
            fen += "-";
        } else {
            fen += &format!("{}", int_to_coord(self.en_passant_target.unwrap()));
        }

        fen += &format!(" {} {}", self.halfmove_clock, self.fullmove_number);

        return fen;
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut to_print = String::new();
        to_print += "\n";

        let mut index: usize = 72;

        let mut column_num: u8 = 8;
        let horiz_space = "   ";

        for _i in 0..8 {
            index -= 16;
            to_print += &format!("{} {}", column_num, horiz_space);
            column_num -= 1;
            for _j in 0..8 {
                let piece_char = piece_to_char(self.board[index], false);

                to_print += &format!("{}{}", piece_char, horiz_space);
                index += 1;
            }
            to_print += "\n";
        }
        to_print += &format!(
            "\n  {}A{}B{}C{}D{}E{}F{}G{}H\n",
            horiz_space,
            horiz_space,
            horiz_space,
            horiz_space,
            horiz_space,
            horiz_space,
            horiz_space,
            horiz_space
        );
        return write!(f, "{}", to_print);
    }
}

struct EngineOptions {
    multi_pv: u8,
    debug_indexes: bool,
    debug_sets_display: bool,
    debug_use_symbols: bool,
}

struct SharedFlags {
    uci_enabled: bool,
    debug_enabled: bool,
    registration_name: String,
    registration_code: String,
    is_ready: bool,
    should_stop: bool,
    should_quit: bool,
    can_quit: bool,
    ponder_hit: bool,
    position: Position,
    options: EngineOptions,
    eval_map: Vec<HashMap<u64, (i32, Vec<HalfMove>)>>,
    repetition_map: HashMap<u64, u8>,
}

fn main() {
    let shared_flags = Arc::new(Mutex::new(SharedFlags {
        uci_enabled: false,
        debug_enabled: false,
        registration_name: String::from("EndGame2"),
        registration_code: String::from("6399"),
        is_ready: true,
        should_stop: false,
        should_quit: false,
        can_quit: false,
        ponder_hit: false,
        position: Position {
            board: [None; 64],
            piece_set: PieceSet {
                all: HashSet::new(),
                white: HashSet::new(),
                black: HashSet::new(),
                white_king: 5,
                black_king: 60,
            },
            move_next: Color::White,
            castling_rights: CastlingRights {
                black: ColorCastlingRights {
                    kingside: true,
                    queenside: true,
                },
                white: ColorCastlingRights {
                    kingside: true,
                    queenside: true,
                },
            },
            en_passant_target: None,
            halfmove_clock: 0,
            fullmove_number: 0,
        },
        // settings
        options: EngineOptions {
            multi_pv: 1,
            debug_indexes: false,
            debug_sets_display: false,
            debug_use_symbols: false,
        },
        eval_map: vec![HashMap::new()],
        repetition_map: HashMap::new(),
    }));

    let shared_flags_clone = Arc::clone(&shared_flags);
    // Create a separate thread to read CLI input to allow interrupts
    thread::spawn(move || {
        handle_cli_input(shared_flags_clone);
    });

    // start main program
    shared_flags.lock().unwrap().uci_enabled = true;
    handle_command("position startpos".to_string(), &shared_flags);

    let shared_flags_clone = Arc::clone(&shared_flags);
    while !shared_flags_clone.lock().unwrap().can_quit {
        thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn handle_cli_input(shared_flags: Arc<Mutex<SharedFlags>>) {
    for line in io::stdin().lock().lines() {
        if let Ok(input) = line {
            handle_command(input, &shared_flags);
        }
    }
}

fn print_handle_command(input: String, shared_flags: &Arc<Mutex<SharedFlags>>) {
    println!("> {}", input);
    handle_command(input, shared_flags);
}

fn handle_command(input: String, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let shared_flags_clone = Arc::clone(shared_flags);
    let input_clone = input.clone();

    thread::spawn(move || {
        let command_owned: Vec<String> = input_clone
            .trim()
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if let Some(word) = command_owned.get(0).map(String::as_str) {
            if !shared_flags_clone.lock().unwrap().uci_enabled {
                if word == "uci" {
                    uci_command(&shared_flags_clone);
                } else {
                    println!("Please enable UCI mode first!")
                }
            } else {
                let command_str = command_owned[1..].join(" ");
                let mut command = command_str.split_whitespace();

                parse_command(&shared_flags_clone, &mut command, word);
            }
        }
    });
}

fn parse_command(
    shared_flags: &Arc<Mutex<SharedFlags>>,
    mut command: &mut SplitWhitespace,
    word: &str,
) {
    match word {
        "uci" => uci_command(shared_flags),
        "debug" => debug_command(&mut command, shared_flags),
        "isready" => isready_command(shared_flags),
        "setoption" => setoption_command(&mut command, shared_flags),
        "register" => register_command(&mut command, shared_flags),
        "ucinewgame" => {
            // clear zobrist
            shared_flags.lock().unwrap().eval_map = vec![HashMap::new()];
        }
        "position" => position_command(&mut command, shared_flags),
        "go" => go_command(&mut command, shared_flags),
        "stop" => stop_command(shared_flags),
        "ponderhit" => ponderhit_command(shared_flags),
        "quit" => quit_command(shared_flags),
        // todo: remove following commands, or only enable in debug mode
        "ref" => print_index_reference(),
        "print" => display_debug(shared_flags),
        "moves" => handle_move_tokens(&mut command, shared_flags),
        "fen" => println!("{}", shared_flags.lock().unwrap().position.to_fen()),
        _ => println!("Error - Unknown command!"),
    }
}

fn ponderhit_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().ponder_hit = true
}

fn stop_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().should_stop = true
}

fn uci_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().uci_enabled = true;

    id_send(shared_flags);

    option_send();

    println!("uciok");
}

fn id_send(shared_flags: &Arc<Mutex<SharedFlags>>) {
    println!("id name {}", shared_flags.lock().unwrap().registration_name);
    println!("id author Koala");
}

fn option_send() {
    println!("option name DebugIndexes type check default true");
    println!("option name DebugSetsDisplay type check default false");
    println!("option name DebugUseSymbols type check default false");
}

fn position_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let token1 = command.next();

    match token1 {
        Some("startpos") => {
            set_board_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR", shared_flags);
            let token2 = command.next();
            if token2 == None {
                return;
            } else if token2.unwrap() != "moves" {
                println!("Error - expected moves token, got {}!", token2.unwrap());
                return;
            }
        }
        Some("fen") => {
            let fen = command.next().unwrap();
            set_board_from_fen(fen, shared_flags);
            set_flags_from_fen(command, shared_flags)
        }
        _ => println!("Position command improperly formatted!"),
    }

    shared_flags.lock().unwrap().repetition_map = HashMap::new();
    let hash = shared_flags.lock().unwrap().position.gen_hash();
    shared_flags.lock().unwrap().repetition_map.insert(hash, 1);

    handle_move_tokens(command, shared_flags);
}

fn handle_move_tokens(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let mut move_token = command.next();

    while move_token != None {
        let parsed_move = string_to_halfmove(shared_flags, move_token.unwrap());

        if parsed_move == None {
            println!("Error - unparsable move - {}", move_token.unwrap());
            break;
        } else {
            let mut position = shared_flags.lock().unwrap().position.clone();
            execute_halfmove(&mut position, parsed_move.unwrap());
            shared_flags.lock().unwrap().position = position;

            let hash = shared_flags.lock().unwrap().position.gen_hash();
            *shared_flags
                .lock()
                .unwrap()
                .repetition_map
                .entry(hash)
                .or_insert(0) += 1;

            display_debug(shared_flags);
        }

        move_token = command.next();
    }
}

fn execute_halfmove(position: &mut Position, to_exec: HalfMove) {
    // no legality checks, assumes that to_exec is legal

    if to_exec.from == 0 && to_exec.to == 0 {
        return;
    }

    position.halfmove_clock += 1;

    let piece: Piece;

    let color = position.board[to_exec.from as usize].unwrap().get_color();

    match to_exec.flag {
        Some(HalfmoveFlag::KnightPromotion) => {
            piece = Piece::Knight(color);
        }
        Some(HalfmoveFlag::BishopPromotion) => {
            piece = Piece::Bishop(color);
        }
        Some(HalfmoveFlag::RookPromotion) => {
            piece = Piece::Rook(color);
        }
        Some(HalfmoveFlag::QueenPromotion) => {
            piece = Piece::Queen(color);
        }
        _ => {
            piece = position.board[to_exec.from as usize].unwrap();
        }
    }

    if to_exec.flag != Some(HalfmoveFlag::Castle) {
        if position.board[to_exec.to as usize] != None
            || position.board[to_exec.from as usize] == Some(Piece::Pawn(position.move_next))
        {
            position.halfmove_clock = 0;
        }

        position.board[to_exec.to as usize] = Some(piece);
        position
            .piece_set
            .add_index_or_color_swap(to_exec.to, color);

        if piece == Piece::King(Color::White) {
            position.castling_rights.white.kingside = false;
            position.castling_rights.white.queenside = false;
            position.piece_set.white_king = to_exec.to;
        } else if piece == Piece::King(Color::Black) {
            position.castling_rights.black.kingside = false;
            position.castling_rights.black.queenside = false;
            position.piece_set.black_king = to_exec.to;
        } else if piece == Piece::Rook(Color::White) {
            if to_exec.from == 0 {
                position.castling_rights.white.queenside = false;
            } else if to_exec.from == 7 {
                position.castling_rights.white.kingside = false;
            }
        } else if piece == Piece::Rook(Color::Black) {
            if to_exec.from == 56 {
                position.castling_rights.black.queenside = false;
            } else if to_exec.from == 63 {
                position.castling_rights.black.kingside = false;
            }
        }
    } else {
        position.board[to_exec.to as usize] = None;
        position.piece_set.remove_index(to_exec.to, color);
        if color == Color::White {
            if to_exec.to == 0 {
                position.board[2] = Some(Piece::King(color));
                position.piece_set.add_index(2, color);
                position.piece_set.white_king = 2;

                position.board[3] = Some(Piece::Rook(color));
                position.piece_set.add_index(3, color);
            } else {
                // to_exec.to = 7
                position.board[6] = Some(Piece::King(color));
                position.piece_set.add_index(6, color);
                position.piece_set.white_king = 6;

                position.board[5] = Some(Piece::Rook(color));
                position.piece_set.add_index(5, color);
            }

            position.castling_rights.white.kingside = false;
            position.castling_rights.white.queenside = false;
        } else {
            if to_exec.to == 56 {
                position.board[58] = Some(Piece::King(color));
                position.piece_set.add_index(58, color);
                position.piece_set.black_king = 58;

                position.board[59] = Some(Piece::Rook(color));
                position.piece_set.add_index(59, color);
            } else {
                // to_exec.to = 63
                position.board[62] = Some(Piece::King(color));
                position.piece_set.add_index(62, color);
                position.piece_set.black_king = 62;

                position.board[61] = Some(Piece::Rook(color));
                position.piece_set.add_index(61, color);
            }

            position.castling_rights.black.kingside = false;
            position.castling_rights.black.queenside = false;
        }
    }

    position.board[to_exec.from as usize] = None;
    position.piece_set.remove_index(to_exec.from, color);

    if to_exec.flag == Some(HalfmoveFlag::EnPassant) {
        let mut target = position.en_passant_target.unwrap();

        if (target / 8) == 5 {
            target -= 8;
        } else {
            target += 8;
        }

        position.board[target as usize] = None;
        position.piece_set.remove_index(target, color.opposite());
    }

    if to_exec.flag == Some(HalfmoveFlag::DoublePawnMove) {
        let middle_space: u8;

        if to_exec.from > to_exec.to {
            middle_space = to_exec.from - 8;
        } else {
            middle_space = to_exec.from + 8;
        }

        position.en_passant_target = Some(middle_space);
    } else {
        position.en_passant_target = None;
    }

    if position.move_next == Color::Black {
        position.fullmove_number += 1;
        position.move_next = Color::White;
    } else {
        position.move_next = Color::Black;
    }

    let king_pos: u8;
    let kingside: bool;
    let queenside: bool;

    if position.move_next == Color::White {
        king_pos = position.piece_set.white_king;
        kingside = position.castling_rights.white.kingside;
        queenside = position.castling_rights.white.queenside;
    } else {
        king_pos = position.piece_set.black_king;
        kingside = position.castling_rights.black.kingside;
        queenside = position.castling_rights.black.queenside;
    }

    if kingside && queenside && is_piece_attacked(king_pos, position.move_next, position) {
        if position.move_next == Color::White {
            position.castling_rights.white.kingside = false;
            position.castling_rights.white.queenside = false;
        } else {
            position.castling_rights.black.kingside = false;
            position.castling_rights.black.queenside = false;
        }
    }
}

fn string_to_halfmove(
    shared_flags: &Arc<Mutex<SharedFlags>>,
    move_string: &str,
) -> Option<HalfMove> {
    let coord1_str: String = move_string.chars().take(2).collect();
    let coord1 = coord_to_int(&coord1_str);

    let coord2_str: String = move_string.chars().skip(2).take(2).collect();
    let mut coord2 = coord_to_int(&coord2_str);

    let position = &shared_flags.lock().unwrap().position;

    let board = &position.board;

    let mut flag = None;
    if board[coord1 as usize] == Some(Piece::Pawn(position.move_next))
        && position.en_passant_target == Some(coord2)
    {
        flag = Some(HalfmoveFlag::EnPassant);
    } else if board[coord1 as usize] == Some(Piece::King(position.move_next)) {
        if position.move_next == Color::White {
            if coord1 == 4 {
                if (coord2 == 7 || coord2 == 6) && position.castling_rights.white.kingside {
                    coord2 = 7;
                    flag = Some(HalfmoveFlag::Castle);
                }
                if (coord2 == 0 || coord2 == 2) && position.castling_rights.white.queenside {
                    coord2 = 0;
                    flag = Some(HalfmoveFlag::Castle);
                }
            }
        } else {
            if coord1 == 60 {
                if (coord2 == 63 || coord2 == 62) && position.castling_rights.black.kingside {
                    coord2 = 63;
                    flag = Some(HalfmoveFlag::Castle);
                }
                if (coord2 == 56 || coord2 == 58) && position.castling_rights.black.queenside {
                    coord2 = 56;
                    flag = Some(HalfmoveFlag::Castle);
                }
            }
        }
    }

    if move_string.len() > 4 {
        flag = match move_string.chars().nth(4) {
            Some('n') => Some(HalfmoveFlag::KnightPromotion),
            Some('b') => Some(HalfmoveFlag::BishopPromotion),
            Some('r') => Some(HalfmoveFlag::RookPromotion),
            Some('q') => Some(HalfmoveFlag::QueenPromotion),
            _ => None,
        };
    }

    return Some(HalfMove {
        from: coord1,
        to: coord2,
        flag,
        is_capture: false,
    });
}

fn set_flags_from_fen(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let move_next_token = command.next();

    match move_next_token {
        Some("w") => {
            shared_flags.lock().unwrap().position.move_next = Color::White;
        }
        Some("b") => {
            shared_flags.lock().unwrap().position.move_next = Color::Black;
        }
        Some("moves") => return,
        _ => println!(
            "Error - expected b or w, received {}",
            move_next_token.unwrap()
        ),
    }

    if let Some(castling_rights_token) = command.next() {
        parse_castling_rights(shared_flags, castling_rights_token);
    }

    if let Some(en_passant_token) = command.next() {
        if en_passant_token == "-" {
            shared_flags.lock().unwrap().position.en_passant_target = None;
        } else {
            let en_passant_target = Some(coord_to_int(en_passant_token));
            shared_flags.lock().unwrap().position.en_passant_target = en_passant_target;
        }
    }
    let next_token = command.next();

    if next_token == None {
        return;
    } else if next_token.unwrap() == "moves" {
        return;
    } else {
        let halfmove_clock_token = next_token.unwrap();
        match halfmove_clock_token.parse::<u16>() {
            Ok(value) => {
                if value > 100 {
                    println!("Error - invalid halfmove clock!");
                }

                shared_flags.lock().unwrap().position.halfmove_clock = value;
            }
            Err(_e) => {
                println!("Error parsing halfmove clock: {}", halfmove_clock_token);
            }
        }
    }

    if let Some(fullmove_number_token) = command.next() {
        match fullmove_number_token.parse::<u16>() {
            Ok(value) => {
                shared_flags.lock().unwrap().position.fullmove_number = value;
            }
            Err(_e) => {
                println!("Error parsing fullmove number: {}", fullmove_number_token);
            }
        }
    }

    let next_token = command.next();

    if next_token == None {
        return;
    } else if next_token.unwrap() == "moves" {
        return;
    }

    println!("Expected token 'moves', found: {}", next_token.unwrap());
}

fn coord_to_int(coord: &str) -> u8 {
    let file = coord.chars().nth(0).unwrap() as u8 - 'a' as u8;

    let rank = coord.chars().nth(1).unwrap().to_digit(10).unwrap() as u8 - 1;

    return rank * 8 + file;
}

fn int_to_coord(num: u8) -> String {
    let file = (num % 8) as u8 + 'a' as u8;

    let rank = (num / 8 + 1).to_string();

    let coord = (file as char).to_string() + &rank;

    return coord;
}

fn parse_castling_rights(shared_flags: &Arc<Mutex<SharedFlags>>, castling_rights_token: &str) {
    for char in castling_rights_token.chars() {
        match char {
            'Q' => {
                shared_flags
                    .lock()
                    .unwrap()
                    .position
                    .castling_rights
                    .white
                    .queenside = true
            }
            'K' => {
                shared_flags
                    .lock()
                    .unwrap()
                    .position
                    .castling_rights
                    .white
                    .kingside = true
            }
            'q' => {
                shared_flags
                    .lock()
                    .unwrap()
                    .position
                    .castling_rights
                    .black
                    .queenside = true
            }
            'k' => {
                shared_flags
                    .lock()
                    .unwrap()
                    .position
                    .castling_rights
                    .black
                    .kingside = true
            }
            '-' => {}
            _ => println!(
                "Error - invalid castling rights, received {}",
                castling_rights_token
            ),
        }
    }
}

fn set_board_from_fen(fen: &str, shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().position = Position {
        board: [None; 64],
        piece_set: PieceSet {
            all: HashSet::new(),
            white: HashSet::new(),
            black: HashSet::new(),
            white_king: 5,
            black_king: 60,
        },
        move_next: Color::White,
        castling_rights: CastlingRights {
            black: ColorCastlingRights {
                kingside: true,
                queenside: true,
            },
            white: ColorCastlingRights {
                kingside: true,
                queenside: true,
            },
        },
        en_passant_target: None,
        halfmove_clock: 0,
        fullmove_number: 0,
    };

    let mut index: usize = 56;

    for char in fen.chars() {
        if char == '/' {
            index -= 16;
        } else {
            handle_fen_char(shared_flags, &mut index, char);
            index += 1;
        }
    }

    display_debug(shared_flags);
    shared_flags.lock().unwrap().position.gen_hash();
}

fn display_debug(shared_flags: &Arc<Mutex<SharedFlags>>) {
    if shared_flags.lock().unwrap().debug_enabled {
        println!();

        if shared_flags.lock().unwrap().options.debug_indexes {
            print_board_with_indexes(shared_flags);
        } else {
            print_board(shared_flags);
        }

        if shared_flags.lock().unwrap().options.debug_sets_display {
            println!("{:?}", shared_flags.lock().unwrap().position.piece_set);
        }
        println!();
        println!();
    }
}

fn handle_fen_char(shared_flags: &Arc<Mutex<SharedFlags>>, mut index: &mut usize, char: char) {
    match char {
        'P' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Pawn(Color::White))
        }
        'N' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Knight(Color::White))
        }
        'B' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Bishop(Color::White))
        }
        'R' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Rook(Color::White))
        }
        'Q' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Queen(Color::White))
        }
        'K' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::King(Color::White));
            shared_flags.lock().unwrap().position.piece_set.white_king = *index as u8;
        }
        'p' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Pawn(Color::Black))
        }
        'n' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Knight(Color::Black))
        }
        'b' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Bishop(Color::Black))
        }
        'r' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Rook(Color::Black))
        }
        'q' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Queen(Color::Black))
        }
        'k' => {
            shared_flags.lock().unwrap().position.board[*index] = Some(Piece::King(Color::Black));
            shared_flags.lock().unwrap().position.piece_set.black_king = *index as u8;
        }
        _ => handle_fen_digit(&mut index, char),
    }

    match char {
        'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => {
            shared_flags
                .lock()
                .unwrap()
                .position
                .piece_set
                .add_index(*index as u8, Color::White);
        }
        'p' | 'n' | 'b' | 'r' | 'q' | 'k' => {
            shared_flags
                .lock()
                .unwrap()
                .position
                .piece_set
                .add_index(*index as u8, Color::Black);
        }
        _ => {}
    }
}

fn piece_to_char(piece: Option<Piece>, use_symbols: bool) -> char {
    if use_symbols {
        match piece {
            Some(Piece::Pawn(Color::White)) => return '♙',
            Some(Piece::Knight(Color::White)) => return '♘',
            Some(Piece::Bishop(Color::White)) => return '♗',
            Some(Piece::Rook(Color::White)) => return '♖',
            Some(Piece::Queen(Color::White)) => return '♕',
            Some(Piece::King(Color::White)) => return '♔',
            Some(Piece::Pawn(Color::Black)) => return '♟',
            Some(Piece::Knight(Color::Black)) => return '♞',
            Some(Piece::Bishop(Color::Black)) => return '♝',
            Some(Piece::Rook(Color::Black)) => return '♜',
            Some(Piece::Queen(Color::Black)) => return '♛',
            Some(Piece::King(Color::Black)) => return '♚',
            _ => {}
        }
        return '⚊';
    } else {
        match piece {
            Some(Piece::Pawn(Color::White)) => return 'P',
            Some(Piece::Knight(Color::White)) => return 'N',
            Some(Piece::Bishop(Color::White)) => return 'B',
            Some(Piece::Rook(Color::White)) => return 'R',
            Some(Piece::Queen(Color::White)) => return 'Q',
            Some(Piece::King(Color::White)) => return 'K',
            Some(Piece::Pawn(Color::Black)) => return 'p',
            Some(Piece::Knight(Color::Black)) => return 'n',
            Some(Piece::Bishop(Color::Black)) => return 'b',
            Some(Piece::Rook(Color::Black)) => return 'r',
            Some(Piece::Queen(Color::Black)) => return 'q',
            Some(Piece::King(Color::Black)) => return 'k',
            _ => {}
        }
        return '-';
    }
}

fn print_board(shared_flags: &Arc<Mutex<SharedFlags>>) {
    let mut index: usize = 72;

    let mut column_num: u8 = 8;
    let horiz_space = "   ";

    for _i in 0..8 {
        index -= 16;
        print!("{} {}", column_num, horiz_space);
        column_num -= 1;
        for _j in 0..8 {
            let use_symbols = shared_flags.lock().unwrap().options.debug_use_symbols;
            let piece_char = piece_to_char(
                shared_flags.lock().unwrap().position.board[index],
                use_symbols,
            );

            print!("{}{}", piece_char, horiz_space);
            index += 1;
        }
        println!();
    }
    println!();
    println!(
        "  {}A{}B{}C{}D{}E{}F{}G{}H",
        horiz_space,
        horiz_space,
        horiz_space,
        horiz_space,
        horiz_space,
        horiz_space,
        horiz_space,
        horiz_space
    );
}

fn print_index_reference() {
    let mut index: usize = 72;

    for _i in 0..8 {
        index -= 16;
        for _j in 0..8 {
            if index < 10 {
                print!("0{}  ", index);
            } else {
                print!("{}  ", index);
            }
            index += 1;
        }
        println!();
    }
}

fn print_board_with_indexes(shared_flags: &Arc<Mutex<SharedFlags>>) {
    let mut index: usize = 72;

    for _i in 0..8 {
        index -= 16;
        for _j in 0..8 {
            let use_symbols = shared_flags.lock().unwrap().options.debug_use_symbols;
            let piece_char = piece_to_char(
                shared_flags.lock().unwrap().position.board[index],
                use_symbols,
            );

            if piece_char == '-' || piece_char == '⚊' {
                print!("  {}   ", piece_char);
            } else {
                if index < 10 {
                    print!("0{}-{}  ", index, piece_char);
                } else {
                    print!("{}-{}  ", index, piece_char);
                }
            }

            index += 1;
        }
        println!();
        println!();
    }
}

fn handle_fen_digit(index: &mut usize, char: char) {
    if char.is_digit(9) {
        if let Some(digit) = char.to_digit(9) {
            *index += digit as usize - 1;
        }
    }
}

fn go_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let position = shared_flags.lock().unwrap().position.clone();

    let token1 = command.next();

    match token1 {
        Some("perft") => {
            if let Some(token2) = command.next() {
                match token2.parse::<u8>() {
                    Ok(depth) => perft_command(position, depth, shared_flags),
                    Err(_) => println!("Error: Depth must be a valid number!"),
                }
            } else {
                println!("Error: Depth not specified for perft command!");
            }
        }
        Some("infinite") => {
            go_search(position, None, None, None, shared_flags);
        }

        Some("nodes") => {
            go_search(
                position,
                Some(command.next().unwrap().parse::<usize>().unwrap()),
                None,
                None,
                shared_flags,
            );
        }

        Some("movetime") => {
            let parsed = command.next().unwrap().parse::<u64>().unwrap();
            let term_time = Some(Instant::now() + Duration::from_millis(parsed));

            go_search(position, None, None, term_time, shared_flags);
        }
        Some("depth") => {
            go_search(
                position,
                None,
                Some(command.next().unwrap().parse::<usize>().unwrap()),
                None,
                shared_flags,
            );
        }
        None => {
            go_search(position, Some(500000), None, None, shared_flags);
        }
        _ => println!("Go command improperly formatted!"),
    }

    if shared_flags.lock().unwrap().should_quit == true {
        shared_flags.lock().unwrap().can_quit = true;
    }
    shared_flags.lock().unwrap().should_stop = false;
}

fn go_search(
    position: Position,
    node_stop: Option<usize>,
    mut depth_stop: Option<usize>,
    time_stop: Option<Instant>,
    shared_flags: &Arc<Mutex<SharedFlags>>,
) {
    let mut tree = PositionTree::from_pos(position);
    let mut moves;
    let mut score;
    let mut depth = 0;
    let start_time;
    let mut nps_start;
    let mut prev_score = 0;
    let mut prev_moves = vec![];

    start_time = Instant::now();

    if let Some(ref mut depth) = depth_stop {
        if *depth <= 1 {
            *depth += 1;
        }
    }

    loop {
        if shared_flags.lock().unwrap().eval_map.len() <= depth + 1 {
            let zobrist = &mut shared_flags.lock().unwrap().eval_map;
            zobrist.push(HashMap::new())
        }

        nps_start = Instant::now();

        let is_maximizing = tree.position.move_next == Color::White;
        let start_pos = tree.position.clone();

        (score, moves) = minimax(
            &mut tree,
            start_pos,
            0,
            0,
            is_maximizing,
            i32::MIN + 1,
            i32::MAX,
            depth,
            shared_flags,
            time_stop,
            node_stop,
        );

        depth += 1;
        tree.depth += 1;

        if time_stop.is_some() && time_stop.unwrap() <= Instant::now() {
            depth -= 1;
            tree.depth -= 1;
            if score.abs() == i32::MAX {
                score = prev_score;
                moves = prev_moves.clone();
                println!("Debug: Reverting to previous depth!");
            }
            break;
        } else {
            prev_score = score;
            prev_moves = moves.clone();
        }

        if score.abs() >= 30000
            || (node_stop.is_some() && node_stop.unwrap() <= tree.leaf_size)
            || (depth_stop.is_some() && depth_stop.unwrap() <= depth)
            || shared_flags.lock().unwrap().should_stop
        {
            break;
        } else if start_time.elapsed().as_millis() > 0 {
            println!(
                "info depth {} nodes {} nps {} score {} currmove {}",
                depth,
                tree.leaf_size,
                ((tree.leaf_size as f64 / nps_start.elapsed().as_nanos() as f64) * 1000000000.0)
                    as u32,
                score,
                moves[0].move_to_coords()
            );
        }
    }

    print!(
        "info depth {} nodes {} nps {} time {} ",
        depth,
        tree.leaf_size,
        ((tree.leaf_size as f64 / nps_start.elapsed().as_nanos() as f64) * 1000000000.0) as u32,
        start_time.elapsed().as_millis()
    );

    if depth <= 2 {
        depth = 3;
    }
    if score >= 30000 {
        print!("score mate {} ", (depth - 1) / 2);
    } else if score <= -30000 {
        print!("score mate -{} ", (depth - 1) / 2);
    } else {
        print!("score cp {} ", score);
        print_pv(&moves);
    }

    print!("bestmove {} ", moves[0].move_to_coords(),);

    if moves.len() > 1 && moves[1].move_to_coords() != "a1a1" {
        println!("ponder {}", moves[1].move_to_coords())
    } else {
        println!();
    }
}

fn print_pv(moves: &Vec<HalfMove>) {
    if moves[0].move_to_coords() == "a1a1" || moves.is_empty() {
        return;
    }

    print!("pv ");

    for i in 0..moves.len() - 1 {
        if moves[i].move_to_coords() == "a1a1" {
            break;
        }
        print!("{} ", moves[i].move_to_coords());
    }
    println!();
}

fn minimax(
    tree: &mut PositionTree,
    position: Position,
    node_depth: usize,
    node_index: usize,
    is_maximizing: bool,
    mut alpha: i32,
    mut beta: i32,
    depth: usize,
    shared_flags: &Arc<Mutex<SharedFlags>>,
    term_time: Option<Instant>,
    term_nodes: Option<usize>,
) -> (i32, Vec<HalfMove>) {
    if depth > 0 {
        match shared_flags.lock().unwrap().eval_map[depth - 1].get(&position.gen_hash()) {
            Some(hashed) => {
                // zobrist cache hit
                tree.nodes[node_depth][node_index].score = hashed.0;
                return hashed.clone();
            }
            None => {}
        }
    }

    if (term_time.is_some() && term_time.unwrap() < Instant::now())
        || shared_flags.lock().unwrap().should_stop
        || (term_nodes.is_some() && term_nodes.unwrap() < tree.leaf_size)
    {
        let eval = if is_maximizing {
            i32::MIN + 1
        } else {
            i32::MAX
        };
        tree.nodes[node_depth][node_index].score = eval;
        return (
            eval,
            vec![tree.nodes[node_depth][node_index].halfmove.clone()],
        );
    }

    if tree.nodes[node_depth][node_index].children.is_none() {
        tree.gen_children(node_depth, node_index);
    }

    let mut eval_exists = false;
    let mut to_search: Vec<(usize, i32)> = Vec::new();
    if tree.nodes[node_depth][node_index].children.is_some() {
        let children = tree.nodes[node_depth][node_index].children.unwrap().clone();
        for i in children.0..children.1 + 1 {
            if depth > 0
                || position.board[tree.nodes[node_depth + 1][i].halfmove.to as usize] != None
            {
                to_search.push((i, tree.nodes[node_depth + 1][i].score));
                if tree.nodes[node_depth + 1][i].score != 0 {
                    eval_exists = true
                }
            }
        }
    }

    if to_search.is_empty() {
        let eval = position_eval(&position, shared_flags);
        tree.nodes[node_depth][node_index].score = eval;
        return (
            eval,
            vec![tree.nodes[node_depth][node_index].halfmove.clone()],
        );
    }

    if eval_exists {
        if is_maximizing {
            to_search.sort_by(|a, b| a.1.cmp(&b.1));
        } else {
            to_search.sort_by(|a, b| b.1.cmp(&a.1));
        }
    }

    let mut best_score = if is_maximizing {
        i32::MIN + 1
    } else {
        i32::MAX
    };
    let mut best_path = Vec::new();
    for i in 0..to_search.len() {
        let mut new_pos = position.clone();

        let halfmove = tree.nodes[node_depth + 1][to_search[i].0].halfmove.clone();
        execute_halfmove(&mut new_pos, halfmove);

        // no more computations if found mate
        if is_maximizing && alpha >= 30000 {
            return (alpha, best_path);
        } else if !is_maximizing && beta <= -30000 {
            return (beta, best_path);
        }

        let (child_score, mut child_path) = minimax(
            tree,
            new_pos,
            node_depth + 1,
            to_search[i].0,
            !is_maximizing,
            alpha,
            beta,
            if depth > 0 { depth - 1 } else { 0 },
            shared_flags,
            term_time,
            term_nodes,
        );

        if is_maximizing {
            if child_score > best_score {
                best_score = child_score;
                child_path.insert(0, halfmove);
                best_path = child_path.clone();
            }
            alpha = alpha.max(best_score);
        } else {
            if child_score < best_score {
                best_score = child_score;
                child_path.insert(0, halfmove);
                best_path = child_path.clone();
            }
            beta = beta.min(best_score);
        }

        if beta <= alpha {
            break;
        }

        // note: no need to early return if finished loop anyways, so check for all but last iter
        if ((term_time.is_some() && term_time.unwrap() < Instant::now())
            || shared_flags.lock().unwrap().should_stop
            || (term_nodes.is_some() && term_nodes.unwrap() < tree.leaf_size))
            && i < to_search.len() - 1
        {
            // note: won't be sorted if early return.
            // also won't store in zobrist, which is intentional, as current is not fully searched
            tree.nodes[node_depth][node_index].score = best_score;
            return (best_score, best_path);
        }
    }

    if depth > 0 {
        let zobrist = &mut shared_flags.lock().unwrap().eval_map;
        zobrist[depth - 1].insert(position.gen_hash(), (best_score, best_path.clone()));
    }

    tree.nodes[node_depth][node_index].score = best_score;
    return (best_score, best_path);
}

fn position_eval(position: &Position, shared_flags: &Arc<Mutex<SharedFlags>>) -> i32 {
    let mut eval = 0;

    // 50-move rule
    if position.halfmove_clock >= 50 {
        return 0;
    }

    // threefold repetition
    let hash = position.gen_hash();
    if let Some(&count) = shared_flags.lock().unwrap().repetition_map.get(&hash) {
        if count >= 2 {
            return 0;
        }
    }

    for &i in position.piece_set.white.iter() {
        eval += get_piece_value(position.board[i as usize].unwrap(), i);
    }
    for &i in position.piece_set.black.iter() {
        eval -= get_piece_value(position.board[i as usize].unwrap(), i);
    }
    return eval;
}

fn get_piece_value(piece: Piece, index: u8) -> i32 {
    let mut value;
    let pawn_table = [
        0, 0, 0, 0, 0, 0, 0, 0, 30, 30, 30, 40, 40, 30, 30, 30, 20, 20, 20, 30, 30, 30, 20, 20, 10,
        10, 15, 25, 25, 15, 10, 10, 5, 5, 5, 20, 20, 5, 5, 5, 5, 0, 0, 5, 5, 0, 0, 5, 5, 5, 5, -10,
        -10, 5, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let knight_table = [
        -5, -5, -5, -5, -5, -5, -5, -5, -5, 0, 0, 10, 10, 0, 0, -5, -5, 5, 10, 10, 10, 10, 5, -5,
        -5, 5, 10, 15, 15, 10, 5, -5, -5, 5, 10, 15, 15, 10, 5, -5, -5, 5, 10, 10, 10, 10, 5, -5,
        -5, 0, 0, 5, 5, 0, 0, -5, -5, -10, -5, -5, -5, -5, -10, -5,
    ];
    let bishop_table = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0,
        10, 0, 5, 0, 10, 0, 0, 10, 0, 5, 0, 10, 0, 10, 10, 0, 10, 0, 0, 10, 0, 10, 10, 0, 10, 0, 0,
        0, -10, 0, 0, -10, 0, 0,
    ];
    let rook_table = [
        10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 10, 0, 0,
        0, 0, 0, 0, 10, 10, 5, 0, 0,
    ];
    let queen_table = [
        -20, -10, -10, -5, -5, -10, -10, -20, -10, 0, 0, 0, 0, 0, 0, -10, -10, 0, 5, 5, 5, 5, 0,
        -10, -5, 0, 5, 5, 5, 5, 0, -5, -5, 0, 5, 5, 5, 5, 0, -5, -10, 5, 5, 5, 5, 5, 0, -10, -10,
        0, 5, 0, 0, 0, 0, -10, -20, -10, -10, 0, 0, -10, -10, -20,
    ];
    let king_table = [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, -5, -5, -5, 0, 0, 0, 0, 10,
        -5, -5, -5, 10, 0,
    ];

    let pos = if piece.get_color() == Color::White {
        (63 - index) as usize
    } else {
        index as usize
    };

    value = piece.get_cp_val() as i32;

    match piece {
        Piece::Pawn(_) => {
            value += pawn_table[pos];
        }
        Piece::Bishop(_) => {
            value += bishop_table[pos];
        }
        Piece::Knight(_) => {
            value += knight_table[pos];
        }
        Piece::Rook(_) => {
            value += rook_table[pos];
        }
        Piece::Queen(_) => {
            value += queen_table[pos];
        }
        Piece::King(_) => {
            value += king_table[pos];
        }
    }

    return value;
}

fn perft_command(position: Position, depth: u8, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let timer = Instant::now();
    let mut tree = PositionTree::from_pos(position);

    let mut perft = 0;
    for _ in 0..(depth) {
        perft = tree.increase_depth();
    }

    if shared_flags.lock().unwrap().debug_enabled {
        tree.print_tree()
    }
    println!(
        "Nodes: {}\nTime elapsed: {} ms",
        perft,
        timer.elapsed().as_millis()
    );
}

fn gen_possible(position: &mut Position) -> Vec<HalfMove> {
    let moves: Vec<HalfMove>;

    moves = gen_pseudolegal_moves(position);

    return moves;
}

fn is_piece_attacked(index: u8, piece_color: Color, position: &Position) -> bool {
    let opp_color = piece_color.opposite();

    let mut dir_offset = -8;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset < 0 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Rook(opp_color)
                || (piece == Piece::King(opp_color) && offset == -8)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = 8;
    offset = dir_offset;

    loop {
        if index as i8 + offset > 63 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Rook(opp_color)
                || (piece == Piece::King(opp_color) && offset == 8)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = 1;
    offset = dir_offset;

    loop {
        if (index as i8 + offset) % 8 == 0 || index as i8 + offset > 63 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Rook(opp_color)
                || (piece == Piece::King(opp_color) && offset == 1)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = -1;
    offset = dir_offset;

    loop {
        if (index as i8 + offset) % 8 == 7 || index as i8 + offset < 0 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Rook(opp_color)
                || (piece == Piece::King(opp_color) && offset == -1)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = 9;
    offset = dir_offset;

    loop {
        if index as i8 + offset > 63 || (index as i8 + offset) % 8 == 0 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Bishop(opp_color)
                || (piece == Piece::King(opp_color) && offset == 9)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = 7;
    offset = dir_offset;

    loop {
        if index as i8 + offset > 63 || (index as i8 + offset) % 8 == 7 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Bishop(opp_color)
                || (piece == Piece::King(opp_color) && offset == 7)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = -9;
    offset = dir_offset;

    loop {
        if index as i8 + offset < 0 || (index as i8 + offset) % 8 == 7 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Bishop(opp_color)
                || (piece == Piece::King(opp_color) && offset == -9)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    dir_offset = -7;
    offset = dir_offset;

    loop {
        if index as i8 + offset < 0 || (index as i8 + offset) % 8 == 0 {
            break;
        }

        if let Some(piece) = position.board[(index as i8 + offset) as usize] {
            if piece == Piece::Queen(opp_color)
                || piece == Piece::Bishop(opp_color)
                || (piece == Piece::King(opp_color) && offset == -7)
            {
                return true;
            }

            break;
        }

        offset += dir_offset;
    }

    // knight checks
    // up 2
    if (index / 8) <= 5 {
        // right 1
        if (index % 8) <= 6 {
            if position.board[(index as i8 + 17) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }

        // left 1
        if (index % 8) >= 1 {
            if position.board[(index as i8 + 15) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }
    }

    // right 2
    if (index % 8) <= 5 {
        // up 1
        if (index / 8) <= 6 {
            if position.board[(index as i8 + 10) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }

        // down 1
        if (index / 8) >= 1 {
            if position.board[(index as i8 - 6) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }
    }

    // down 2
    if (index / 8) >= 2 {
        // right 1
        if (index % 8) <= 6 {
            if position.board[(index as i8 - 15) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }

        // left 1
        if (index % 8) >= 1 {
            if position.board[(index as i8 - 17) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }
    }

    // left 2
    if (index % 8) >= 2 {
        // up 1
        if (index / 8) <= 6 {
            if position.board[(index as i8 + 6) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }

        // down 1
        if (index / 8) >= 1 {
            if position.board[(index as i8 - 10) as usize] == Some(Piece::Knight(opp_color)) {
                return true;
            }
        }
    }

    // pawn checks (not counting en-passant)
    if opp_color == Color::White && index > 7 {
        if index % 8 > 0 {
            if position.board[(index as i8 - 9) as usize] == Some(Piece::Pawn(opp_color)) {
                return true;
            }
        }

        if index % 8 < 7 {
            if position.board[(index as i8 - 7) as usize] == Some(Piece::Pawn(opp_color)) {
                return true;
            }
        }
    }

    if opp_color == Color::Black && index < 56 {
        if index % 8 > 0 {
            if position.board[(index as i8 + 7) as usize] == Some(Piece::Pawn(opp_color)) {
                return true;
            }
        }

        if index % 8 < 7 {
            if position.board[(index as i8 + 9) as usize] == Some(Piece::Pawn(opp_color)) {
                return true;
            }
        }
    }

    // todo: implement en-passant check so fn can be generalized for universal use including pawns

    return false;
}

fn gen_pseudolegal_moves(position: &Position) -> Vec<HalfMove> {
    let color = position.move_next;

    let piece_set: HashSet<u8>;

    if color == Color::Black {
        piece_set = position.piece_set.black.clone();
    } else {
        piece_set = position.piece_set.white.clone();
    }

    let mut moves: Vec<HalfMove> = Vec::new();

    for i in piece_set {
        // gen pseudolegal moves for each piece at index i
        // add each move to moves vector
        let result = gen_piece_pseudolegal_moves(i, position);
        moves.extend(result);

        // likely no need to gen new threads here, will likely be suboptimal due to thread overhead.
        // if no need for threads, we can pass moves as an address instead and return nothing
        // todo: test thread implementation performance
        // Our tree will exponentially grow so fast itd be pointless to do it here.

        // just a thought, if we make the eval properly, do we even need to check for legality?
    }

    if color == Color::Black {
        if position.castling_rights.black.kingside {
            if position.board[63] == Some(Piece::Rook(Color::Black))
                && position.board[62] == None
                && position.board[61] == None
                && position.board[60] == Some(Piece::King(Color::Black))
                && !is_piece_attacked(61, Color::Black, position)
                && !is_piece_attacked(62, Color::Black, position)
            {
                moves.push(HalfMove {
                    from: 60,
                    to: 63,
                    flag: Some(HalfmoveFlag::Castle),
                    is_capture: false,
                });
            }
        }

        if position.castling_rights.black.queenside {
            if position.board[56] == Some(Piece::Rook(Color::Black))
                && position.board[57] == None
                && position.board[58] == None
                && position.board[59] == None
                && position.board[60] == Some(Piece::King(Color::Black))
                && !is_piece_attacked(59, Color::Black, position)
                && !is_piece_attacked(58, Color::Black, position)
            {
                moves.push(HalfMove {
                    from: 60,
                    to: 56,
                    flag: Some(HalfmoveFlag::Castle),
                    is_capture: false,
                });
            }
        }
    } else {
        if position.castling_rights.white.queenside {
            if position.board[0] == Some(Piece::Rook(Color::White))
                && position.board[1] == None
                && position.board[2] == None
                && position.board[3] == None
                && position.board[4] == Some(Piece::King(Color::White))
                && !is_piece_attacked(3, Color::White, position)
                && !is_piece_attacked(2, Color::White, position)
            {
                moves.push(HalfMove {
                    from: 4,
                    to: 0,
                    flag: Some(HalfmoveFlag::Castle),
                    is_capture: false,
                });
            }
        }

        if position.castling_rights.white.kingside {
            if position.board[7] == Some(Piece::Rook(Color::White))
                && position.board[6] == None
                && position.board[5] == None
                && position.board[4] == Some(Piece::King(Color::White))
                && !is_piece_attacked(5, Color::White, position)
                && !is_piece_attacked(6, Color::White, position)
            {
                moves.push(HalfMove {
                    from: 4,
                    to: 7,
                    flag: Some(HalfmoveFlag::Castle),
                    is_capture: false,
                });
            }
        }
    }

    return moves;
}

fn gen_piece_pseudolegal_moves(piece_index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves;

    match position.board[piece_index as usize] {
        Some(Piece::Pawn(Color::White)) => {
            moves = gen_white_pawn_moves(piece_index, position);
        }
        Some(Piece::Pawn(Color::Black)) => {
            moves = gen_black_pawn_moves(piece_index, position);
        }
        Some(Piece::Knight(_)) => {
            moves = gen_knight_moves(piece_index, position);
        }
        Some(Piece::Rook(_)) => {
            moves = gen_rook_moves(piece_index, position);
        }
        Some(Piece::Bishop(_)) => {
            moves = gen_bishop_moves(piece_index, position);
        }
        Some(Piece::Queen(_)) => {
            moves = gen_queen_moves(piece_index, position);
        }
        Some(Piece::King(_)) => {
            moves = gen_normal_king_moves(piece_index, position);
        }
        None => panic!("Error, index contained in piece_set has no piece on board!"),
    }

    for i in 0..moves.len() {
        if position.board[moves[i].to as usize] == None
            && moves[i].flag != Some(HalfmoveFlag::EnPassant)
        {
            moves[i].is_capture = true;
        }
    }

    return moves;
}

fn gen_normal_king_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    gen_halfmove_with_check(7, index, position, &mut moves);
    gen_halfmove_with_check(8, index, position, &mut moves);
    gen_halfmove_with_check(9, index, position, &mut moves);
    gen_halfmove_with_check(1, index, position, &mut moves);
    gen_halfmove_with_check(-7, index, position, &mut moves);
    gen_halfmove_with_check(-8, index, position, &mut moves);
    gen_halfmove_with_check(-9, index, position, &mut moves);
    gen_halfmove_with_check(-1, index, position, &mut moves);

    return moves;
}

fn gen_halfmove_with_check(offset: i8, index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    if index as i8 + offset > 63 || index as i8 + offset < 0 {
        return;
    }

    // rightward bound check
    if (offset % 8 == 1 || offset % 8 == -7) && index % 8 == 7 {
        return;
    }

    // leftward bound check
    if (offset % 8 == 7 || offset % 8 == -1) && index % 8 == 0 {
        return;
    }

    gen_halfmove(offset, index, position, moves);
}

fn gen_queen_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    gen_down_left(index, position, &mut moves);
    gen_down_right(index, position, &mut moves);
    gen_up_left(index, position, &mut moves);
    gen_up_right(index, position, &mut moves);
    gen_downwards(index, position, &mut moves);
    gen_right(index, position, &mut moves);
    gen_upwards(index, position, &mut moves);
    gen_left(index, position, &mut moves);

    return moves;
}

fn gen_knight_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    // total of 8 move combinations

    // first, check bounds for length 2, then check bounds for length 1

    // up 2
    if (index / 8) <= 5 {
        // right 1
        if (index % 8) <= 6 {
            gen_halfmove(17, index, position, &mut moves);
        }

        // left 1
        if (index % 8) >= 1 {
            gen_halfmove(15, index, position, &mut moves);
        }
    }

    // right 2
    if (index % 8) <= 5 {
        // up 1
        if (index / 8) <= 6 {
            gen_halfmove(10, index, position, &mut moves);
        }

        // down 1
        if (index / 8) >= 1 {
            gen_halfmove(-6, index, position, &mut moves);
        }
    }

    // down 2
    if (index / 8) >= 2 {
        // right 1
        if (index % 8) <= 6 {
            gen_halfmove(-15, index, position, &mut moves);
        }

        // left 1
        if (index % 8) >= 1 {
            gen_halfmove(-17, index, position, &mut moves);
        }
    }

    // left 2
    if (index % 8) >= 2 {
        // up 1
        if (index / 8) <= 6 {
            gen_halfmove(6, index, position, &mut moves);
        }

        // down 1
        if (index / 8) >= 1 {
            gen_halfmove(-10, index, position, &mut moves);
        }
    }

    return moves;
}

fn gen_upwards(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = 8;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset > 63 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_downwards(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = -8;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset < 0 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_right(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = 1;
    let mut offset: i8 = dir_offset;

    loop {
        if (index as i8 + offset) % 8 == 0 || index as i8 + offset > 63 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_left(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = -1;
    let mut offset: i8 = dir_offset;

    loop {
        if (index as i8 + offset) % 8 == 7 || index as i8 + offset < 0 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_up_right(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = 9;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset > 63 || (index as i8 + offset) % 8 == 0 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_up_left(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = 7;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset > 63 || (index as i8 + offset) % 8 == 7 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_down_right(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = -7;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset < 0 || (index as i8 + offset) % 8 == 0 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_down_left(index: u8, position: &Position, moves: &mut Vec<HalfMove>) {
    let dir_offset = -9;
    let mut offset: i8 = dir_offset;

    loop {
        if index as i8 + offset < 0 || (index as i8 + offset) % 8 == 7 {
            break;
        }

        if !gen_halfmove(offset, index, position, moves) {
            break;
        }

        offset += dir_offset;
    }
}

fn gen_bishop_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    gen_down_left(index, position, &mut moves);
    gen_down_right(index, position, &mut moves);
    gen_up_left(index, position, &mut moves);
    gen_up_right(index, position, &mut moves);

    return moves;
}

fn gen_rook_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    gen_downwards(index, position, &mut moves);
    gen_right(index, position, &mut moves);
    gen_upwards(index, position, &mut moves);
    gen_left(index, position, &mut moves);

    return moves;
}

fn gen_halfmove(offset: i8, index: u8, position: &Position, moves: &mut Vec<HalfMove>) -> bool {
    let mut to_return = true;

    if let Some(piece) = position.board[(index as i8 + offset) as usize] {
        if piece.get_color() == position.move_next {
            return false;
        }
        to_return = false;
    }

    moves.push(HalfMove {
        from: index,
        to: (index as i8 + offset) as u8,
        flag: None,
        is_capture: false,
    });

    return to_return;
}

fn gen_white_pawn_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    let board = position.board;
    let piece = board[index as usize].unwrap();
    let color = piece.get_color();

    // straight move
    if board[(index + 8) as usize] == None {
        // nothing in the way
        if (index / 8) != 6 {
            moves.push(HalfMove {
                from: index,
                to: (index + 8),
                flag: None,
                is_capture: false,
            });
            if (index / 8 == 1) && board[(index + 16) as usize] == None {
                moves.push(HalfMove {
                    from: index,
                    to: (index + 16),
                    flag: Some(HalfmoveFlag::DoublePawnMove),
                    is_capture: false,
                });
            }
        } else {
            // promotion
            moves.push(HalfMove {
                from: index,
                to: (index + 8),
                flag: Some(HalfmoveFlag::KnightPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index + 8),
                flag: Some(HalfmoveFlag::BishopPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index + 8),
                flag: Some(HalfmoveFlag::RookPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index + 8),
                flag: Some(HalfmoveFlag::QueenPromotion),
                is_capture: false,
            });
        }
    }

    // captures
    let should_promote: bool;
    if (index / 8) == 6 {
        should_promote = true;
    } else {
        should_promote = false;
    }
    if (index % 8) != 0 {
        // left capture
        if let Some(target) = board[(index + 7) as usize] {
            if target.get_color() != color {
                if should_promote {
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 7),
                        flag: Some(HalfmoveFlag::KnightPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 7),
                        flag: Some(HalfmoveFlag::BishopPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 7),
                        flag: Some(HalfmoveFlag::RookPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 7),
                        flag: Some(HalfmoveFlag::QueenPromotion),
                        is_capture: false,
                    });
                } else {
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 7),
                        flag: None,
                        is_capture: false,
                    });
                }
            }
        } else if let Some(target) = position.en_passant_target {
            // en passant
            if index + 7 == target {
                moves.push(HalfMove {
                    from: index,
                    to: (index + 7),
                    flag: Some(HalfmoveFlag::EnPassant),
                    is_capture: false,
                });
            }
        }
    }
    if (index % 8) != 7 {
        // right capture
        if let Some(target) = board[(index + 9) as usize] {
            if target.get_color() != color {
                if should_promote {
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 9),
                        flag: Some(HalfmoveFlag::KnightPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 9),
                        flag: Some(HalfmoveFlag::BishopPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 9),
                        flag: Some(HalfmoveFlag::RookPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 9),
                        flag: Some(HalfmoveFlag::QueenPromotion),
                        is_capture: false,
                    });
                } else {
                    moves.push(HalfMove {
                        from: index,
                        to: (index + 9),
                        flag: None,
                        is_capture: false,
                    });
                }
            }
        } else if let Some(target) = position.en_passant_target {
            // en passant
            if index + 9 == target {
                moves.push(HalfMove {
                    from: index,
                    to: (index + 9),
                    flag: Some(HalfmoveFlag::EnPassant),
                    is_capture: false,
                });
            }
        }
    }

    return moves;
}

fn gen_black_pawn_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    let board = position.board;
    let piece = board[index as usize].unwrap();
    let color = piece.get_color();

    // straight move
    if board[(index - 8) as usize] == None {
        // nothing in the way
        if (index / 8) != 1 {
            moves.push(HalfMove {
                from: index,
                to: (index - 8),
                flag: None,
                is_capture: false,
            });
            if (index / 8 == 6) && board[(index - 16) as usize] == None {
                moves.push(HalfMove {
                    from: index,
                    to: (index - 16),
                    flag: Some(HalfmoveFlag::DoublePawnMove),
                    is_capture: false,
                });
            }
        } else {
            // promotion
            moves.push(HalfMove {
                from: index,
                to: (index - 8),
                flag: Some(HalfmoveFlag::KnightPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index - 8),
                flag: Some(HalfmoveFlag::BishopPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index - 8),
                flag: Some(HalfmoveFlag::RookPromotion),
                is_capture: false,
            });
            moves.push(HalfMove {
                from: index,
                to: (index - 8),
                flag: Some(HalfmoveFlag::QueenPromotion),
                is_capture: false,
            });
        }
    }

    // captures (left/right orientation with white as bottom)
    let should_promote: bool;
    if (index / 8) == 1 {
        should_promote = true;
    } else {
        should_promote = false;
    }
    if (index % 8) != 0 {
        // left capture
        if let Some(target) = board[(index - 9) as usize] {
            if target.get_color() != color {
                if should_promote {
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 9),
                        flag: Some(HalfmoveFlag::KnightPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 9),
                        flag: Some(HalfmoveFlag::BishopPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 9),
                        flag: Some(HalfmoveFlag::RookPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 9),
                        flag: Some(HalfmoveFlag::QueenPromotion),
                        is_capture: false,
                    });
                } else {
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 9),
                        flag: None,
                        is_capture: false,
                    });
                }
            }
        } else if let Some(target) = position.en_passant_target {
            // en passant
            if index - 9 == target {
                moves.push(HalfMove {
                    from: index,
                    to: (index - 9),
                    flag: Some(HalfmoveFlag::EnPassant),
                    is_capture: false,
                });
            }
        }
    }
    if (index % 8) != 7 {
        // right capture
        if let Some(target) = board[(index - 7) as usize] {
            if target.get_color() != color {
                if should_promote {
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 7),
                        flag: Some(HalfmoveFlag::KnightPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 7),
                        flag: Some(HalfmoveFlag::BishopPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 7),
                        flag: Some(HalfmoveFlag::RookPromotion),
                        is_capture: false,
                    });
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 7),
                        flag: Some(HalfmoveFlag::QueenPromotion),
                        is_capture: false,
                    });
                } else {
                    moves.push(HalfMove {
                        from: index,
                        to: (index - 7),
                        flag: None,
                        is_capture: false,
                    });
                }
            }
        } else if let Some(target) = position.en_passant_target {
            // en passant
            if index - 7 == target {
                moves.push(HalfMove {
                    from: index,
                    to: (index - 7),
                    flag: Some(HalfmoveFlag::EnPassant),
                    is_capture: false,
                });
            }
        }
    }

    return moves;
}

fn quit_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().should_stop = true;
    shared_flags.lock().unwrap().should_quit = true;
}

fn register_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let token1 = command.next();

    if token1 == Some("later") {
        return;
    }

    parse_register_tokenset(command, token1, shared_flags);

    let token2 = command.next();

    parse_register_tokenset(command, token2, shared_flags);
}

fn parse_register_tokenset(
    command: &mut SplitWhitespace,
    token1: Option<&str>,
    shared_flags: &Arc<Mutex<SharedFlags>>,
) {
    match token1 {
        Some("name") => {
            if let Some(next_token) = command.next() {
                shared_flags.lock().unwrap().registration_name = next_token.parse().unwrap();
            }
        }
        Some("code") => {
            if let Some(next_token) = command.next() {
                shared_flags.lock().unwrap().registration_code = next_token.parse().unwrap();
            }
        }
        None => {}
        _ => println!(
            "Error - invalid register command, received {}",
            token1.unwrap()
        ),
    }
}

fn setoption_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    if command.next() != Some("name") {
        println!("Invalid setoption command - expected name token!");
        return;
    }

    let mut option = command.next();

    while option != None {
        match option {
            Some("MultiPV") => {
                if command.next() != Some("value") {
                    println!("Invalid setoption command - expected value token!");
                    return;
                }

                shared_flags.lock().unwrap().options.multi_pv =
                    command.next().unwrap().chars().nth(0).unwrap() as u8;
            }
            Some("DebugIndexes") => {
                if command.next() != Some("value") {
                    println!("Invalid setoption command - expected value token!");
                    return;
                }

                match command.next() {
                    Some("true") => shared_flags.lock().unwrap().options.debug_indexes = true,
                    Some("false") => shared_flags.lock().unwrap().options.debug_indexes = false,
                    _ => {
                        println!("Invalid setoption command - expected true or false!");
                        return;
                    }
                }
            }
            Some("DebugSetsDisplay") => {
                if command.next() != Some("value") {
                    println!("Invalid setoption command - expected value token!");
                    return;
                }

                match command.next() {
                    Some("true") => shared_flags.lock().unwrap().options.debug_sets_display = true,
                    Some("false") => {
                        shared_flags.lock().unwrap().options.debug_sets_display = false
                    }
                    _ => {
                        println!("Invalid setoption command - expected true or false!");
                        return;
                    }
                }
            }
            Some("DebugUseSymbols") => {
                if command.next() != Some("value") {
                    println!("Invalid setoption command - expected value token!");
                    return;
                }

                match command.next() {
                    Some("true") => shared_flags.lock().unwrap().options.debug_use_symbols = true,
                    Some("false") => shared_flags.lock().unwrap().options.debug_use_symbols = false,
                    _ => {
                        println!("Invalid setoption command - expected true or false!");
                        return;
                    }
                }
            }
            _ => {
                println!("Invalid option: {}!", option.unwrap());
                return;
            }
        }
        option = command.next();
    }

    // TODO: add malformed option command check
}

fn isready_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    // TODO: if engine is busy doing anything, wait for flags to finish
    // if calculating, return it immediately; no need to wait

    while !shared_flags.lock().unwrap().is_ready {
        thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("readyok");
}

fn debug_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    match command.next() {
        Some("on") => shared_flags.lock().unwrap().debug_enabled = true,
        Some("off") => shared_flags.lock().unwrap().debug_enabled = false,
        _ => println!("Debug command must select on or off!"),
    }
}
