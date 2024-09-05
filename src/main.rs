use std::{env, fs, process::ExitCode};

use anyhow::Error;

use hbt::markdown;

fn main() -> Result<ExitCode, Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        let exe = args[0].to_owned();
        eprintln!("Usage: {} <file>", exe);
        return Ok(ExitCode::FAILURE);
    }
    let file = &args[1];
    let contents = fs::read_to_string(file)?;
    let collection = markdown::parse(&contents)?;
    let entities = collection.entities();
    for entity in entities {
        println!("{}", entity.url())
    }
    Ok(ExitCode::SUCCESS)
}
