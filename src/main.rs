use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Error;
use clap::Parser;

use hbt::markdown;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Dump all entries
    #[arg(short, long)]
    dump: bool,
    /// File to read
    #[arg(required = true)]
    file: PathBuf,
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    let file = args.file;
    let contents = fs::read_to_string(&file)?;
    let collection = markdown::parse(&contents)?;

    if args.dump {
        let entities = collection.entities();
        for entity in entities {
            println!("{}", entity.url())
        }
    } else {
        let length = collection.len();
        println!("{}: {} entities", file.to_string_lossy(), length)
    }

    Ok(ExitCode::SUCCESS)
}
