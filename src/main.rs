use std::io::{self, BufRead};
use std::sync::atomic::{AtomicBool, Ordering, AtomicPtr};
use std::ptr;
use std::str::SplitWhitespace;


static UCI_ENABLED: AtomicBool = AtomicBool::new(false);
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);
static REGISTRATION_NAME: AtomicPtr<String> = AtomicPtr::new(ptr::null_mut());
static REGISTRATION_CODE: AtomicPtr<String> = AtomicPtr::new(ptr::null_mut());



fn main() {
    REGISTRATION_NAME.store(Box::into_raw(Box::new("".to_string())), Ordering::Relaxed);
    REGISTRATION_CODE.store(Box::into_raw(Box::new("".to_string())), Ordering::Relaxed);

    // Create a separate thread to read CLI input to allow interrupts
    std::thread::spawn(|| {
        handle_cli_input();
    });

    // Main program logic
    loop {


        // print value of DEBUG_ENABLED
        println!("debug enabled: {}", DEBUG_ENABLED.load(Ordering::Relaxed));

        // print value of UCI_ENABLED
        println!("uci enabled: {}", UCI_ENABLED.load(Ordering::Relaxed));

        // print value of REGISTRATION_NAME
        let registration_name_ptr = REGISTRATION_NAME.load(Ordering::SeqCst);
        let registration_name_string = unsafe { Box::from_raw(registration_name_ptr) };
        println!("registration name: {}", *registration_name_string);

        // print value of REGISTRATION_CODE
        let registration_code_ptr = REGISTRATION_CODE.load(Ordering::SeqCst);
        let registration_code_string = unsafe { Box::from_raw(registration_code_ptr) };
        println!("registration code: {}", *registration_code_string);


        // Sleep for a while to simulate other work
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}

fn handle_cli_input() {
    for line in io::stdin().lock().lines() {
        if let Ok(input) = line {
            handle_command(input);
        }
    }
}

fn handle_command(input: String) {
    let mut command = input.trim().split_whitespace();
    if let Some(word) = command.next() {
        match word {
            "uci" => UCI_ENABLED.store(true, Ordering::Relaxed),
            "debug" => debug_command(&mut command),
            "isready" => isready_command(),
            "setoption" => setoption_command(&mut command),
            "register" => register_command(&mut command),
            "quit" => println!("test"),
            _ => println!("Unknown command!")
        }
    }
}

fn register_command(command: &mut SplitWhitespace) {
    let token1 = command.next();

    if token1 == Some("later") {
        return;
    }

    parse_register_tokenset(command, token1);

    let token2 = command.next();

    parse_register_tokenset(command, token2);

}

fn parse_register_tokenset(command: &mut SplitWhitespace, token1: Option<&str>) {
    match token1 {
        Some("name") => {
            if let Some(next_token) = command.next() {
                let token_box = Box::into_raw(Box::new(next_token.to_string()));
                REGISTRATION_NAME.store(token_box, Ordering::Relaxed);
            }
        },
        Some("code") => {
            if let Some(next_token) = command.next() {
                let token_box = Box::into_raw(Box::new(next_token.to_string()));
                REGISTRATION_CODE.store(token_box, Ordering::Relaxed);
            }
        },
        Some("") => {},
        _ => println!("Register command improperly formatted!")
    }
}

fn setoption_command(command: &mut SplitWhitespace) {
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

fn debug_command(command: &mut SplitWhitespace) {
    match command.next() {
        Some("on") => DEBUG_ENABLED.store(true, Ordering::Relaxed),
        Some("off") => DEBUG_ENABLED.store(false, Ordering::Relaxed),
        _ => println!("Debug command must select on or off!")
    }
}