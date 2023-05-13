use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::str::SplitWhitespace;

#[derive(Copy, Clone)]
enum Color {
    Black,
    White,
}

#[derive(Copy, Clone)]
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

struct Position {
    board: [Option<Piece>; 64],
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
            board: [None; 64]
            },
    }));

    let shared_flags_clone = Arc::clone(&shared_flags);
    // Create a separate thread to read CLI input to allow interrupts
    thread::spawn(move || {
        handle_cli_input(shared_flags_clone);
    });

    // Main program logic
    let shared_flags_clone = Arc::clone(&shared_flags);

    thread::spawn(move ||  {

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
    });

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
            _ => println!("Unknown command!")
        }
    }
}

fn position_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {

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
        _ => println!("Register command improperly formatted!")
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