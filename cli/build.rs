use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::Path;
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

struct Env<'a> {
    manifest_dir_path: &'a Path,
    test_data_path: &'a Path,
}

fn find_input_extension(file_name: &str) -> Option<&str> {
    const INPUT_EXTENSIONS: &[&str] = &[".input.html", ".input.md", ".input.json", ".input.xml"];
    INPUT_EXTENSIONS.iter().find_map(|ext| file_name.strip_suffix(ext))
}

fn find_output_extension(file_name: &str) -> Option<(&str, &str)> {
    const OUTPUT_EXTENSIONS: &[(&str, &str)] =
        &[(".expected.yaml", "yaml"), (".expected.html", "html")];
    OUTPUT_EXTENSIONS
        .iter()
        .find_map(|(ext, format)| file_name.strip_suffix(ext).map(|stem| (stem, *format)))
}

fn generate_test_name(category: &str, stem: &str, format: &str) -> String {
    match format {
        "html" => format!("html_export_{}", stem),
        _ => format!("{}_{}", category, stem),
    }
}

fn write_test_macro<P: AsRef<Path>, Q: AsRef<Path>>(
    f: &mut fs::File,
    test_name: &str,
    format: &str,
    relative_input: P,
    absolute_output: Q,
) -> Result<(), io::Error> {
    writeln!(
        f,
        r#"cli_test!({}, "{}", "{}", "{}");"#,
        test_name,
        format,
        relative_input.as_ref().display(),
        absolute_output.as_ref().display()
    )
}

fn load_known_issues() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let issues_file = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("issues");
    let contents = fs::read_to_string(&issues_file)?;

    let issues: Vec<String> = contents
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect();

    Ok(issues)
}

fn is_problematic_test(category: &str, stem: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let known_issues = load_known_issues()?;
    let test_patterns = [
        format!("{}/{}.input.html", category, stem),
        format!("{}/{}.input.json", category, stem),
        format!("{}/{}.input.xml", category, stem),
        format!("{}/{}.input.md", category, stem),
    ];

    Ok(test_patterns.iter().any(|pattern| known_issues.contains(pattern)))
}

fn generate_tests_for_dir(
    f: &mut fs::File,
    env: &Env,
    category: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = env.test_data_path.join(category);

    if !path.exists() {
        return Ok(());
    }

    let include_failing_tests = env::var("HBT_INCLUDE_FAILING_TESTS").is_ok();

    // Single-pass directory processing
    let mut input_files = std::collections::HashMap::new();
    let mut output_tests = Vec::new();

    for entry in fs::read_dir(&path)? {
        let entry = entry?;
        let file_path = entry.path();
        let file_name = match file_path.file_name().and_then(OsStr::to_str) {
            Some(name) => name,
            None => continue,
        };

        // Check for input files
        if let Some(stem) = find_input_extension(file_name) {
            input_files.insert(stem.to_string(), file_path);
        }
        // Check for output files
        else if let Some((stem, format)) = find_output_extension(file_name) {
            output_tests.push((stem.to_string(), format.to_string(), file_path));
        }
    }

    // Generate test macros for matched input/output pairs
    for (stem, format, output_path) in output_tests {
        if let Some(input_path) = input_files.get(&stem) {
            // Skip problematic tests unless explicitly enabled
            if !include_failing_tests && is_problematic_test(category, &stem)? {
                continue;
            }

            let test_name = generate_test_name(category, &stem, &format);
            let absolute_output_path = env.manifest_dir_path.join(&output_path);
            write_test_macro(f, &test_name, &format, input_path, &absolute_output_path)?;
        }
    }

    writeln!(f)?;
    Ok(())
}

fn generate_tests(env: &Env) -> Result<(), Box<dyn std::error::Error>> {
    const TEST_CATEGORIES: &[&str] = &["html", "markdown", "pinboard"];

    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("generated_tests.rs");
    let mut f = fs::File::create(&dest_path)?;

    writeln!(f, "// Auto-generated test cases")?;
    writeln!(f)?;

    for category in TEST_CATEGORIES {
        generate_tests_for_dir(&mut f, env, category)?;
    }

    println!("cargo:rerun-if-changed=tests/data");
    println!("cargo:rerun-if-changed=issues");
    println!("cargo:rerun-if-env-changed=HBT_INCLUDE_FAILING_TESTS");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let manifest_dir_path = Path::new(&manifest_dir);

    let cargo_workspace_dir_path = manifest_dir_path
        .parent()
        .ok_or(io::Error::new(io::ErrorKind::NotADirectory, "Failed to find workspace dir"))?;

    if cargo_workspace_dir_path.join(".git").exists() {
        commit_info_git();
    } else {
        commit_info_env();
    }

    let test_data_path = &manifest_dir_path.join("tests/data");
    let env = Env { manifest_dir_path, test_data_path };
    generate_tests(&env)?;

    Ok(())
}
