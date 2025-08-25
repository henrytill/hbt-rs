use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::Error;
use clap::Parser;

use hbt_core::collection::Collection;
#[cfg(feature = "pinboard")]
use hbt_core::collection::Entity;
use hbt_core::markdown;
#[cfg(feature = "pinboard")]
use hbt_core::pinboard::Post;

use hbt::version;

#[derive(clap::ValueEnum, Debug, Clone)]
enum InputFormat {
    Html,
    #[cfg(feature = "pinboard")]
    Json,
    #[cfg(feature = "pinboard")]
    Xml,
    Markdown,
}

#[derive(clap::ValueEnum, Debug, Clone)]
enum OutputFormat {
    Yaml,
    Html,
}

#[derive(Parser, Debug)]
#[command(about, long_about = None, version = version::version_info().to_string())]
struct Args {
    /// Input format
    #[arg(short = 'f', long = "from", value_enum)]
    from: Option<InputFormat>,
    /// Output format
    #[arg(short = 't', long = "to", value_enum)]
    to: Option<OutputFormat>,
    /// Output file (defaults to stdout)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,
    /// Show collection info (entity count)
    #[arg(long = "info")]
    info: bool,
    /// List all tags
    #[arg(long = "list-tags")]
    list_tags: bool,
    /// Read mappings from <FILE>
    #[arg(long = "mappings", value_name = "FILE")]
    mappings: Option<PathBuf>,
    /// Input file
    file: PathBuf,
}

#[cfg(feature = "pinboard")]
fn create_collection(mut posts: Vec<Post>) -> Result<Collection, Error> {
    // Sort posts by timestamp to match OCaml version behavior
    posts.sort_by(|a, b| a.time.cmp(&b.time));

    let mut ret = Collection::with_capacity(posts.len());
    for post in posts {
        let entity = Entity::try_from(post)?;
        ret.insert(entity);
    }
    Ok(ret)
}

fn detect_input_format(file: &Path) -> Result<InputFormat, Error> {
    let maybe_extension = file.extension();
    match maybe_extension {
        Some(ext) if ext.as_encoded_bytes() == b"html" => Ok(InputFormat::Html),
        #[cfg(feature = "pinboard")]
        Some(ext) if ext.as_encoded_bytes() == b"json" => Ok(InputFormat::Json),
        #[cfg(feature = "pinboard")]
        Some(ext) if ext.as_encoded_bytes() == b"xml" => Ok(InputFormat::Xml),
        Some(ext) if ext.as_encoded_bytes() == b"md" => Ok(InputFormat::Markdown),
        Some(ext) => Err(Error::msg(format!("No parser for extension: {}", ext.to_string_lossy()))),
        _ => Err(Error::msg(format!("No parser for file: {}", file.display()))),
    }
}

fn update_collection(args: &Args, collection: &mut Collection) -> Result<(), Error> {
    if let Some(mappings) = &args.mappings {
        let contents = fs::read_to_string(mappings)?;
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&contents)?;

        let mappings = yaml_value
            .as_mapping()
            .ok_or_else(|| Error::msg("Mapping file must contain a YAML mapping"))?
            .iter()
            .filter_map(|(k, v)| {
                let key = k.as_str()?.to_string();
                let value = v.as_str()?.to_string();
                Some((key, value))
            })
            .collect::<Vec<_>>();

        collection.update_labels(mappings)?;
    }
    Ok(())
}

fn write_output(writer: &mut dyn Write, content: &str) -> Result<(), Error> {
    writer.write_all(content.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn print_collection(args: &Args, collection: &Collection) -> Result<(), Error> {
    let output = if args.info {
        let length = collection.len();
        format!("{}: {} entities\n", args.file.to_string_lossy(), length)
    } else if args.list_tags {
        let mut all_tags = BTreeSet::new();
        for entity in collection.entities() {
            all_tags.extend(entity.labels())
        }
        let tags_output = all_tags.iter().map(|tag| tag.as_str()).collect::<Vec<_>>().join("\n");
        if tags_output.is_empty() { String::new() } else { format!("{}\n", tags_output) }
    } else if let Some(format) = &args.to {
        match format {
            OutputFormat::Yaml => serde_yaml::to_string(collection)?,
            OutputFormat::Html => collection.to_html()?,
        }
    } else {
        return Err(Error::msg(
            "Must specify an output format (-t) or analysis flag (--info, --list-tags)",
        ));
    };

    if let Some(output_file) = &args.output {
        let mut file = std::fs::File::create(output_file)?;
        write_output(&mut file, &output)?;
    } else {
        write_output(&mut io::stdout(), &output)?;
    }

    Ok(())
}

fn process_input(args: &Args, input: &str, format: InputFormat) -> Result<(), Error> {
    let mut collection = match format {
        InputFormat::Html => Collection::from_html_str(input)?,
        #[cfg(feature = "pinboard")]
        InputFormat::Json => {
            let posts = Post::from_json(input)?;
            create_collection(posts)?
        }
        #[cfg(feature = "pinboard")]
        InputFormat::Xml => {
            let posts = Post::from_xml(input)?;
            create_collection(posts)?
        }
        InputFormat::Markdown => markdown::parse(input)?,
    };

    update_collection(args, &mut collection)?;
    print_collection(args, &collection)?;
    Ok(())
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    let file = &args.file;
    let contents = fs::read_to_string(file)?;

    let input_format = match &args.from {
        Some(format) => format.clone(),
        None => detect_input_format(file)?,
    };

    process_input(&args, &contents, input_format)?;

    Ok(ExitCode::SUCCESS)
}
