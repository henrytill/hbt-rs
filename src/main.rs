use std::{env, fmt, fs, io, process::ExitCode};

use hbt::markdown;

#[derive(Debug)]
enum Error {
    Usage(String),
    Io(io::Error),
    Markdown(markdown::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Usage(exe) => write!(f, "Usage: {} <file>", exe),
            Error::Io(err) => err.fmt(f),
            Error::Markdown(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Usage(_) => None,
            Error::Io(err) => Some(err),
            Error::Markdown(err) => Some(err),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<markdown::Error> for Error {
    fn from(err: markdown::Error) -> Error {
        Error::Markdown(err)
    }
}

fn run() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        let exe = args[0].to_owned();
        return Err(Error::Usage(exe));
    }
    let file = &args[1];
    let contents = fs::read_to_string(file)?;
    let collection = markdown::parse(&contents)?;
    println!("collection.len(): {}", collection.len());
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::FAILURE
        }
    }
}
