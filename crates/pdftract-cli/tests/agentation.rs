//! Agentation wiring contract for every HTML entry point in the workspace.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn workspace_html_files() -> Vec<PathBuf> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    WalkDir::new(workspace)
        .into_iter()
        .filter_entry(|entry| {
            !entry.file_type().is_dir()
                || entry.file_name().to_str().map_or(false, |name| {
                    !name.starts_with('.') && name != "node_modules" && name != "target"
                })
        })
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "html" | "htm"))
        })
        .collect()
}

#[test]
fn every_html_entry_point_declares_agentation_before_loading_it() {
    let html_files = workspace_html_files();
    assert!(
        !html_files.is_empty(),
        "the workspace should contain HTML entries"
    );

    for path in html_files {
        let html = fs::read_to_string(&path).expect("HTML entry point should be readable");
        let import_map_start = html
            .find(r#"<script type="importmap">"#)
            .unwrap_or_else(|| panic!("{} is missing an import map", path.display()));
        let import_map_end = html[import_map_start..]
            .find("</script>")
            .map(|offset| import_map_start + offset)
            .expect("the import map should be closed");
        let import_map =
            &html[import_map_start + r#"<script type="importmap">"#.len()..import_map_end];
        let import_map: Value = serde_json::from_str(import_map.trim()).unwrap_or_else(|error| {
            panic!("{} has invalid import map JSON: {error}", path.display())
        });
        let imports = import_map
            .get("imports")
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("{} import map has no imports", path.display()));

        for dependency in [
            "react",
            "react-dom",
            "react-dom/client",
            "react/jsx-runtime",
            "agentation",
        ] {
            assert!(
                imports.contains_key(dependency),
                "{} import map is missing {dependency}",
                path.display()
            );
        }
        assert_eq!(
            imports["agentation"],
            "https://esm.sh/agentation@3.0.2?external=react,react-dom",
            "{} must pin the Agentation module",
            path.display()
        );

        let agentation_script = html
            .find("agentation.js")
            .unwrap_or_else(|| panic!("{} does not load the Agentation bootstrap", path.display()));
        assert!(
            import_map_end < agentation_script,
            "{} must declare its import map before loading Agentation",
            path.display()
        );
    }
}
