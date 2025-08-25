use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

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

struct Env {
    manifest_dir_path: PathBuf,
    test_data_path: PathBuf,
}

fn find_input_file<P: AsRef<Path>>(path: P, dir_path: &str) -> Option<PathBuf> {
    const INPUT_EXTENSIONS: [&str; 4] = ["html", "md", "json", "xml"];

    let file_stem = path.as_ref().file_stem()?;
    let dir = Path::new(dir_path);

    INPUT_EXTENSIONS
        .iter()
        .map(|ext| dir.join(file_stem).with_extension(ext))
        .find(|path| path.exists())
}

fn has_extension<P: AsRef<Path>>(path: P, expected_ext: &str) -> bool {
    path.as_ref().extension().is_some_and(|ext| ext == expected_ext)
}

fn write_test_macro<P: AsRef<Path>, Q: AsRef<Path>>(
    f: &mut fs::File,
    macro_name: &str,
    test_name: &str,
    relative_input: P,
    absolute_output: Q,
) -> Result<(), io::Error> {
    writeln!(
        f,
        r#"{}!({}, "{}", "{}");"#,
        macro_name,
        test_name,
        relative_input.as_ref().display(),
        absolute_output.as_ref().display()
    )
}

fn generate_tests_for_dir<P: AsRef<Path>>(
    f: &mut fs::File,
    manifest_dir_path: P,
    dir_path: &str,
    category: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    const YAML_EXT: &str = "yaml";

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
            .and_then(OsStr::to_str)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file stem"))?;

        if let Some(input_path) = find_input_file(&file_path, dir_path) {
            let test_name = format!("{}_{}", category, file_stem);
            let output_path = manifest_dir_path.as_ref().join(file_path);
            write_test_macro(f, "cli_to_yaml_test", &test_name, &input_path, &output_path)?;
        }
    }

    writeln!(f)?;
    Ok(())
}

fn generate_html_export_tests<P: AsRef<Path>>(
    f: &mut fs::File,
    manifest_dir_path: P,
    input_dir: &str,
    export_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    const HTML_EXT: &str = "html";
    const EXPORT_SUFFIX: &str = "_export";

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
            .and_then(OsStr::to_str)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid file stem"))?;

        if let Some(original_name) = file_stem.strip_suffix(EXPORT_SUFFIX) {
            let test_name = format!("html_export_{}", original_name);
            let input_path = Path::new(input_dir).join(original_name).with_extension(HTML_EXT);
            if !input_path.exists() {
                continue;
            }
            let output_path = manifest_dir_path.as_ref().join(export_file_path);
            write_test_macro(f, "cli_to_html_test", &test_name, &input_path, &output_path)?;
        }
    }

    writeln!(f)?;
    Ok(())
}

fn generate_tests(env: Env) -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut f = fs::File::create(&dest_path)?;

    writeln!(f, "// Auto-generated test cases")?;
    writeln!(f)?;

    generate_tests_for_dir(&mut f, &env.manifest_dir_path, "tests/data/html", "html")?;
    generate_tests_for_dir(&mut f, &env.manifest_dir_path, "tests/data/markdown", "markdown")?;
    generate_tests_for_dir(&mut f, &env.manifest_dir_path, "tests/data/pinboard", "pinboard")?;

    generate_html_export_tests(
        &mut f,
        &env.manifest_dir_path,
        "tests/data/html",
        "tests/data/html/export",
    )?;

    println!("cargo:rerun-if-changed=tests/data");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let manifest_dir_path = PathBuf::from(&manifest_dir);
    println!("cargo::warning={}", manifest_dir_path.to_string_lossy());

    let cargo_workspace_dir_path = manifest_dir_path
        .parent()
        .ok_or(io::Error::new(io::ErrorKind::NotADirectory, "Failed to find workspace dir"))?;

    if cargo_workspace_dir_path.join(".git").exists() {
        commit_info_git();
    } else {
        commit_info_env();
    }

    let test_data_path = {
        let mut tmp = manifest_dir_path.clone();
        tmp.push("test");
        tmp.push("data");
        tmp
    };

    let env = Env { manifest_dir_path, test_data_path };

    generate_tests(env)?;

    Ok(())
}
