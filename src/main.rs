use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering, AtomicPtr};
use std::ptr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::str::SplitWhitespace;


struct SharedFlags {
    uci_enabled: bool,
    debug_enabled: bool,
    registration_name: String,
    registration_code: String
}


fn main() {

    let shared_flags =  Arc::new(Mutex::new(SharedFlags {
        uci_enabled: false,
        debug_enabled: false,
        registration_name: String::from("EndGame2"),
        registration_code: String::from("6399"),
    }));

    let shared_flags_clone = Arc::clone(&shared_flags);
    // Create a separate thread to read CLI input to allow interrupts
    std::thread::spawn(move || {
        handle_cli_input(shared_flags_clone);
    });

    // Main program logic
    loop {

        // print value of DEBUG_ENABLED
        println!("debug enabled: {}", shared_flags.lock().unwrap().debug_enabled);

        // print value of UCI_ENABLED
        println!("uci enabled: {}", shared_flags.lock().unwrap().uci_enabled);

        // print value of REGISTRATION_NAME
        println!("registration name: {}", shared_flags.lock().unwrap().registration_name);

        // print value of REGISTRATION_CODE
        println!("registration code: {}", shared_flags.lock().unwrap().registration_code);


        // Sleep for a while to simulate other work
        std::thread::sleep(std::time::Duration::from_secs(5));
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
            "isready" => isready_command(),
            "setoption" => setoption_command(&mut command, shared_flags),
            "register" => register_command(&mut command, shared_flags),
            "quit" => println!("test"),
            _ => println!("Unknown command!")
        }
    }
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

fn isready_command() {
    // TODO: if engine is busy doing anything, wait for flags to finish
    // if calculating, return it immediately; no need to wait

    // once tasks are done:
    println!("readyok");
}

fn debug_command(command: &mut SplitWhitespace, shared_flags: &Arc<Mutex<SharedFlags>>) {
    match command.next() {
        Some("on") => shared_flags.lock().unwrap().debug_enabled = true,
        Some("off") => shared_flags.lock().unwrap().debug_enabled = false,
        _ => println!("Debug command must select on or off!")
    }
}