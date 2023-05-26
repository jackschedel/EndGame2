use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::{fmt, thread};
use std::str::SplitWhitespace;
use hashbrown::HashSet;

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

#[derive(Debug, PartialEq)]
enum HalfmoveFlag {
    KnightPromotion,
    BishopPromotion,
    RookPromotion,
    QueenPromotion,
    Castle,
    EnPassant,
    DoublePawnMove,
}

impl Color {
    fn opposite(&self) -> Color {
        match *self {
            Color::Black => Color::White,
            Color::White => Color::Black
        }
    }
}

impl Piece {
    fn is_white(&self) -> bool {
        match *self {
            Piece::Pawn(Color::White)
            | Piece::Knight(Color::White)
            | Piece::Bishop(Color::White)
            | Piece::Rook(Color::White)
            | Piece::Queen(Color::White)
            | Piece::King(Color::White) => true,
            _ => false,
        }
    }

    fn is_black(&self) -> bool {
        match *self {
            Piece::Pawn(Color::Black)
            | Piece::Knight(Color::Black)
            | Piece::Bishop(Color::Black)
            | Piece::Rook(Color::Black)
            | Piece::Queen(Color::Black)
            | Piece::King(Color::Black) => true,
            _ => false,
        }
    }

    fn is_pawn(&self) -> bool {
        matches!(self, Piece::Pawn(_))
    }

    fn is_knight(&self) -> bool {
        matches!(self, Piece::Knight(_))
    }

    fn is_bishop(&self) -> bool {
        matches!(self, Piece::Bishop(_))
    }

    fn is_rook(&self) -> bool {
        matches!(self, Piece::Rook(_))
    }

    fn is_queen(&self) -> bool {
        matches!(self, Piece::Queen(_))
    }

    fn is_king(&self) -> bool {
        matches!(self, Piece::King(_))
    }

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
}

#[derive(PartialEq)]
struct HalfMove {
    from: u8,
    to: u8,
    flag: Option<HalfmoveFlag>,
}
impl fmt::Debug for HalfMove {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Customize the output of Debug for YourObject

        if self.flag == None {
            return write!(f, "[{} {}]", int_to_coord(self.from), int_to_coord(self.to));
        } else {
            return write!(f, "[{:?} {} {}]", self.flag, int_to_coord(self.from), int_to_coord(self.to));
        }

    }
}


#[derive(Clone)]
struct ColorCastlingRights {
    kingside: bool,
    queenside: bool,
}

#[derive(Clone)]
struct PieceSet {
    all: HashSet<u8>,
    white: HashSet<u8>,
    black: HashSet<u8>
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
#[derive(Clone)]
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
}


fn main() {


    let shared_flags =  Arc::new(Mutex::new(SharedFlags {
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
        options: EngineOptions {
            multi_pv: 1,
            debug_indexes: false,
            debug_sets_display: false,
            debug_use_symbols: false,
        }
    }));

    let shared_flags_clone = Arc::clone(&shared_flags);
    // Create a separate thread to read CLI input to allow interrupts
    thread::spawn(move || {
        handle_cli_input(shared_flags_clone);
    });

    // Main program logic
    let shared_flags_clone = Arc::clone(&shared_flags);


    handle_command("uci".to_string(), &shared_flags);

    handle_command("debug on".to_string(), &shared_flags);

    // general FEN loading
    let position_cmd = "position fen 8/8/4k3/1p2p2p/PPpn3P/2N4r/5K2/2R5 b - - 2 53 moves Nd4b3";

    // general moving around
    //let position_cmd = "position startpos moves e2e4 e7e5 Ng1f3 Nb8c6 Bf1b5 a7a6 Bb5xc6 d7xc6 e1h1 f7f6 d2d4";

    // en passant
    //let position_cmd = "position fen k7/4p2p/8/8/8/8/3P4/K7 w - - 0 1 moves d2d4 h7h6 d4d5 e7e5 d5xe6";

    // en passant -1
    // let position_cmd = "position fen k7/4p2p/8/8/8/8/3P4/K7 w - - 0 1 moves d2d4 h7h6 d4d5 e7e5";

    // en passant -1 lose flag
    // let position_cmd = "position fen k7/4p2p/8/8/8/8/3P4/K7 w - - 0 1 moves d2d4 e7e5 d4d5 h7h6";

    // castling kingside
    //let position_cmd = "position startpos moves g2g3 Ng8f6 Ng1f3 Nf6g8 Bf1g2 Ng8f6 e1h1 g7g6 Nf3e1 Bf8g7 Ne1f3 e8h8";

    // castling queenside
    //let position_cmd = "position startpos moves e2e3 e7e6 Qd1e2 Qd8e7 d2d3 d7d6 Bc1d2 Bc8d7 Nb1c3 Nb8c6 e1a1 e8a8";

    // startpos+1
    //let position_cmd = "position startpos moves e2e3";


    handle_command(position_cmd.to_string(), &shared_flags);

    // print_index_reference();



    /*
    thread::spawn(move ||  {
        loop {
            // print value of DEBUG_ENABLED
            println!("debug enabled: {}", shared_flags_clone.lock().unwrap().debug_enabled);

            // print value of UCI_ENABLED
            println!("uci enabled: {}", shared_flags_clone.lock().unwrap().uci_enabled);

            // print value of REGISTRATION_NAME
            println!("registration name: {}", shared_flags_clone.lock().unwrap().registration_name);

            // print value of REGISTRATION_CODE
            println!("registration code: {}", shared_flags_clone.lock().unwrap().registration_code);

            // Sleep for a while to simulate other work
            thread::sleep(std::time::Duration::from_secs(5));
        }
    });
     */

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

fn handle_command(input: String, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let mut command = input.trim().split_whitespace();
    if let Some(word) = command.next() {
        if !shared_flags.lock().unwrap().uci_enabled {
            if word == "uci" {
                uci_command(shared_flags);
            } else {
                println!("Please enable UCI mode first!")
            }
        } else {
            parse_command(shared_flags, &mut command, word);
        }
    }
}

fn parse_command(shared_flags: &Arc<Mutex<SharedFlags>>, mut command: &mut SplitWhitespace, word: &str) {
    match word {
        "uci" => uci_command(shared_flags),
        "debug" => debug_command(&mut command, shared_flags),
        "isready" => isready_command(shared_flags),
        "setoption" => setoption_command(&mut command, shared_flags),
        "register" => register_command(&mut command, shared_flags),
        "ucinewgame" => {},
        "position" => position_command(&mut command, shared_flags),
        "go" => go_command(&mut command, shared_flags),
        "stop" => stop_command(shared_flags),
        "ponderhit" => ponderhit_command(shared_flags),
        "quit" => quit_command(shared_flags),
        // todo: remove ref command
        "ref" => print_index_reference(),
        _ => println!("Error - Unknown command!")
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

    option_send(shared_flags);

    println!("uciok");
}

fn id_send(shared_flags: &Arc<Mutex<SharedFlags>>) {
    println!("id name {}", shared_flags.lock().unwrap().registration_name);
    println!("id author Koala");
}

fn option_send(shared_flags: &Arc<Mutex<SharedFlags>>) {
    println!("option name DebugIndexes type check default true");
    println!("option name DebugSetsDisplay type check default false");
    println!("option name DebugUseSymbols type check default false");
}

fn position_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let token1 = command.next();

    match token1 {
        Some("startpos") => {
            set_board_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR", shared_flags);
        },
        Some("fen") => {
            let fen = command.next().unwrap();
            set_board_from_fen(fen, shared_flags);
            set_flags_from_fen(command, shared_flags)
        },
        _ => println!("Position command improperly formatted!")
    }

    let token2 = command.next();

    if token2 == None {
        return;
    } else if token2.unwrap() != "moves" {
        println!("Error - expected moves token, got {}!", token2.unwrap());
        return;
    }

    let mut move_token = command.next();

    while move_token != None {

        let parsed_move = string_to_halfmove(shared_flags, move_token.unwrap());

        if parsed_move == None {
            println!("Error - unparsable move - {}", move_token.unwrap());
            break;
        } else {
            execute_halfmove(shared_flags, parsed_move.unwrap());
        }

        move_token = command.next();
    }
}

fn execute_halfmove(shared_flags: &Arc<Mutex<SharedFlags>>, to_exec: HalfMove) {
    // no legality checks, assumes that to_exec is legal

    shared_flags.lock().unwrap().position.halfmove_clock += 1;

    let piece: Piece;

    let color = shared_flags.lock().unwrap().position.board[to_exec.from as usize].unwrap().get_color();

    match to_exec.flag {
        Some(HalfmoveFlag::KnightPromotion) => {
            piece = Piece::Knight(color);
        },
        Some(HalfmoveFlag::BishopPromotion) => {
            piece = Piece::Bishop(color);
        },
        Some(HalfmoveFlag::RookPromotion) => {
            piece = Piece::Rook(color);
        },
        Some(HalfmoveFlag::QueenPromotion) => {
            piece = Piece::Queen(color);
        },
        _ => {
            piece = shared_flags.lock().unwrap().position.board[to_exec.from as usize].unwrap();
        }
    }

    if to_exec.flag != Some(HalfmoveFlag::Castle) {
        if shared_flags.lock().unwrap().position.board[to_exec.to as usize] != None ||
            shared_flags.lock().unwrap().position.board[to_exec.from as usize].unwrap().is_pawn() {
            shared_flags.lock().unwrap().position.halfmove_clock = 0;
        }

        shared_flags.lock().unwrap().position.board[to_exec.to as usize] = Some(piece);
        shared_flags.lock().unwrap().position.piece_set.add_index_or_color_swap(to_exec.to, color);
    } else {
        shared_flags.lock().unwrap().position.board[to_exec.to as usize] = None;
        shared_flags.lock().unwrap().position.piece_set.remove_index(to_exec.to, color);
        if color == Color::White {
            if to_exec.to == 0 {
                shared_flags.lock().unwrap().position.board[2] = Some(Piece::King(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(2, color);

                shared_flags.lock().unwrap().position.board[3] = Some(Piece::Rook(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(3, color);
            } else {
                // to_exec.to = 7
                shared_flags.lock().unwrap().position.board[6] = Some(Piece::King(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(6, color);

                shared_flags.lock().unwrap().position.board[5] = Some(Piece::Rook(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(5, color);

            }

            shared_flags.lock().unwrap().position.castling_rights.white.kingside = false;
            shared_flags.lock().unwrap().position.castling_rights.white.queenside = false;
        } else {
            if to_exec.to == 56 {
                shared_flags.lock().unwrap().position.board[58] = Some(Piece::King(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(58, color);

                shared_flags.lock().unwrap().position.board[59] = Some(Piece::Rook(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(59, color);
            } else {
                // to_exec.to = 63
                shared_flags.lock().unwrap().position.board[62] = Some(Piece::King(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(62, color);

                shared_flags.lock().unwrap().position.board[61] = Some(Piece::Rook(color));
                shared_flags.lock().unwrap().position.piece_set.add_index(61, color);

            }

            shared_flags.lock().unwrap().position.castling_rights.black.kingside = false;
            shared_flags.lock().unwrap().position.castling_rights.black.queenside = false;
        }
    }

    shared_flags.lock().unwrap().position.board[to_exec.from as usize] = None;
    shared_flags.lock().unwrap().position.piece_set.remove_index(to_exec.from, color);

    if to_exec.flag == Some(HalfmoveFlag::EnPassant) {
        let mut target = shared_flags.lock().unwrap().position.en_passant_target.unwrap();

        if (target / 8) == 5 {
            target -= 8;
        } else {
            target += 8;
        }

        shared_flags.lock().unwrap().position.board[target as usize] = None;
        shared_flags.lock().unwrap().position.piece_set.remove_index(target, color.opposite());
    }

    if to_exec.flag == Some(HalfmoveFlag::DoublePawnMove) {
        let middle_space:u8;

        if to_exec.from > to_exec.to {
            middle_space = to_exec.from - 8;
        } else {
            middle_space = to_exec.from + 8;
        }

        shared_flags.lock().unwrap().position.en_passant_target = Some(middle_space);
    } else {
        shared_flags.lock().unwrap().position.en_passant_target = None;
    }

    if shared_flags.lock().unwrap().position.move_next == Color::Black {
        shared_flags.lock().unwrap().position.fullmove_number += 1;
        shared_flags.lock().unwrap().position.move_next = Color::White;
    } else {
        shared_flags.lock().unwrap().position.move_next = Color::Black;
    }

    display_debug(shared_flags);

}

fn string_to_halfmove(shared_flags: &Arc<Mutex<SharedFlags>>, move_string: &str) -> Option<HalfMove> {
    let mut is_pieceless_move = true;

    let mut char_index = 0;

    let mut is_capture = false;

    match move_string.chars().nth(0) {
        Some('N') | Some('B') | Some('R') | Some('Q') | Some('K') => {
            is_pieceless_move = false;
            char_index += 1;
        },
        None => return None,
        _ => {}
    }

    let coord1_str: String = move_string.chars().skip(char_index).take(2).collect();
    let coord1 = coord_to_int(&coord1_str);

    char_index += 2;

    let coord_separator: char = move_string.chars().nth(char_index).unwrap();

    if coord_separator == '-'  {
        char_index += 1;
    } else if coord_separator == 'x'{
        char_index += 1;
        is_capture = true;
    }

    let coord2_str: String = move_string.chars().skip(char_index).take(2).collect();
    let coord2 = coord_to_int(&coord2_str);

    let mut flag: Option<HalfmoveFlag> = None;

    let board = shared_flags.lock().unwrap().position.board;

    if is_pieceless_move {
        // pawn action or castling

        if coord1 % 8 == coord2 % 8 {
            // straight pawn move (i.e. not a capture)

            if board[coord1 as usize] == None || !board[coord1 as usize].unwrap().is_pawn() {
                println!("Error - no pawn at {}!", coord1_str);
                return None;
            }

            match (coord2 / 8).abs_diff(coord1 / 8) {
                1 => {},
                2 => {
                    flag = Some(HalfmoveFlag::DoublePawnMove);
                },
                _ => {
                    println!("Error - invalid pawn move from {} to {}!", coord1_str, coord2_str);
                    return None;
                },
            }
        } else if is_capture {
            // pawn captures

            if board[coord1 as usize] == None || !board[coord1 as usize].unwrap().is_pawn() {
                println!("Error - no pawn at {}!", coord1_str);
                return None;
            }

            let file_diff = (coord1 % 8).abs_diff(coord2 % 8);

            let rank_diff = (coord1 / 8).abs_diff(coord2 / 8);

            if rank_diff > 1 || file_diff > 1{
                println!("Error - invalid pawn capture!");
                return None;
            }

            let en_passant_target = shared_flags.lock().unwrap().position.en_passant_target;

            if Some(coord2) == en_passant_target {
                flag = Some(HalfmoveFlag::EnPassant);
            }

            // note: technically no checks for backwards pawn captures
            // these checks are just for debugging, will want to add check vs genned moves later

        } else {
            // castle

            let from_pos=board[coord1 as usize];

            let to_pos=board[coord2 as usize];

            if from_pos == None || to_pos == None {
                println!("Error - invalid castle or forgot to specify piece!");
                return None;
            }

            let from_piece=from_pos.unwrap();

            let to_piece=to_pos.unwrap();



            let file_diff = (coord1 % 8).abs_diff(coord2 % 8);

            let rank_diff = (coord1 / 8).abs_diff(coord2 / 8);

            if !from_piece.is_king() || !to_piece.is_rook() || rank_diff != 0 || file_diff > 4{
                println!("Error - invalid castle or forgot to specify piece!");
                return None;
            }

            flag = Some(HalfmoveFlag::Castle);

            // note: no checks for whether the player is allowed to castle
            // these checks are just for debugging, will want to add check vs genned moves later

        }

        match move_string.chars().nth(char_index + 2) {
            Some('n') | Some('b') | Some('r') | Some('q') => {
                if board[coord1 as usize] == None || !board[coord1 as usize].unwrap().is_pawn() {
                    println!("Error - promoting, expected pawn!");
                    return None;
                }
                // note: no check for correct rank on promotion
            },
            None => {},
            _ => {
                println!("Error - unexpected promotion char: {:?}", move_string.chars().nth(char_index + 2));
                return None;
            }
        }

        match move_string.chars().nth(char_index + 2) {
            Some('n') => {
                flag = Some(HalfmoveFlag::KnightPromotion);
            },
            Some('b') => {
                flag = Some(HalfmoveFlag::BishopPromotion);
            },
            Some('r') => {
                flag = Some(HalfmoveFlag::RookPromotion);
            },
            Some('q') => {
                flag = Some(HalfmoveFlag::QueenPromotion);
            },
            _ => {}
        }

    }

    // note: no checks for if there are pieces in between the to + from
    // all checks should be tacked on after the fact
    // don't need to worry about efficiency because this will never be called unless debugging

    return Some(HalfMove {
        from: coord1,
        to: coord2,
        flag
    });
}

fn set_flags_from_fen(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let move_next_token = command.next();

    match move_next_token {
        Some("w") => {
            shared_flags.lock().unwrap().position.move_next = Color::White;
        },
        Some("b") => {
            shared_flags.lock().unwrap().position.move_next = Color::Black;
        },
        Some("moves") => return,
        _ => println!("Error - expected b or w, received {}", move_next_token.unwrap())
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

    if let Some(halfmove_clock_token) = command.next() {
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
            'Q' => shared_flags.lock().unwrap().position.castling_rights.white.queenside = true,
            'K' => shared_flags.lock().unwrap().position.castling_rights.white.kingside = true,
            'q' => shared_flags.lock().unwrap().position.castling_rights.black.queenside = true,
            'k' => shared_flags.lock().unwrap().position.castling_rights.black.kingside = true,
            '-' => {},
            _ => println!("Error - invalid castling rights, received {}", castling_rights_token)
        }
    }
}

fn set_board_from_fen(fen: &str, shared_flags: &Arc<Mutex<SharedFlags>>) {

    shared_flags.lock().unwrap().position.board = [None; 64];

    let mut index:usize = 56;

    for char in fen.chars() {
        if char == '/' {
            index -= 16;
        } else {
            handle_fen_char(shared_flags, &mut index, char);
            index += 1;
        }
    }

    display_debug(shared_flags);
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
            println!("All: {:?}", shared_flags.lock().unwrap().position.piece_set.all);
            println!("White: {:?}", shared_flags.lock().unwrap().position.piece_set.white);
            println!("Black: {:?}", shared_flags.lock().unwrap().position.piece_set.black);
        }
        println!();
        println!();
    }
}

fn handle_fen_char(shared_flags: &Arc<Mutex<SharedFlags>>, mut index: &mut usize, char: char) {
    match char {
        'P' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Pawn(Color::White)),
        'N' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Knight(Color::White)),
        'B' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Bishop(Color::White)),
        'R' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Rook(Color::White)),
        'Q' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Queen(Color::White)),
        'K' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::King(Color::White)),
        'p' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Pawn(Color::Black)),
        'n' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Knight(Color::Black)),
        'b' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Bishop(Color::Black)),
        'r' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Rook(Color::Black)),
        'q' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::Queen(Color::Black)),
        'k' => shared_flags.lock().unwrap().position.board[*index] = Some(Piece::King(Color::Black)),
        _ => handle_fen_digit(&mut index, char)
    }

    match char {
        'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => {
            shared_flags.lock().unwrap().position.piece_set.add_index(*index as u8, Color::White);
        },
        'p' | 'n' | 'b' | 'r' | 'q' | 'k' => {
            shared_flags.lock().unwrap().position.piece_set.add_index(*index as u8, Color::Black);
        },
        _ => { }
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
    let mut index:usize = 72;

    let mut column_num: u8 = 8;
    let horiz_space = "   ";

    for _i in 0..8  {
        index -= 16;
        print!("{} {}", column_num, horiz_space);
        column_num -= 1;
        for _j in 0..8  {
            let use_symbols = shared_flags.lock().unwrap().options.debug_use_symbols;
            let piece_char = piece_to_char(shared_flags.lock().unwrap().position.board[index], use_symbols);

            print!("{}{}", piece_char, horiz_space);
            index += 1;
        }
        println!();
    }
    println!();
    println!("  {}A{}B{}C{}D{}E{}F{}G{}H", horiz_space, horiz_space, horiz_space, horiz_space, horiz_space, horiz_space, horiz_space, horiz_space);
}

fn print_index_reference() {
    let mut index:usize = 72;

    for _i in 0..8  {
        index -= 16;
        for _j in 0..8  {
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
    let mut index:usize = 72;

    for _i in 0..8  {
        index -= 16;
        for _j in 0..8  {
            let use_symbols = shared_flags.lock().unwrap().options.debug_use_symbols;
            let piece_char = piece_to_char(shared_flags.lock().unwrap().position.board[index], use_symbols);

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
        if let Some(digit) = char.to_digit(9){
            *index += digit as usize - 1;
        }
    }
}

fn go_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    let position = shared_flags.lock().unwrap().position.clone();
    let color = position.move_next;
    println!("{:?}", gen_color_pseudolegal_moves(color, &position));
}

fn gen_color_pseudolegal_moves(color: Color, position: &Position) -> Vec<HalfMove> {

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
        moves.extend(gen_piece_pseudolegal_moves(i, position));

        // likely no need to gen new threads here, will likely be suboptimal due to thread overhead.
        // if no need for threads, we can pass moves as an address instead and return nothing
        // todo: test thread implementation performance
        // Our tree will exponentially grow so fast itd be pointless to do it here.

        // just a thought, if we make the eval properly, do we even need to check for legality?
    }

    return moves;
}

fn gen_piece_pseudolegal_moves(piece_index: u8, position: &Position) -> Vec<HalfMove> {

    let piece_option = position.board[piece_index as usize];

    match piece_option {
        Some(Piece::Pawn(Color::White)) => {return gen_white_pawn_pseudolegal_moves(piece_index, position)},
        Some(Piece::Pawn(Color::Black)) => {return gen_black_pawn_pseudolegal_moves(piece_index, position)},
        Some(Piece::Knight(_)) => {return gen_knight_pseudolegal_moves(piece_index, position)},
        Some(_) => {},
        None => panic!("Error, index contained in piece_set has no piece on board!")
    }

    return Vec::new();
}

fn gen_knight_pseudolegal_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    let board = position.board;

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

fn gen_halfmove(offset: i8, index: u8, position: &Position, moves: &mut Vec<HalfMove>) {

    if let Some(piece) = position.board[(index as i8 + offset) as usize ] {
        if piece.get_color() == position.move_next {
            return;
        }
    }

    moves.push(HalfMove{from: index, to: (index as i8 + offset) as u8, flag: None});

}

fn gen_white_pawn_pseudolegal_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    let board = position.board;
    let piece = board[index as usize].unwrap();
    let color = piece.get_color();

    // straight move
    if  board[(index + 8) as usize] == None {
        // nothing in the way
        if (index / 8) != 6 {
            moves.push(HalfMove{from: index, to: (index + 8), flag: None});
            if (index / 8 == 1) && board[(index + 16) as usize] == None {
                moves.push(HalfMove{from: index, to: (index + 16), flag: Some(HalfmoveFlag::DoublePawnMove)});
            }
        } else {
            // promotion
            moves.push(HalfMove{from: index, to: (index + 8), flag: Some(HalfmoveFlag::KnightPromotion)});
            moves.push(HalfMove{from: index, to: (index + 8), flag: Some(HalfmoveFlag::BishopPromotion)});
            moves.push(HalfMove{from: index, to: (index + 8), flag: Some(HalfmoveFlag::RookPromotion)});
            moves.push(HalfMove{from: index, to: (index + 8), flag: Some(HalfmoveFlag::QueenPromotion)});
        }
    }

    // captures
    if (index % 8) != 0 {
        // left capture
        if let Some(target) = board[(index + 7) as usize] {
            if target.get_color() != color {
                moves.push(HalfMove { from: index, to: (index + 7), flag: None });
            }
        } else if let Some(target) =  position.en_passant_target {
            // en passant
            if index + 7 == target {
                moves.push(HalfMove { from: index, to: (index + 7), flag: Some(HalfmoveFlag::EnPassant)});
            }
        }
    }
    if (index % 8) != 7 {
        // right capture
        if let Some(target) = board[(index + 9) as usize] {
            if target.get_color() != color {
                moves.push(HalfMove { from: index, to: (index + 9), flag: None });
            }
        } else if let Some(target) =  position.en_passant_target {
            // en passant
            if index + 9 == target {
                moves.push(HalfMove { from: index, to: (index + 9), flag: Some(HalfmoveFlag::EnPassant)});
            }
        }
    }

    return moves;
}

fn gen_black_pawn_pseudolegal_moves(index: u8, position: &Position) -> Vec<HalfMove> {
    let mut moves: Vec<HalfMove> = Vec::new();

    let board = position.board;
    let piece = board[index as usize].unwrap();
    let color = piece.get_color();

    // straight move
    if  board[(index - 8) as usize] == None {
        // nothing in the way
        if (index / 8) != 1 {
            moves.push(HalfMove{from: index, to: (index - 8), flag: None});
            if (index / 8 == 7) && board[(index - 16) as usize] == None {
                moves.push(HalfMove{from: index, to: (index - 16), flag: Some(HalfmoveFlag::DoublePawnMove)});
            }
        } else {
            // promotion
            moves.push(HalfMove{from: index, to: (index - 8), flag: Some(HalfmoveFlag::KnightPromotion)});
            moves.push(HalfMove{from: index, to: (index - 8), flag: Some(HalfmoveFlag::BishopPromotion)});
            moves.push(HalfMove{from: index, to: (index - 8), flag: Some(HalfmoveFlag::RookPromotion)});
            moves.push(HalfMove{from: index, to: (index - 8), flag: Some(HalfmoveFlag::QueenPromotion)});
        }
    }

    // captures (left/right orientation with white as bottom)
    if (index % 8) != 0 {
        // left capture
        if let Some(target) = board[(index - 9) as usize] {
            if target.get_color() != color {
                moves.push(HalfMove { from: index, to: (index - 9), flag: None });
            }
        } else if let Some(target) =  position.en_passant_target {
            // en passant
            if index - 9 == target {
                moves.push(HalfMove { from: index, to: (index - 9), flag: Some(HalfmoveFlag::EnPassant)});
            }
        }
    }
    if (index % 8) != 7 {
        // right capture
        if let Some(target) = board[(index - 7) as usize] {
            if target.get_color() != color {
                moves.push(HalfMove { from: index, to: (index - 7), flag: None });
            }
        } else if let Some(target) =  position.en_passant_target {
            // en passant
            if index - 7 == target {
                moves.push(HalfMove { from: index, to: (index - 7), flag: Some(HalfmoveFlag::EnPassant)});
            }
        }
    }

    return moves;
}

fn quit_command(shared_flags: &Arc<Mutex<SharedFlags>>) {
    shared_flags.lock().unwrap().should_stop = true;
    shared_flags.lock().unwrap().should_quit = true;

    // TODO: remove this line, should be set once computations are stored
    shared_flags.lock().unwrap().can_quit = true;

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

fn parse_register_tokenset(command: &mut SplitWhitespace, token1: Option<&str>, shared_flags: &Arc<Mutex<SharedFlags>>) {
    match token1 {
        Some("name") => {
            if let Some(next_token) = command.next() {
                shared_flags.lock().unwrap().registration_name = next_token.parse().unwrap();
            }
        },
        Some("code") => {
            if let Some(next_token) = command.next() {
                shared_flags.lock().unwrap().registration_code = next_token.parse().unwrap();
            }
        },
        None => {},
        _ => println!("Error - invalid register command, received {}", token1.unwrap())
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

                shared_flags.lock().unwrap().options.multi_pv = command.next().unwrap().chars().nth(0).unwrap() as u8;
            },
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
            },
            Some("DebugSetsDisplay") => {
                if command.next() != Some("value") {
                    println!("Invalid setoption command - expected value token!");
                    return;
                }

                match command.next() {
                    Some("true") => shared_flags.lock().unwrap().options.debug_sets_display = true,
                    Some("false") => shared_flags.lock().unwrap().options.debug_sets_display = false,
                    _ => {
                        println!("Invalid setoption command - expected true or false!");
                        return;
                    }
                }
            },
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
            },
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
        _ => println!("Debug command must select on or off!")
    }
}