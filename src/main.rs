use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Error;
use clap::Parser;

use hbt::markdown;
#[cfg(feature = "pinboard")]
use hbt::pinboard::Post;

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

#[cfg(feature = "pinboard")]
fn print_posts(args: &Args, posts: Vec<Post>) -> Result<(), Error> {
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

#[cfg(feature = "pinboard")]
fn html(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_html(input)?;
    print_posts(args, posts)
}

#[cfg(feature = "pinboard")]
fn json(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_json(input)?;
    print_posts(args, posts)
}

#[cfg(feature = "pinboard")]
fn xml(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_xml(input)?;
    print_posts(args, posts)
}

fn markdown(args: &Args, input: &str) -> Result<(), Error> {
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

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    let file = &args.file;
    let maybe_extension = file.extension();
    let contents = fs::read_to_string(file)?;

    match maybe_extension {
        #[cfg(feature = "pinboard")]
        Some(ext) if ext.as_encoded_bytes() == b"html" => html(&args, &contents)?,
        #[cfg(feature = "pinboard")]
        Some(ext) if ext.as_encoded_bytes() == b"json" => json(&args, &contents)?,
        #[cfg(feature = "pinboard")]
        Some(ext) if ext.as_encoded_bytes() == b"xml" => xml(&args, &contents)?,
        Some(ext) if ext.as_encoded_bytes() == b"md" => markdown(&args, &contents)?,
        Some(ext) => {
            return Err(Error::msg(format!("No parser for extension: {}", ext.to_string_lossy())));
        }
        _ => {
            return Err(Error::msg(format!("No parser for file: {}", file.display())));
        }
    }

    Ok(ExitCode::SUCCESS)
}
