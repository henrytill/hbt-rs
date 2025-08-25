use std::fs;
use std::io::{Error, ErrorKind, Result, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const INPUT_EXTENSIONS: &[&str] = &["html", "md", "json", "xml"];
const YAML_EXT: &str = "yaml";
const HTML_EXT: &str = "html";
const EXPORT_SUFFIX: &str = "_export";

fn commit_info_git() {
    let output = match Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--date=short")
        .arg("--format=%H %h %cd")
        .arg("--abbrev=7")
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return,
    };
    let stdout = String::from_utf8(output.stdout).unwrap();
    let mut parts = stdout.split_whitespace();
    let mut next = || parts.next().unwrap();
    println!("cargo:rustc-env=HBT_COMMIT_HASH={}", next());
    println!("cargo:rustc-env=HBT_COMMIT_SHORT_HASH={}", next());
    println!("cargo:rustc-env=HBT_COMMIT_DATE={}", next())
}

fn commit_info_env() {
    for var in ["HBT_COMMIT_HASH", "HBT_COMMIT_SHORT_HASH", "HBT_COMMIT_DATE"] {
        if let Ok(value) = std::env::var(var) {
            println!("cargo:rustc-env={}={}", var, value);
        }
    }
}

fn find_input_file(yaml_path: &Path, dir_path: &str) -> Option<PathBuf> {
    let file_stem = yaml_path.file_stem()?;
    let dir = Path::new(dir_path);

    INPUT_EXTENSIONS
        .iter()
        .map(|ext| dir.join(file_stem).with_extension(ext))
        .find(|path| path.exists())
}

fn has_extension(path: &Path, expected_ext: &str) -> bool {
    path.extension().is_some_and(|ext| ext == expected_ext)
}

fn write_test_macro(
    f: &mut fs::File,
    macro_name: &str,
    test_name: &str,
    relative_input: &Path,
    absolute_output: &Path,
) -> Result<()> {
    writeln!(
        f,
        r#"{}!({}, "{}", "{}");"#,
        macro_name,
        test_name,
        relative_input.display(),
        absolute_output.display()
    )
}

fn generate_tests_for_dir(f: &mut fs::File, dir_path: &str, category: &str) -> Result<()> {
    let path = Path::new(dir_path);

    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        if !has_extension(&file_path, YAML_EXT) {
            continue;
        }

        let file_stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid file stem"))?;

        if let Some(input_path) = find_input_file(&file_path, dir_path) {
            let test_name = format!("{}_{}", category, file_stem);
            let relative_input = input_path
                .strip_prefix("tests/")
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid path prefix"))?;
            let absolute_output = std::fs::canonicalize(&file_path)?;

            write_test_macro(f, "cli_to_yaml_test", &test_name, &relative_input, &absolute_output)?;
        }
    }

    writeln!(f)?;
    Ok(())
}

fn generate_html_export_tests(f: &mut fs::File, input_dir: &str, export_dir: &str) -> Result<()> {
    let export_path = Path::new(export_dir);

    if !export_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(export_path)? {
        let entry = entry?;
        let export_file_path = entry.path();

        if !has_extension(&export_file_path, HTML_EXT) {
            continue;
        }

        let file_stem = export_file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "Invalid file stem"))?;

        if let Some(original_name) = file_stem.strip_suffix(EXPORT_SUFFIX) {
            let input_file = Path::new(input_dir).join(original_name).with_extension(HTML_EXT);

            if input_file.exists() {
                let test_name = format!("html_export_{}", original_name);
                let relative_input = input_file
                    .strip_prefix("tests/")
                    .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid path prefix"))?;
                let absolute_export = std::fs::canonicalize(&export_file_path)?;

                write_test_macro(
                    f,
                    "cli_to_html_test",
                    &test_name,
                    &relative_input,
                    &absolute_export,
                )?;
            }
        }
    }

    writeln!(f)?;
    Ok(())
}

fn generate_tests() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    writeln!(f, "// Auto-generated test cases").unwrap();
    writeln!(f).unwrap();

    generate_tests_for_dir(&mut f, "tests/data/html", "html").unwrap();
    generate_tests_for_dir(&mut f, "tests/data/markdown", "markdown").unwrap();
    generate_tests_for_dir(&mut f, "tests/data/pinboard", "pinboard").unwrap();

    generate_html_export_tests(&mut f, "tests/data/html", "tests/data/html/export").unwrap();

    println!("cargo:rerun-if-changed=tests/data");
}

fn main() {
    if Path::new("../.git").exists() {
        commit_info_git();
    } else {
        commit_info_env();
    }

    generate_tests();
}
