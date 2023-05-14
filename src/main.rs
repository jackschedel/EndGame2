use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::str::SplitWhitespace;

#[derive(Debug, Copy, Clone)]
enum Color {
    Black,
    White,
}

#[derive(Debug, Copy, Clone)]
enum Piece {
    Pawn(Color),
    Knight(Color),
    Bishop(Color),
    Rook(Color),
    Queen(Color),
    King(Color),
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
}

struct ColorCastlingRights {
    kingside: bool,
    queenside: bool,
}

struct CastlingRights {
    black: ColorCastlingRights,
    white: ColorCastlingRights,
}

struct Position {
    board: [Option<Piece>; 64],
    move_next: Color,
    castling_rights: CastlingRights,
    en_passant_target: Option<u8>,
    halfmove_clock: u16,
    fullmove_number: u16,
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
    }));

    let shared_flags_clone = Arc::clone(&shared_flags);
    // Create a separate thread to read CLI input to allow interrupts
    thread::spawn(move || {
        handle_cli_input(shared_flags_clone);
    });

    /*
    // Main program logic
    let shared_flags_clone = Arc::clone(&shared_flags);

    shared_flags_clone.lock().unwrap().debug_enabled = true;
    set_board_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR", &shared_flags_clone);
    set_board_from_fen("r1b1k1nr/p2p1pNp/n2B4/1p1NP2P/6P1/3P1Q2/P1P1K3/q5b1", &shared_flags_clone);
    */

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
                shared_flags.lock().unwrap().uci_enabled = true;
            } else {
                println!("Please enable UCI mode first!")
            }
        } else {
            parse_cli_command(shared_flags, &mut command, word);
        }
    }
}

fn parse_cli_command(shared_flags: &Arc<Mutex<SharedFlags>>, mut command: &mut SplitWhitespace, word: &str) {
    match word {
        "uci" => shared_flags.lock().unwrap().uci_enabled = true,
        "debug" => debug_command(&mut command, shared_flags),
        "isready" => isready_command(shared_flags),
        "setoption" => setoption_command(&mut command, shared_flags),
        "register" => register_command(&mut command, shared_flags),
        "ucinewgame" => {},
        "position" => position_command(&mut command, shared_flags),
        "go" => go_command(&mut command, shared_flags),
        "stop" => shared_flags.lock().unwrap().should_stop = true,
        "ponderhit" => shared_flags.lock().unwrap().ponder_hit = true,
        "quit" => quit_command(shared_flags),
        _ => println!("Error - Unknown command!")
    }
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

    parse_register_tokenset(command, token1, shared_flags);

    let token2 = command.next();

    parse_register_tokenset(command, token2, shared_flags);
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
            let en_passant_target = Option::from(coord_to_int(en_passant_token));
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

    if shared_flags.lock().unwrap().debug_enabled {
        println!();
        print_board(shared_flags);
    }
}

fn handle_fen_char(shared_flags: &Arc<Mutex<SharedFlags>>, mut index: &mut usize, char: char) {
    match char {
        'P' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Pawn(Color::White)),
        'N' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Knight(Color::White)),
        'B' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Bishop(Color::White)),
        'R' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Rook(Color::White)),
        'Q' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Queen(Color::White)),
        'K' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::King(Color::White)),
        'p' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Pawn(Color::Black)),
        'n' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Knight(Color::Black)),
        'b' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Bishop(Color::Black)),
        'r' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Rook(Color::Black)),
        'q' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::Queen(Color::Black)),
        'k' => shared_flags.lock().unwrap().position.board[*index] = Option::from(Piece::King(Color::Black)),
        _ => handle_fen_digit(&mut index, char)
    }
}

fn piece_to_char(piece: Option<Piece>) -> char {
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

fn print_board(shared_flags: &Arc<Mutex<SharedFlags>>) {
    let mut index:usize = 72;

    for _i in 0..8  {
        index -= 16;
        for _j in 0..8  {
            print!("{}  ", piece_to_char(shared_flags.lock().unwrap().position.board[index]));
            index += 1;
        }
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
    match command.next() {
        //Some("") => {},
        _ => println!("Invalid option!")
    }
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