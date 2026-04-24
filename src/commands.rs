use std::io::Write;
use std::{collections::VecDeque, str::FromStr, sync::Arc, thread::JoinHandle};

#[derive(Debug)]
pub enum Command {
    Quit,
    Echo(String),
    Select(String),
}

#[derive(Debug)]
pub enum NvimCommand {
    /// Opens the node in the current window.
    /// roam does the UUID -> filename transform on the nvim side
    Open(String),
    Echo(String),
}

impl NvimCommand {
    fn write(&self, stream: &mut impl Write) -> std::io::Result<()> {
        match self {
            NvimCommand::Open(id) => writeln!(stream, "open {id}"),
            NvimCommand::Echo(pong) => writeln!(stream, "echo {pong}"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Empty,
    Unknown,
    MissingArg,
}

impl FromStr for Command {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::Empty);
        }
        let mut command = s;
        let mut args: Option<&str> = None;
        if let Some((c, a)) = s.split_once(' ') {
            command = c;
            args = Some(a);
        }
        match (command, args) {
            ("quit", _) => Ok(Command::Quit),
            ("echo", args) => {
                if let Some(args) = args {
                    Ok(Command::Echo(args.to_string()))
                } else {
                    Ok(Command::Echo("pong".to_string()))
                }
            }
            ("select", args) => {
                if let Some(args) = args {
                    Ok(Command::Select(args.to_string()))
                } else {
                    Err(Error::MissingArg)
                }
            }
            (_, _) => Err(Error::Unknown),
        }
    }
}

#[derive(Clone)]
pub struct CommandIPC {
    commands: Arc<std::sync::Mutex<VecDeque<Command>>>,
}

impl CommandIPC {
    pub fn new<F>(notify: F) -> (Self, JoinHandle<()>)
    where
        F: Fn() + Send + 'static,
    {
        let s = Self {
            commands: Arc::default(),
        };
        let handle = s.read_stdin(notify);
        (s, handle)
    }

    pub fn send_to_nvim(&self, command: NvimCommand) {
        let _ = command.write(&mut std::io::stdout());
    }

    pub fn push(&mut self, command: Command) {
        {
            let mut q = self.commands.lock().expect("push");
            q.push_back(command);
        }
    }

    pub fn try_pull(&mut self) -> Option<Command> {
        let mut q = self.commands.lock().expect("try_pull");
        q.pop_front()
    }

    fn read_stdin<F>(&self, notify: F) -> std::thread::JoinHandle<()>
    where
        F: Fn() + Send + 'static,
    {
        let mut write_end = self.clone();
        std::thread::spawn(move || {
            let mut buf = String::new();
            let mut push = |c| {
                write_end.push(c);
                notify();
            };

            loop {
                buf.clear();
                if let Err(e) = std::io::stdin().read_line(&mut buf) {
                    eprintln!("Error: {e}");
                    continue;
                }
                let cmd = buf.trim_end_matches('\n');
                if let Err(e) = cmd.parse().map(&mut push) {
                    eprintln!("Error: {e:?}");
                }
            }
        })
    }
}
