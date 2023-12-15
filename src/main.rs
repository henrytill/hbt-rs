use std::{env, fs};

use anyhow::Result;

use hbt::markdown;

#[derive(Debug)]
enum Error {
    Usage(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Usage(exe) => write!(f, "Usage: {} <file>", exe),
        }
    }
}

impl std::error::Error for Error {}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        let exe = args[0].to_owned();
        return Err(Into::into(Error::Usage(exe)));
    }
    let file = &args[1];
    let contents = fs::read_to_string(file)?;
    let collection = markdown::parse(&contents)?;
    println!("collection.len(): {}", collection.len());
    Ok(())
}
