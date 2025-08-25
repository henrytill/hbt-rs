use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn generate_tests() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut f = fs::File::create(&dest_path).unwrap();

    writeln!(f, "// Auto-generated test cases").unwrap();
    writeln!(f).unwrap();

    // Generate tests for each directory
    generate_tests_for_dir(&mut f, "tests/data/html", "html").unwrap();
    generate_tests_for_dir(&mut f, "tests/data/markdown", "markdown").unwrap();
    generate_tests_for_dir(&mut f, "tests/data/pinboard", "pinboard").unwrap();

    println!("cargo:rerun-if-changed=tests/data");
}

fn generate_tests_for_dir(f: &mut fs::File, dir_path: &str, category: &str) -> std::io::Result<()> {
    let path = Path::new(dir_path);

    if !path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_path = entry.path();

        // Skip if it's not a .yaml file (these are our expected outputs)
        if file_path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }

        let file_stem = file_path.file_stem().unwrap().to_str().unwrap();

        // Find the corresponding input file
        if let Some(input_path) = find_input_file(&file_path, dir_path) {
            let test_name = format!("{}_{}", category, file_stem);
            let relative_input = input_path.strip_prefix("tests/").unwrap();
            let absolute_output = std::fs::canonicalize(&file_path).unwrap();

            writeln!(
                f,
                r#"cli_to_yaml_test!({}, "{}", "{}");"#,
                test_name,
                relative_input.display().to_string().replace('\\', "/"),
                absolute_output.display().to_string().replace('\\', "/")
            )?;
        }
    }

    writeln!(f)?;
    Ok(())
}

fn find_input_file(yaml_path: &Path, dir_path: &str) -> Option<PathBuf> {
    let file_stem = yaml_path.file_stem()?.to_str()?;
    let dir = Path::new(dir_path);

    // Common extensions to check for input files
    let extensions = ["html", "md", "json", "xml"];

    for ext in &extensions {
        let input_path = dir.join(format!("{}.{}", file_stem, ext));
        if input_path.exists() {
            return Some(input_path);
        }
    }

    None
}

fn main() {
    if Path::new("../.git").exists() {
        commit_info_git();
    } else {
        commit_info_env();
    }

    generate_tests();
}
