use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
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

fn update(args: &Args, coll: &mut Collection) -> Result<(), Error> {
    let Some(mappings) = &args.mappings else {
        return Ok(());
    };

    let contents = fs::read_to_string(mappings)?;
    let yaml: serde_norway::Value = serde_norway::from_str(&contents)?;

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

fn print(args: &Args, coll: &Collection) -> Result<(), Error> {
    if args.info {
        let length = coll.len();
        let file_name = args
            .file
            .as_ref()
            .map(|f| f.to_string_lossy())
            .unwrap_or("input".into());
        let output = format!("{}: {} entities\n", file_name, length);
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout);
        writer.write_all(output.as_bytes())?;
    } else if args.list_tags {
        let mut all_tags = BTreeSet::new();
        for entity in coll.entities() {
            all_tags.extend(entity.labels())
        }
        let tags_output = all_tags
            .iter()
            .map(|tag| tag.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let output = if tags_output.is_empty() {
            String::new()
        } else {
            format!("{}\n", tags_output)
        };
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout);
        writer.write_all(output.as_bytes())?;
    } else if let Some(format) = &args.to {
        if let Some(output_file) = &args.output {
            let file = File::create(output_file)?;
            let writer = BufWriter::new(file);
            format.unparse(writer, coll)?
        } else {
            let stdout = io::stdout();
            let writer = BufWriter::new(stdout);
            format.unparse(writer, coll)?
        };
    } else {
        return Err(Error::msg(
            "Must specify an output format (-t) or analysis flag (--info, --list-tags)",
        ));
    };

    Ok(())
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    if args.schema {
        let schema = schema_for!(CollectionRepr);
        if let Some(output_file) = &args.output {
            let file = File::create(output_file)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &schema)?;
            writer.flush()?;
        } else {
            let stdout = io::stdout();
            let mut writer = BufWriter::new(stdout);
            serde_json::to_writer_pretty(&mut writer, &schema)?;
            writer.flush()?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let file = args
        .file
        .as_ref()
        .ok_or_else(|| Error::msg("Input file required"))?;

    let input_format = match &args.from {
        Some(format) => *format,
        None => {
            let no_parser = || Error::msg(format!("No parser for file: {}", file.display()));
            Format::<INPUT>::detect(file).ok_or_else(no_parser)?
        }
    };

    let f = File::open(file)?;
    let mut reader = BufReader::new(f);
    let mut coll = input_format.parse(&mut reader)?;
    update(&args, &mut coll)?;
    print(&args, &coll)?;

    Ok(ExitCode::SUCCESS)
}
