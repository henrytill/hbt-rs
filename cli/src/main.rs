use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use anyhow::Error;
use clap::Parser;
use schemars::schema_for;

use hbt_core::collection::{Collection, CollectionRepr};
use hbt_core::format::{Format, INPUT, OUTPUT};

use hbt::version;

#[derive(Parser, Debug)]
#[command(about, long_about = None, version = version::version_info().to_string())]
struct Args {
    /// Input format
    #[arg(short = 'f', long = "from", value_enum)]
    from: Option<Format<INPUT>>,

    /// Output format
    #[arg(short = 't', long = "to", value_enum)]
    to: Option<Format<OUTPUT>>,

    /// Output file (defaults to stdout)
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Show collection info (entity count)
    #[arg(long = "info")]
    info: bool,

    /// List all tags
    #[arg(long = "list-tags")]
    list_tags: bool,

    /// Output Collection JSON schema
    #[arg(long = "schema")]
    schema: bool,

    /// Read mappings from <FILE>
    #[arg(long = "mappings", value_name = "FILE")]
    mappings: Option<PathBuf>,

    /// Input file
    file: Option<PathBuf>,
}

fn update_collection(args: &Args, coll: &mut Collection) -> Result<(), Error> {
    let Some(mappings) = &args.mappings else { return Ok(()) };

    let contents = fs::read_to_string(mappings)?;
    let yaml: serde_yaml::Value = serde_yaml::from_str(&contents)?;

    let mappings = yaml
        .as_mapping()
        .ok_or_else(|| Error::msg("Mapping file must contain a YAML mapping"))?
        .iter()
        .filter_map(|(k, v)| {
            let key = k.as_str()?.to_string();
            let value = v.as_str()?.to_string();
            Some((key, value))
        })
        .collect::<Vec<_>>();

    coll.update_labels(mappings)?;

    Ok(())
}

fn write_output(writer: &mut dyn Write, content: &str) -> Result<(), Error> {
    writer.write_all(content.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn print_collection(args: &Args, coll: &Collection) -> Result<(), Error> {
    let output = if args.info {
        let length = coll.len();
        let file_name = args.file.as_ref().map(|f| f.to_string_lossy()).unwrap_or("input".into());
        format!("{}: {} entities\n", file_name, length)
    } else if args.list_tags {
        let mut all_tags = BTreeSet::new();
        for entity in coll.entities() {
            all_tags.extend(entity.labels())
        }
        let tags_output = all_tags.iter().map(|tag| tag.as_str()).collect::<Vec<_>>().join("\n");
        if tags_output.is_empty() { String::new() } else { format!("{}\n", tags_output) }
    } else if let Some(format) = &args.to {
        format.unparse(coll)?
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

fn process_input(args: &Args, input: &str, format: Format<INPUT>) -> Result<(), Error> {
    let mut coll = format.parse(input)?;
    update_collection(args, &mut coll)?;
    print_collection(args, &coll)?;
    Ok(())
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    // Handle schema output (no input file required)
    if args.schema {
        let schema = schema_for!(CollectionRepr);
        let schema_json = serde_json::to_string_pretty(&schema)?;
        if let Some(output_file) = &args.output {
            let mut file = std::fs::File::create(output_file)?;
            write_output(&mut file, &schema_json)?;
        } else {
            write_output(&mut io::stdout(), &schema_json)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let file = args.file.as_ref().ok_or_else(|| Error::msg("Input file required"))?;
    let contents = fs::read_to_string(file)?;

    let input_format = match &args.from {
        Some(format) => *format,
        None => {
            let no_parser = || Error::msg(format!("No parser for file: {}", file.display()));
            Format::<INPUT>::detect(file).ok_or_else(no_parser)?
        }
    };

    process_input(&args, &contents, input_format)?;

    Ok(ExitCode::SUCCESS)
}
