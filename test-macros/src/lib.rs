use proc_macro::TokenStream;
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::Span;
use quote::quote;
use syn::{self, Error, Ident, LitStr};
use walkdir::WalkDir;

const INPUT_EXTENSIONS: &[&str] = &[".input.html", ".input.json", ".input.xml", ".input.md"];
const EXPECTED_YAML_EXT: &str = ".expected.yaml";
const EXPECTED_HTML_EXT: &str = ".expected.html";

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct TestCase {
    category: String,
    stem: String,
    input_path: String,
    expected_path: String,
}

impl TestCase {
    fn new(
        category: String,
        stem: String,
        input_path: PathBuf,
        expected_path: PathBuf,
    ) -> Option<Self> {
        if !input_path.exists() || !expected_path.exists() {
            return None;
        }
        Some(Self {
            category,
            stem,
            input_path: input_path.to_str()?.to_string(),
            expected_path: expected_path.to_str()?.to_string(),
        })
    }
}

fn resolve_base_path(rel_path: &str) -> Result<PathBuf, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR environment variable not set".to_string())?;

    let base_path = Path::new(&manifest_dir).join(rel_path);

    if !base_path.exists() {
        return Err(format!(
            "Test data directory does not exist: {}",
            base_path.display()
        ));
    }

    Ok(base_path)
}

fn extract_category(path: &Path) -> Option<&str> {
    path.parent()
        .and_then(Path::file_name)
        .and_then(OsStr::to_str)
}

fn walk(root: &Path) -> impl Iterator<Item = (PathBuf, String)> {
    WalkDir::new(root)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| path.is_file())
        .filter_map(|path| {
            let file_name = path.file_name()?.to_str()?.to_string();
            Some((path, file_name))
        })
}

#[proc_macro]
pub fn test_parsers(input: TokenStream) -> TokenStream {
    let path_lit = syn::parse_macro_input!(input as LitStr);
    let base_path = match resolve_base_path(&path_lit.value()) {
        Ok(path) => path,
        Err(err) => {
            let error = Error::new(path_lit.span(), err);
            return error.to_compile_error().into();
        }
    };

    let mut test_cases = BTreeSet::new();

    for (path, file_name) in walk(&base_path) {
        if let Some(stem) = INPUT_EXTENSIONS
            .iter()
            .find_map(|ext| file_name.strip_suffix(ext))
        {
            let Some(category) = extract_category(&path) else {
                continue;
            };

            let expected_path = path.with_file_name(format!("{}{}", stem, EXPECTED_YAML_EXT));
            if let Some(test_case) =
                TestCase::new(category.to_string(), stem.to_string(), path, expected_path)
            {
                test_cases.insert(test_case);
            }
        }
    }

    let tests = test_cases.iter().map(|tc| {
        let test_ident = Ident::new(
            &format!("parse_{}_{}", tc.category, tc.stem),
            Span::call_site(),
        );
        let input_path = &tc.input_path;
        let expected_path = &tc.expected_path;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_parser(#input_path, #expected_path)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::File;
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT};

        fn test_parser(input_path: &str, expected_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let input_format = Format::<INPUT>::detect(input_path).ok_or("Could not detect format")?;

            let input_file = File::open(input_path)?;
            let mut input_reader = BufReader::new(input_file);
            let parsed_collection = input_format.parse(&mut input_reader)?;

            let expected_file = File::open(expected_path)?;
            let expected_reader = BufReader::new(expected_file);
            let expected_collection: Collection = serde_norway::from_reader(expected_reader)?;

            assert_eq!(
                parsed_collection,
                expected_collection,
                "Collection mismatch for input: {}\nExpected from: {}",
                input_path,
                expected_path
            );

            Ok(())
        }

        #(#tests)*
    };

    TokenStream::from(expanded)
}

#[proc_macro]
pub fn test_formatters(input: TokenStream) -> TokenStream {
    let path_lit = syn::parse_macro_input!(input as LitStr);
    let base_path = match resolve_base_path(&path_lit.value()) {
        Ok(path) => path,
        Err(err) => {
            let error = Error::new(path_lit.span(), err);
            return error.to_compile_error().into();
        }
    };

    let mut test_cases = BTreeSet::new();

    for (path, file_name) in walk(&base_path) {
        if let Some(stem) = file_name.strip_suffix(EXPECTED_HTML_EXT) {
            let Some(category) = extract_category(&path) else {
                continue;
            };

            for ext in INPUT_EXTENSIONS {
                let input_path = path.with_file_name(format!("{}{}", stem, ext));
                if let Some(test_case) = TestCase::new(
                    category.to_string(),
                    stem.to_string(),
                    input_path,
                    path.clone(),
                ) {
                    test_cases.insert(test_case);
                    break;
                }
            }
        }
    }

    let tests = test_cases.iter().map(|tc| {
        let test_ident = Ident::new(
            &format!("format_{}_{}", tc.category, tc.stem),
            Span::call_site(),
        );
        let input_path = &tc.input_path;
        let expected_path = &tc.expected_path;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_formatter(#input_path, #expected_path)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::{File, read_to_string};
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT, OUTPUT};

        fn test_formatter(input_path: &str, expected_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let input_format = Format::<INPUT>::detect(input_path).ok_or("Could not detect format")?;

            let input_file = File::open(input_path)?;
            let mut input_reader = BufReader::new(input_file);
            let collection = input_format.parse(&mut input_reader)?;

            let mut output = Vec::new();
            let html_format = Format::<OUTPUT>::detect("output.html").ok_or("Could not create HTML format")?;
            html_format.unparse(&mut output, &collection)?;
            let actual_html = String::from_utf8(output)?;

            let expected_html = read_to_string(expected_path)?;

            assert_eq!(
                actual_html.trim(),
                expected_html.trim(),
                "HTML output mismatch for input: {}\nExpected from: {}",
                input_path,
                expected_path
            );

            Ok(())
        }

        #(#tests)*
    };

    TokenStream::from(expanded)
}
