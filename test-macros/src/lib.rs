use proc_macro::TokenStream;
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::Span;
use quote::quote;
use syn::{self, Ident, LitStr};
use walkdir::WalkDir;

struct TestCase {
    category: String,
    stem: String,
    format: String,
    input_path: PathBuf,
    expected_path: PathBuf,
}

#[proc_macro]
pub fn test_parsers(input: TokenStream) -> TokenStream {
    let path_lit = syn::parse_macro_input!(input as LitStr);
    let rel_path = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let base_path = Path::new(&manifest_dir).join(&rel_path);

    if !base_path.exists() {
        panic!(
            "Test data directory does not exist: {}",
            base_path.display()
        );
    }

    let mut test_cases = Vec::new();

    for entry in WalkDir::new(&base_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(OsStr::to_str) {
            Some(name) => name,
            None => continue,
        };

        let input_extensions = [".input.html", ".input.json", ".input.xml", ".input.md"];
        let stem_and_ext = input_extensions.iter().find_map(|ext| {
            file_name.strip_suffix(ext).map(|stem| {
                let format_ext = ext.trim_start_matches(".input.");
                (stem, format_ext)
            })
        });

        if let Some((stem, format_ext)) = stem_and_ext {
            let category = path
                .parent()
                .and_then(Path::file_name)
                .and_then(OsStr::to_str)
                .expect("Could not determine test category");

            let expected_path = path.with_file_name(format!("{}.expected.yaml", stem));
            if expected_path.exists() {
                test_cases.push(TestCase {
                    category: category.to_string(),
                    stem: stem.to_string(),
                    format: format_ext.to_string(),
                    input_path: path.to_path_buf(),
                    expected_path,
                });
            }
        }
    }

    test_cases.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.stem.cmp(&b.stem))
            .then(a.format.cmp(&b.format))
    });

    let tests = test_cases.iter().map(|tc| {
        let test_name = format!("parse_{}_{}", tc.category, tc.stem);
        let test_ident = Ident::new(&test_name, Span::call_site());

        let input_path = tc.input_path.to_str().expect("Invalid UTF-8 in path");
        let expected_path = tc.expected_path.to_str().expect("Invalid UTF-8 in path");
        let format = &tc.format;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_parser(#input_path, #expected_path, #format)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::File;
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT};

        fn test_parser(input_path: &str, expected_path: &str, format_str: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    let rel_path = path_lit.value();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let base_path = Path::new(&manifest_dir).join(&rel_path);

    if !base_path.exists() {
        panic!(
            "Test data directory does not exist: {}",
            base_path.display()
        );
    }

    let mut test_cases = Vec::new();

    for entry in WalkDir::new(&base_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(OsStr::to_str) {
            Some(name) => name,
            None => continue,
        };

        if let Some(stem) = file_name.strip_suffix(".expected.html") {
            let category = path
                .parent()
                .and_then(Path::file_name)
                .and_then(OsStr::to_str)
                .expect("Could not determine test category");

            let input_extensions = [".input.html", ".input.json", ".input.xml", ".input.md"];
            for ext in input_extensions {
                let input_path = path.with_file_name(format!("{}{}", stem, ext));
                if input_path.exists() {
                    let format_ext = ext.trim_start_matches(".input.");
                    test_cases.push(TestCase {
                        category: category.to_string(),
                        stem: stem.to_string(),
                        format: format_ext.to_string(),
                        input_path,
                        expected_path: path.to_path_buf(),
                    });
                    break;
                }
            }
        }
    }

    test_cases.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then(a.stem.cmp(&b.stem))
            .then(a.format.cmp(&b.format))
    });

    let tests = test_cases.iter().map(|tc| {
        let test_name = format!("format_{}_{}", tc.category, tc.stem);
        let test_ident = Ident::new(&test_name, Span::call_site());

        let input_path = tc.input_path.to_str().expect("Invalid UTF-8 in path");
        let expected_path = tc.expected_path.to_str().expect("Invalid UTF-8 in path");
        let format = &tc.format;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_formatter(#input_path, #expected_path, #format)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::{File, read_to_string};
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT, OUTPUT};

        fn test_formatter(input_path: &str, expected_path: &str, format_str: &str) -> Result<(), Box<dyn std::error::Error>> {
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
