/// Tests for UnmappedGlyphNamesConfig default value handling.
///
/// This test verifies that unmapped_glyph_names defaults to an empty list
/// when not specified in the config file, and works correctly when specified.

use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Copy of the UnmappedGlyphNamesConfig struct from build.rs for testing purposes.
#[derive(Debug, serde::Deserialize)]
struct UnmappedGlyphNamesConfig {
    /// List of glyph names to skip during CMAP and ToUnicode entry creation.
    #[serde(default)]
    unmapped_glyph_names: Vec<String>,

    /// Optional description of the configuration purpose.
    #[serde(default)]
    description: Option<String>,

    /// Configuration format version identifier.
    #[serde(default)]
    version: Option<String>,
}

#[test]
fn test_unmapped_glyph_names_defaults_to_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file WITHOUT unmapped_glyph_names field
    let config_content = r#"{
      "description": "Test config without unmapped_glyph_names",
      "version": "1.0"
    }"#;

    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    // Parse the config
    let json_content = fs::read_to_string(&config_path).unwrap();
    let config: UnmappedGlyphNamesConfig = serde_json::from_str(&json_content).unwrap();

    // Verify that unmapped_glyph_names defaults to an empty list
    assert!(config.unmapped_glyph_names.is_empty());
    assert_eq!(config.unmapped_glyph_names.len(), 0);

    // Verify other fields are parsed correctly
    assert_eq!(config.description, Some("Test config without unmapped_glyph_names".to_string()));
    assert_eq!(config.version, Some("1.0".to_string()));
}

#[test]
fn test_unmapped_glyph_names_specified() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file WITH unmapped_glyph_names field
    let config_content = r#"{
      "unmapped_glyph_names": [".notdef", ".null", "g000"],
      "description": "Test config with unmapped_glyph_names",
      "version": "1.0"
    }"#;

    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    // Parse the config
    let json_content = fs::read_to_string(&config_path).unwrap();
    let config: UnmappedGlyphNamesConfig = serde_json::from_str(&json_content).unwrap();

    // Verify that unmapped_glyph_names is parsed correctly
    assert_eq!(config.unmapped_glyph_names.len(), 3);
    assert_eq!(config.unmapped_glyph_names, vec![".notdef", ".null", "g000"]);

    // Verify other fields are parsed correctly
    assert_eq!(config.description, Some("Test config with unmapped_glyph_names".to_string()));
    assert_eq!(config.version, Some("1.0".to_string()));
}

#[test]
fn test_unmapped_glyph_names_empty_array() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config with explicit empty array
    let config_content = r#"{
      "unmapped_glyph_names": [],
      "description": "Test config with empty array"
    }"#;

    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    // Parse the config
    let json_content = fs::read_to_string(&config_path).unwrap();
    let config: UnmappedGlyphNamesConfig = serde_json::from_str(&json_content).unwrap();

    // Verify that unmapped_glyph_names is an empty list
    assert!(config.unmapped_glyph_names.is_empty());
    assert_eq!(config.unmapped_glyph_names.len(), 0);
}

#[test]
fn test_unmapped_glyph_names_minimal_config() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a completely empty config (all fields should default)
    let config_content = r#"{}"#;

    let mut file = fs::File::create(&config_path).unwrap();
    file.write_all(config_content.as_bytes()).unwrap();

    // Parse the config
    let json_content = fs::read_to_string(&config_path).unwrap();
    let config: UnmappedGlyphNamesConfig = serde_json::from_str(&json_content).unwrap();

    // Verify all fields default correctly
    assert!(config.unmapped_glyph_names.is_empty());
    assert!(config.description.is_none());
    assert!(config.version.is_none());
}
