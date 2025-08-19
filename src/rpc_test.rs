fn main() -> std::io::Result<()> {
    let mut buf = String::new();
    println!("Hello");
    'reading: loop {
        buf.clear();
        let _read = std::io::stdin().read_line(&mut buf)?;
        let cmd = buf.trim_end_matches('\n');
        match cmd {
            "quit" => {
                println!("Quitting");
                break 'reading;
            }
            msg if msg.len() > 0 => {
                eprintln!("Unknown command '{msg}'");
            }
            msg => {
                assert!(msg.is_empty());
                eprintln!("Unknown empty '{msg}'");
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}
