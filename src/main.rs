use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Error;
use clap::Parser;

use hbt::{markdown, pinboard::Post};

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

fn md(args: &Args, input: &str) -> Result<(), Error> {
    let collection = markdown::parse(input)?;

    if args.dump {
        let entities = collection.entities();
        for entity in entities {
            println!("{}", entity.url())
        }
    } else {
        let length = collection.len();
        println!("{}: {} entities", args.file.to_string_lossy(), length)
    }

    Ok(())
}

fn xml(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_xml(input)?;

    if args.dump {
        for post in posts {
            println!("{}", post.href())
        }
    } else {
        let length = posts.len();
        println!("{}: {} posts", args.file.to_string_lossy(), length)
    }

    Ok(())
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    let file = &args.file;
    let maybe_extension = file.extension();
    let contents = fs::read_to_string(&file)?;

    match maybe_extension {
        Some(ext) if ext.as_encoded_bytes() == b"md" => md(&args, &contents)?,
        Some(ext) if ext.as_encoded_bytes() == b"xml" => xml(&args, &contents)?,
        _ => (),
    }

    Ok(ExitCode::SUCCESS)
}
