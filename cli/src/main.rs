#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![deny(clippy::unwrap_in_result)]

use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
    process::ExitCode,
};

use anyhow::{Context, Error};
use clap::Parser;
use schemars::schema_for;

use hbt_core::collection::{Collection, CollectionRepr};
use hbt_core::entity::Label;
use hbt_core::{InputFormat, OutputFormat};

use hbt::version;

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

    /// Output Collection JSON schema
    #[arg(long = "schema")]
    schema: bool,

    /// Read mappings from <FILE>
    #[arg(long = "mappings", value_name = "FILE")]
    mappings: Option<PathBuf>,

    /// Input file
    file: Option<PathBuf>,
}

/// Runs `f` against the output file if one was given, and stdout otherwise.
///
/// The buffer is flushed explicitly rather than left to the drop: a `BufWriter` dropped with
/// buffered data discards the write error, so a failure to reach disk would look like success.
fn with_writer<T>(
    output: Option<&PathBuf>,
    f: impl FnOnce(&mut dyn Write) -> Result<T, Error>,
) -> Result<T, Error> {
    if let Some(path) = output {
        let file = File::create(path)
            .with_context(|| format!("Could not create output file: {}", path.display()))?;
        let mut writer = BufWriter::new(file);
        let value = f(&mut writer)?;
        writer
            .flush()
            .with_context(|| format!("Could not write to {}", path.display()))?;
        Ok(value)
    } else {
        let stdout = io::stdout();
        let mut writer = BufWriter::new(stdout.lock());
        let value = f(&mut writer)?;
        writer.flush()?;
        Ok(value)
    }
}

fn update(args: &Args, coll: &mut Collection) -> Result<(), Error> {
    let Some(path) = &args.mappings else {
        return Ok(());
    };

    let contents = fs::read_to_string(path)
        .with_context(|| format!("Could not read mappings file: {}", path.display()))?;
    let yaml: serde_norway::Value = serde_norway::from_str(&contents)
        .with_context(|| format!("Could not parse mappings file: {}", path.display()))?;

    // An entry that is not a string pair used to be dropped silently, so a typo in the mappings
    // file left the labels it was meant to rewrite untouched with no indication why.
    let mappings = yaml
        .as_mapping()
        .ok_or_else(|| Error::msg("Mapping file must contain a YAML mapping"))?
        .iter()
        .map(|(k, v)| {
            let key = k
                .as_str()
                .ok_or_else(|| Error::msg("Mapping file keys must be strings"))?;
            let value = v
                .as_str()
                .with_context(|| format!("Mapping for {key:?} must be a string"))?;
            Ok((key.to_string(), value.to_string()))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    coll.update_labels(mappings);

    Ok(())
}

fn print(args: &Args, coll: &Collection) -> Result<(), Error> {
    if args.info {
        let length = coll.len();
        let file_name = args
            .file
            .as_ref()
            .map_or("input".into(), |f| f.to_string_lossy());
        let output = format!("{file_name}: {length} entities\n");
        return with_writer(None, |writer| Ok(writer.write_all(output.as_bytes())?));
    }

    if args.list_tags {
        let mut all_tags = BTreeSet::new();
        for entity in coll.entities() {
            all_tags.extend(entity.labels());
        }
        let tags_output = all_tags
            .into_iter()
            .map(Label::as_str)
            .collect::<Vec<_>>()
            .join("\n");
        let output = if tags_output.is_empty() {
            String::new()
        } else {
            format!("{tags_output}\n")
        };
        return with_writer(None, |writer| Ok(writer.write_all(output.as_bytes())?));
    }

    let format = match args.to {
        Some(format) => Some(format),
        None => args.output.as_ref().and_then(OutputFormat::detect),
    };

    if let Some(format) = format {
        return with_writer(args.output.as_ref(), |writer| {
            Ok(format.unparse(&mut &mut *writer, coll)?)
        });
    }

    Err(Error::msg(
        "Must specify an output format (-t) or analysis flag (--info, --list-tags)",
    ))
}

fn main() -> Result<ExitCode, Error> {
    let args = Args::parse();

    if args.schema {
        let schema = schema_for!(CollectionRepr);
        with_writer(args.output.as_ref(), |writer| {
            Ok(serde_json::to_writer_pretty(writer, &schema)?)
        })?;
        return Ok(ExitCode::SUCCESS);
    }

    let file = args
        .file
        .as_ref()
        .ok_or_else(|| Error::msg("Input file required"))?;

    let input_format = if let Some(format) = args.from {
        format
    } else {
        let no_parser = || Error::msg(format!("No parser for file: {}", file.display()));
        InputFormat::detect(file).ok_or_else(no_parser)?
    };

    let f = File::open(file)
        .with_context(|| format!("Could not open input file: {}", file.display()))?;
    let mut reader = BufReader::new(f);
    let mut coll = input_format
        .parse(&mut reader)
        .with_context(|| format!("Could not parse {}", file.display()))?;
    update(&args, &mut coll)?;
    print(&args, &coll)?;

    Ok(ExitCode::SUCCESS)
}
