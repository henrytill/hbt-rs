use proc_macro::TokenStream;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use proc_macro2::Span;
use quote::quote;
use syn::{
    self, Error, Ident, LitStr, Token,
    parse::{Parse, ParseStream},
};
use walkdir::WalkDir;

struct Args {
    path: LitStr,
    ext: LitStr,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![,]>()?;
        let ext: LitStr = input.parse()?;
        Ok(Args { path, ext })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct TestCase {
    stem: String,
    input_path: String,
    expected_path: String,
}

#[derive(Debug)]
struct TestCaseBuilder {
    stem: String,
    input_path: Option<PathBuf>,
    expected_path: Option<PathBuf>,
}

impl TestCaseBuilder {
    fn new(stem: String) -> Self {
        Self {
            stem,
            input_path: None,
            expected_path: None,
        }
    }

    fn set_input(&mut self, path: PathBuf) {
        self.input_path = Some(path);
    }

    fn set_expected(&mut self, path: PathBuf) {
        self.expected_path = Some(path);
    }

    fn is_complete(&self) -> bool {
        self.input_path.is_some() && self.expected_path.is_some()
    }

    fn build(self) -> Option<TestCase> {
        Some(TestCase {
            stem: self.stem,
            input_path: self.input_path?.to_str()?.to_string(),
            expected_path: self.expected_path?.to_str()?.to_string(),
        })
    }
}

fn split_filename(filename: &str) -> Vec<&str> {
    filename.split('.').collect()
}

fn discover_parser_tests(base_path: &Path, input_ext: &str) -> Result<Vec<TestCase>, String> {
    if !base_path.exists() {
        return Err(format!(
            "Test data directory does not exist: {}",
            base_path.display()
        ));
    }

    let mut builders: BTreeMap<String, TestCaseBuilder> = BTreeMap::new();

    for entry in WalkDir::new(base_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        let parts = split_filename(filename);

        match parts.as_slice() {
            [stem, "input", ext] if *ext == input_ext => {
                let builder = builders
                    .entry(stem.to_string())
                    .or_insert_with(|| TestCaseBuilder::new(stem.to_string()));
                builder.set_input(path.to_path_buf());
            }
            [stem, "expected", "yaml"] => {
                let builder = builders
                    .entry(stem.to_string())
                    .or_insert_with(|| TestCaseBuilder::new(stem.to_string()));
                builder.set_expected(path.to_path_buf());
            }
            _ => {}
        }
    }

    let mut test_cases: Vec<TestCase> = builders
        .into_values()
        .filter(TestCaseBuilder::is_complete)
        .filter_map(TestCaseBuilder::build)
        .collect();

    test_cases.sort();
    Ok(test_cases)
}

fn discover_formatter_tests(base_path: &Path, output_ext: &str) -> Result<Vec<TestCase>, String> {
    if !base_path.exists() {
        return Err(format!(
            "Test data directory does not exist: {}",
            base_path.display()
        ));
    }

    let mut builders: BTreeMap<String, TestCaseBuilder> = BTreeMap::new();

    for entry in WalkDir::new(base_path)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        let parts = split_filename(filename);

        match parts.as_slice() {
            [stem, "input", _] => {
                let builder = builders
                    .entry(stem.to_string())
                    .or_insert_with(|| TestCaseBuilder::new(stem.to_string()));
                builder.set_input(path.to_path_buf());
            }
            [stem, "expected", ext] if *ext == output_ext => {
                let builder = builders
                    .entry(stem.to_string())
                    .or_insert_with(|| TestCaseBuilder::new(stem.to_string()));
                builder.set_expected(path.to_path_buf());
            }
            _ => {}
        }
    }

    let mut test_cases: Vec<TestCase> = builders
        .into_values()
        .filter(TestCaseBuilder::is_complete)
        .filter_map(TestCaseBuilder::build)
        .collect();

    test_cases.sort();
    Ok(test_cases)
}

fn resolve_path(rel_path: &str) -> Result<PathBuf, String> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map_err(|_| "CARGO_MANIFEST_DIR environment variable not set".to_string())?;

    let base_path = Path::new(&manifest_dir)
        .parent()
        .ok_or_else(|| format!("Could not get parent directory of {}", manifest_dir))?;

    let full_path = base_path.join(rel_path);

    if !full_path.exists() {
        return Err(format!(
            "Test data directory does not exist: {}",
            full_path.display()
        ));
    }

    Ok(full_path)
}

#[proc_macro]
pub fn test_parser(input: TokenStream) -> TokenStream {
    let args: Args = syn::parse_macro_input!(input);

    let base_path = match resolve_path(&args.path.value()) {
        Ok(path) => path,
        Err(err) => {
            let error = Error::new(args.path.span(), err);
            return error.to_compile_error().into();
        }
    };

    let test_cases = match discover_parser_tests(&base_path, &args.ext.value()) {
        Ok(cases) => cases,
        Err(err) => {
            let error = Error::new(args.path.span(), err);
            return error.to_compile_error().into();
        }
    };

    let tests = test_cases.iter().map(|tc| {
        let test_ident = Ident::new(&format!("test_{}", tc.stem), Span::call_site());
        let input_path = &tc.input_path;
        let expected_path = &tc.expected_path;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_parser_input(#input_path, #expected_path)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::File;
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT};

        fn test_parser_input(input_path: &str, expected_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let input_format = Format::<INPUT>::detect(input_path)
                .ok_or_else(|| format!("Could not detect format for: {}", input_path))?;

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
pub fn test_formatter(input: TokenStream) -> TokenStream {
    let args: Args = syn::parse_macro_input!(input);

    let base_path = match resolve_path(&args.path.value()) {
        Ok(path) => path,
        Err(err) => {
            let error = Error::new(args.path.span(), err);
            return error.to_compile_error().into();
        }
    };

    let test_cases = match discover_formatter_tests(&base_path, &args.ext.value()) {
        Ok(cases) => cases,
        Err(err) => {
            let error = Error::new(args.path.span(), err);
            return error.to_compile_error().into();
        }
    };

    let tests = test_cases.iter().map(|tc| {
        let test_ident = Ident::new(&format!("test_{}", tc.stem), Span::call_site());
        let input_path = &tc.input_path;
        let expected_path = &tc.expected_path;

        quote! {
            #[test]
            fn #test_ident() -> Result<(), Box<dyn std::error::Error>> {
                test_formatter_output(#input_path, #expected_path)?;
                Ok(())
            }
        }
    });

    let expanded = quote! {
        use std::io::BufReader;
        use std::fs::{File, read_to_string};
        use hbt_core::collection::Collection;
        use hbt_core::format::{Format, INPUT, OUTPUT};

        fn test_formatter_output(input_path: &str, expected_path: &str) -> Result<(), Box<dyn std::error::Error>> {
            let input_format = Format::<INPUT>::detect(input_path)
                .ok_or_else(|| format!("Could not detect format for: {}", input_path))?;

            let input_file = File::open(input_path)?;
            let mut input_reader = BufReader::new(input_file);
            let collection = input_format.parse(&mut input_reader)?;

            let mut output = Vec::new();
            let html_format = Format::<OUTPUT>::detect("output.html")
                .ok_or("Could not create HTML format")?;
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
