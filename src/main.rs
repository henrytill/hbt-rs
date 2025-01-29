use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::Error;
use clap::Parser;

use hbt::collection::Collection;
#[cfg(feature = "pinboard")]
use hbt::collection::Entity;
use hbt::markdown;
#[cfg(feature = "pinboard")]
use hbt::pinboard::Post;
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Dump all entries
    #[arg(short, long)]
    dump: bool,
    /// File to read
    #[arg(required = true)]
    file: PathBuf,
    /// Read mappings from <FILE>
    #[arg(short, long, value_name = "FILE")]
    mappings: Option<PathBuf>,
}

#[cfg(feature = "pinboard")]
fn create_collection(posts: Vec<Post>) -> Result<Collection, Error> {
    let mut ret = Collection::with_capacity(posts.len());
    for post in posts {
        let entity = Entity::try_from(post)?;
        ret.insert(entity);
    }
    Ok(ret)
}

fn update_collection(args: &Args, collection: &mut Collection) -> Result<(), Error> {
    if let Some(mappings) = &args.mappings {
        let contents = fs::read_to_string(mappings)?;
        let contents_value: Value = serde_json::from_str(&contents)?;
        collection.update_labels(contents_value)?;
    }
    Ok(())
}

fn print_collection(args: &Args, collection: &Collection) {
    if args.dump {
        let entities = collection.entities();
        for entity in entities {
            println!("{}", entity.url())
        }
    } else {
        let length = collection.len();
        println!("{}: {} entities", args.file.to_string_lossy(), length)
    }
}

#[cfg(feature = "pinboard")]
fn html(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_html(input)?;
    let mut collection = create_collection(posts)?;
    update_collection(args, &mut collection)?;
    print_collection(args, &collection);
    Ok(())
}

#[cfg(feature = "pinboard")]
fn json(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_json(input)?;
    let mut collection = create_collection(posts)?;
    update_collection(args, &mut collection)?;
    print_collection(args, &collection);
    Ok(())
}

#[cfg(feature = "pinboard")]
fn xml(args: &Args, input: &str) -> Result<(), Error> {
    let posts = Post::from_xml(input)?;
    let mut collection = create_collection(posts)?;
    update_collection(args, &mut collection)?;
    print_collection(args, &collection);
    Ok(())
}

fn markdown(args: &Args, input: &str) -> Result<(), Error> {
    let mut collection = markdown::parse(input)?;
    update_collection(args, &mut collection)?;
    print_collection(args, &collection);
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
