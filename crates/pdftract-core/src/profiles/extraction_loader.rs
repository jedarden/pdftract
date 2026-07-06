//! Extraction profile loader (Phase 7.10).
//!
//! Loads extraction profiles from built-in sources, system directories,
//! XDG config paths, and custom --profile-dir flags.

use super::extraction::ExtractionProfile;
use super::loader::{check_forbidden_keys, ForbiddenKeyError, ProfileLoadError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Profile source with priority metadata.
#[derive(Debug, Clone)]
pub struct ProfileSource {
    /// The loaded profile
    pub profile: ExtractionProfile,

    /// Where this profile came from
    pub source: ProfileOrigin,

    /// Whether this overrides a built-in profile
    pub overrides_builtin: bool,

    /// Whether this overrides a community profile
    pub overrides_community: bool,
}

/// Origin of a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileOrigin {
    /// Built-in profile (compiled into binary)
    BuiltIn,

    /// Community profile (profiles/community/)
    Community,

    /// System-wide profile (/etc/pdftract/profiles/)
    System,

    /// User profile (XDG config directory)
    User,

    /// Custom profile directory (--profile-dir)
    Custom(PathBuf),
}

/// Load all extraction profiles from the search path.
///
/// Search order (lowest to highest priority):
/// 1. Built-in profiles (compiled in)
/// 2. Community profiles (profiles/community/)
/// 3. System directory (/etc/pdftract/profiles/)
/// 4. User directory (XDG config: ~/.config/pdftract/profiles/)
/// 5. Custom directories (--profile-dir, repeatable)
///
/// Later sources override earlier ones on name collision.
pub fn load_extraction_profiles(
    custom_dirs: &[PathBuf],
) -> Result<Vec<ProfileSource>, ProfileLoadError> {
    let mut profiles_by_name: HashMap<String, ProfileSource> = HashMap::new();

    // 1. Load built-in profiles
    load_builtin_profiles(&mut profiles_by_name)?;

    // 2. Load community profiles
    let community_dir = PathBuf::from("profiles/community");
    if community_dir.exists() {
        load_profiles_from_dir(
            &community_dir,
            ProfileOrigin::Community,
            &mut profiles_by_name,
        )?;
    }

    // 3. Load system profiles
    let system_dir = PathBuf::from("/etc/pdftract/profiles");
    if system_dir.exists() {
        load_profiles_from_dir(&system_dir, ProfileOrigin::System, &mut profiles_by_name)?;
    }

    // 4. Load user profiles (XDG config)
    if let Some(user_dir) = get_xdg_profile_dir() {
        if user_dir.exists() {
            load_profiles_from_dir(&user_dir, ProfileOrigin::User, &mut profiles_by_name)?;
        }
    }

    // 5. Load custom profiles (--profile-dir)
    for custom_dir in custom_dirs {
        if custom_dir.exists() {
            let origin = ProfileOrigin::Custom(custom_dir.clone());
            load_profiles_from_dir(custom_dir, origin, &mut profiles_by_name)?;
        }
    }

    // Convert to vector, sorted by priority then by name
    let mut profiles: Vec<ProfileSource> = profiles_by_name.into_values().collect();
    profiles.sort_by(|a, b| {
        b.profile
            .priority
            .cmp(&a.profile.priority)
            .then_with(|| a.profile.name.cmp(&b.profile.name))
    });

    Ok(profiles)
}

/// Get the XDG config directory for pdftract profiles.
///
/// Returns ~/.config/pdftract/profiles/ or None if XDG config is not available.
pub fn get_xdg_profile_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("pdftract").join("profiles"))
}

/// Load built-in extraction profiles.
///
/// These are compiled into the binary via include_str!.
fn load_builtin_profiles(
    profiles: &mut HashMap<String, ProfileSource>,
) -> Result<(), ProfileLoadError> {
    #[cfg(feature = "profiles")]
    {
        // Load each built-in profile individually
        let profile_results: Vec<(&str, Result<ExtractionProfile, ProfileLoadError>)> = vec![
            (
                "invoice",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/invoice/profile.yaml"),
                    "profiles/builtin/invoice/profile.yaml",
                ),
            ),
            (
                "receipt",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/receipt/profile.yaml"),
                    "profiles/builtin/receipt/profile.yaml",
                ),
            ),
            (
                "contract",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/contract/profile.yaml"),
                    "profiles/builtin/contract/profile.yaml",
                ),
            ),
            (
                "scientific_paper",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/scientific_paper/profile.yaml"),
                    "profiles/builtin/scientific_paper/profile.yaml",
                ),
            ),
            (
                "slide_deck",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/slide_deck/profile.yaml"),
                    "profiles/builtin/slide_deck/profile.yaml",
                ),
            ),
            (
                "form",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/form/profile.yaml"),
                    "profiles/builtin/form/profile.yaml",
                ),
            ),
            (
                "bank_statement",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/bank_statement/profile.yaml"),
                    "profiles/builtin/bank_statement/profile.yaml",
                ),
            ),
            (
                "legal_filing",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/legal_filing/profile.yaml"),
                    "profiles/builtin/legal_filing/profile.yaml",
                ),
            ),
            (
                "book_chapter",
                load_profile_yaml(
                    include_str!("../../../../profiles/builtin/book_chapter/profile.yaml"),
                    "profiles/builtin/book_chapter/profile.yaml",
                ),
            ),
        ];

        for (name, result) in profile_results {
            match result {
                Ok(profile) => {
                    profiles.insert(
                        profile.name.clone(),
                        ProfileSource {
                            profile,
                            source: ProfileOrigin::BuiltIn,
                            overrides_builtin: false,
                            overrides_community: false,
                        },
                    );
                }
                Err(e) => {
                    eprintln!("Failed to parse built-in profile '{}': {}", name, e);
                }
            }
        }
    }

    Ok(())
}

/// Load a profile from YAML content.
fn load_profile_yaml(
    content: &str,
    source_path: &str,
) -> Result<ExtractionProfile, ProfileLoadError> {
    // Check for forbidden keys first
    let yaml_value = serde_yaml::from_str::<serde_yaml::Value>(content)?;

    // Get the original content for line number detection
    if let Err(e) = check_forbidden_keys(&yaml_value, "", content) {
        return Err(ProfileLoadError::ForbiddenKey {
            key: e.key,
            path: format!("{}: {}", source_path, e.path),
            line: e.line,
        });
    }

    // Parse as ExtractionProfile
    let profile: ExtractionProfile =
        serde_yaml::from_str(content).map_err(ProfileLoadError::YamlError)?;

    Ok(profile)
}

/// Load profiles from a directory.
fn load_profiles_from_dir(
    dir: &Path,
    origin: ProfileOrigin,
    profiles: &mut HashMap<String, ProfileSource>,
) -> Result<(), ProfileLoadError> {
    let entries = fs::read_dir(dir).map_err(ProfileLoadError::IoError)?;

    for entry in entries {
        let entry = entry.map_err(ProfileLoadError::IoError)?;
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            // Check for profile.yaml subdirectory (e.g., invoice/profile.yaml)
            let profile_yaml = path.join("profile.yaml");
            if profile_yaml.exists() {
                if let Ok(profile) = load_profile_file(&profile_yaml) {
                    let (overrides_builtin, overrides_community) =
                        if let Some(existing) = profiles.get(&profile.name) {
                            match &existing.source {
                                ProfileOrigin::BuiltIn => (true, false),
                                ProfileOrigin::Community => (false, true),
                                _ => (false, false),
                            }
                        } else {
                            (false, false)
                        };

                    profiles.insert(
                        profile.name.clone(),
                        ProfileSource {
                            profile,
                            source: origin.clone(),
                            overrides_builtin,
                            overrides_community,
                        },
                    );
                }
            }
            continue;
        }

        // Only load .yaml files
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }

        if let Ok(profile) = load_profile_file(&path) {
            let (overrides_builtin, overrides_community) =
                if let Some(existing) = profiles.get(&profile.name) {
                    match &existing.source {
                        ProfileOrigin::BuiltIn => (true, false),
                        ProfileOrigin::Community => (false, true),
                        _ => (false, false),
                    }
                } else {
                    (false, false)
                };

            profiles.insert(
                profile.name.clone(),
                ProfileSource {
                    profile,
                    source: origin.clone(),
                    overrides_builtin,
                    overrides_community,
                },
            );
        }
    }

    Ok(())
}

/// Load a single profile from a file.
pub fn load_profile_file(path: &Path) -> Result<ExtractionProfile, ProfileLoadError> {
    let content = fs::read_to_string(path).map_err(ProfileLoadError::IoError)?;
    load_profile_yaml(&content, &path.to_string_lossy())
}

/// Find a profile by name or path.
///
/// - If `name_or_path` is an existing file path, load it directly
/// - Otherwise, search for a profile with that name in the loaded profiles
pub fn find_profile(
    name_or_path: &str,
    profiles: &[ProfileSource],
) -> Result<ExtractionProfile, ProfileLoadError> {
    // First, check if it's a file path
    let path = PathBuf::from(name_or_path);
    if path.exists() {
        return load_profile_file(&path);
    }

    // Search by name
    for source in profiles {
        if source.profile.name == name_or_path {
            return Ok(source.profile.clone());
        }
    }

    Err(ProfileLoadError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Profile '{}' not found", name_or_path),
    )))
}

/// Validate a profile file without loading it into the profile set.
///
/// Returns Ok(()) if the profile is valid, Err with details if invalid.
pub fn validate_profile_file(path: &Path) -> Result<(), ProfileLoadError> {
    let content = fs::read_to_string(path).map_err(ProfileLoadError::IoError)?;

    // Check for forbidden keys
    let yaml_value =
        serde_yaml::from_str::<serde_yaml::Value>(&content).map_err(ProfileLoadError::YamlError)?;

    check_forbidden_keys(&yaml_value, "", &content).map_err(|e| {
        ProfileLoadError::ForbiddenKey {
            key: e.key,
            path: e.path,
            line: e.line,
        }
    })?;

    // Try to parse as ExtractionProfile
    let _: ExtractionProfile =
        serde_yaml::from_str(&content).map_err(ProfileLoadError::YamlError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_xdg_profile_dir() {
        let dir = get_xdg_profile_dir();
        assert!(dir.is_some());
        let path = dir.unwrap();
        assert!(path.ends_with("pdftract/profiles"));
    }

    #[test]
    fn test_load_builtin_profiles() {
        let mut profiles = HashMap::new();
        let result = load_builtin_profiles(&mut profiles);

        #[cfg(feature = "profiles")]
        {
            assert!(result.is_ok());
            // Should have loaded some profiles
            assert!(!profiles.is_empty());
        }
    }

    #[test]
    fn test_validate_simple_profile() {
        let yaml = r#"
name: test
description: Test profile
priority: 10
match:
  text_contains:
    patterns: ["test"]
"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let profile_path = temp_dir.path().join("test.yaml");
        fs::write(&profile_path, yaml).unwrap();

        let result = validate_profile_file(&profile_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_profile_with_forbidden_key() {
        let yaml = r#"
name: test
description: Test profile
priority: 10
match:
  text_contains:
    patterns: ["test"]
api_key: "secret"
"#;

        let temp_dir = tempfile::tempdir().unwrap();
        let profile_path = temp_dir.path().join("test.yaml");
        fs::write(&profile_path, yaml).unwrap();

        let result = validate_profile_file(&profile_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_extraction_profiles_empty() {
        let profiles = load_extraction_profiles(&[]).unwrap();
        #[cfg(feature = "profiles")]
        assert!(!profiles.is_empty()); // At least built-ins
    }
}
