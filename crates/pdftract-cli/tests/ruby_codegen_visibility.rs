//! Regression coverage for the Ruby SDK client's effective method visibility.

use pdftract_cli::codegen::{CodeGenerator, Language};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Visibility {
    Public,
    Private,
}

struct CurrentDirGuard(PathBuf);

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.0);
    }
}

const PUBLIC_METHODS: &[&str] = &[
    "initialize",
    "extract",
    "extract_text",
    "extract_markdown",
    "extract_stream",
    "search",
    "get_metadata",
    "hash",
    "classify",
    "verify_receipt",
];

const PRIVATE_METHODS: &[&str] = &["exec", "map_error"];

fn method_name(line: &str) -> Option<&str> {
    let definition = line.trim().strip_prefix("def ")?;
    definition.split(['(', ' ', '\t']).next()
}

#[test]
fn ruby_codegen_contract_methods_are_public() {
    // CodeGenerator follows the CLI and resolves its template and contract
    // paths from the workspace root. Cargo runs this integration test from
    // the crate directory, so scope the required CWD change to this test.
    let original_dir = std::env::current_dir().expect("failed to read current directory");
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    std::env::set_current_dir(&workspace_root).expect("failed to enter workspace root");
    let _restore_dir = CurrentDirGuard(original_dir);

    let template_dir = PathBuf::from("templates/sdk-skeleton");
    let mut generator = CodeGenerator::new(&template_dir, "1.0.0".to_string())
        .expect("failed to construct SDK code generator");
    let output_dir = TempDir::new().expect("failed to create SDK output directory");

    generator
        .generate(Language::Ruby, output_dir.path())
        .expect("failed to render Ruby SDK");

    let methods_path = output_dir.path().join("lib/pdftract/codegen/methods.rb");
    let methods_source = std::fs::read_to_string(&methods_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", methods_path.display()));

    // Ruby visibility keywords apply to subsequent method definitions in the
    // same class body. Keep this deliberately small and source-oriented: the
    // test needs no Ruby interpreter or subprocess to catch a bare `private`
    // leaking into the generated contract methods.
    let mut visibility = Visibility::Public;
    let mut methods = HashMap::new();
    for (line_number, line) in methods_source.lines().enumerate() {
        match line.trim() {
            "public" => visibility = Visibility::Public,
            "private" => visibility = Visibility::Private,
            _ => {}
        }

        if let Some(name) = method_name(line) {
            assert!(
                methods
                    .insert(name, (visibility, line_number + 1))
                    .is_none(),
                "generated Ruby client defines {name} more than once"
            );
        }
    }

    for method in PUBLIC_METHODS {
        let (actual_visibility, line_number) = methods
            .get(method)
            .unwrap_or_else(|| panic!("generated Ruby client is missing {method}"));
        assert_eq!(
            *actual_visibility,
            Visibility::Public,
            "generated Ruby method {method} at line {line_number} is not public"
        );
    }

    // Default-deny any new generated method: only the two intentional
    // internal helpers may be non-public.
    for (method, (actual_visibility, line_number)) in &methods {
        if *actual_visibility == Visibility::Private {
            assert!(
                PRIVATE_METHODS.contains(method),
                "generated Ruby method {method} at line {line_number} is unexpectedly private"
            );
        }
    }
}
