use crate::commands::{Command, CommandIPC};

mod commands;

fn main() {
    println!("Hello");
    let (mut cmd, handle) = CommandIPC::new(|| println!("Notify!"));
    loop {
        if let Some(c) = cmd.try_pull() {
            match c {
                Command::Quit => break,
                Command::Echo(s) => println!("Echo {s}"),
                Command::Select(s) => println!("Select {s}"),
            }
        }
    }
    let _ = handle.join();
}
