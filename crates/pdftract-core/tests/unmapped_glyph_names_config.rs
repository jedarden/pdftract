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
    assert!(
        config.unmapped_glyph_names.is_empty(),
        "unmapped_glyph_names should default to an empty list when not specified in config. \
         Expected: empty Vec. \
         Found: {:?}. \
         Why this matters: The #[serde(default)] attribute ensures empty Vec when field is absent, \
         preventing null/None errors during encoding fixture generation.",
        config.unmapped_glyph_names
    );
    assert_eq!(
        config.unmapped_glyph_names.len(),
        0,
        "unmapped_glyph_names length should be 0 when defaulted. \
         Expected: 0. \
         Found: {}. \
         Why this matters: Confirms the default is truly empty, not a populated list.",
        config.unmapped_glyph_names.len()
    );

    // Verify other fields are parsed correctly
    assert_eq!(
        config.description,
        Some("Test config without unmapped_glyph_names".to_string()),
        "description field should be parsed correctly. \
         Expected: Some(\"Test config without unmapped_glyph_names\"). \
         Found: {:?}. \
         Why this matters: Verifies the parser correctly handles string fields when unmapped_glyph_names is absent.",
        config.description
    );
    assert_eq!(
        config.version,
        Some("1.0".to_string()),
        "version field should be parsed correctly. \
         Expected: Some(\"1.0\"). \
         Found: {:?}. \
         Why this matters: Confirms version parsing is independent of the unmapped_glyph_names field.",
        config.version
    );
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
    assert_eq!(
        config.unmapped_glyph_names.len(),
        3,
        "unmapped_glyph_names length should be 3 when explicitly specified. \
         Expected: 3. \
         Found: {}. \
         Why this matters: Confirms the parser correctly counts the number of glyph names in the array.",
        config.unmapped_glyph_names.len()
    );
    assert_eq!(
        config.unmapped_glyph_names,
        vec![".notdef", ".null", "g000"],
        "unmapped_glyph_names should contain exactly the specified glyph names. \
         Expected: [\".notdef\", \".null\", \"g000\"]. \
         Found: {:?}. \
         Why this matters: Verifies the parser preserves both standard PDF glyphs (.notdef, .null) \
         and custom PUA glyphs (g000) as configured in build/unmapped-glyph-names.json.",
        config.unmapped_glyph_names
    );

    // Verify other fields are parsed correctly
    assert_eq!(
        config.description,
        Some("Test config with unmapped_glyph_names".to_string()),
        "description field should be parsed correctly when unmapped_glyph_names is present. \
         Expected: Some(\"Test config with unmapped_glyph_names\"). \
         Found: {:?}. \
         Why this matters: Confirms description parsing works correctly alongside the unmapped_glyph_names array.",
        config.description
    );
    assert_eq!(
        config.version,
        Some("1.0".to_string()),
        "version field should be parsed correctly when unmapped_glyph_names is present. \
         Expected: Some(\"1.0\"). \
         Found: {:?}. \
         Why this matters: Verifies version parsing is independent of the unmapped_glyph_names field.",
        config.version
    );
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
    assert!(
        config.unmapped_glyph_names.is_empty(),
        "unmapped_glyph_names should be empty when explicitly set to empty array. \
         Expected: empty Vec. \
         Found: {:?}. \
         Why this matters: An explicit empty array [] should be treated the same as a missing field, \
         producing an empty Vec instead of None or an error.",
        config.unmapped_glyph_names
    );
    assert_eq!(
        config.unmapped_glyph_names.len(),
        0,
        "unmapped_glyph_names length should be 0 for explicit empty array. \
         Expected: 0. \
         Found: {}. \
         Why this matters: Confirms the parser correctly handles the explicit [] case without \
         misinterpreting it as a missing or malformed field.",
        config.unmapped_glyph_names.len()
    );
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
    assert!(
        config.unmapped_glyph_names.is_empty(),
        "unmapped_glyph_names should default to empty for completely empty config. \
         Expected: empty Vec. \
         Found: {:?}. \
         Why this matters: The #[serde(default)] attribute ensures the field is always present \
         as an empty Vec, never None, even when the entire config is just {{}}.",
        config.unmapped_glyph_names
    );
    assert!(
        config.description.is_none(),
        "description should default to None when not specified in empty config. \
         Expected: None. \
         Found: {:?}. \
         Why this matters: Confirms that optional fields correctly default to None instead of \
         producing empty strings or errors.",
        config.description
    );
    assert!(
        config.version.is_none(),
        "version should default to None when not specified in empty config. \
         Expected: None. \
         Found: {:?}. \
         Why this matters: Verifies that optional version field correctly defaults to None, \
         allowing configs without version specification.",
        config.version
    );
}
