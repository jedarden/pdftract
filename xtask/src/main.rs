use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Profile {
    description: String,
    #[serde(default)]
    profile_fields: BTreeMap<String, ProfileField>,
    #[serde(default)]
    match_config: MatchConfig,
}

#[derive(Debug, Deserialize)]
struct ProfileField {
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    extraction: ExtractionConfig,
}

#[derive(Debug, Deserialize, Default)]
struct ExtractionConfig {
    #[serde(default)]
    patterns: Vec<String>,
    #[serde(default)]
    fallback: serde_yaml::Value,
}

#[derive(Debug, Deserialize, Default)]
struct MatchConfig {
    #[serde(default)]
    text_patterns: Vec<String>,
    #[serde(default)]
    structural: Vec<String>,
    #[serde(default)]
    page_count_hint: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: xtask <command>");
        eprintln!("Commands:");
        eprintln!("  doc-profile <profile-name>  Generate README skeleton for a profile");
        eprintln!("  doc-profiles                 Generate README skeletons for all profiles");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "doc-profile" => {
            if args.len() < 3 {
                eprintln!("Usage: xtask doc-profile <profile-name>");
                std::process::exit(1);
            }
            generate_profile_readme(&args[2])?;
        }
        "doc-profiles" => {
            let profiles_dir = Path::new("profiles/builtin");
            for entry in fs::read_dir(profiles_dir)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    let profile_name = entry.file_name().to_string_lossy().to_string();
                    if let Err(e) = generate_profile_readme(&profile_name) {
                        eprintln!("Error generating README for {}: {}", profile_name, e);
                    }
                }
            }
        }
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn generate_profile_readme(profile_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let profile_path = Path::new("profiles/builtin").join(profile_name).join("profile.yaml");
    let readme_path = Path::new("profiles/builtin").join(profile_name).join("README.md");

    if !profile_path.exists() {
        return Err(format!("Profile YAML not found: {}", profile_path.display()).into());
    }

    let yaml_content = fs::read_to_string(&profile_path)?;
    let profile: Profile = serde_yaml::from_str(&yaml_content)?;

    let mut readme = String::new();

    // Title and description
    readme.push_str(&format!("# {} Profile\n\n", profile_name.to_uppercase()));
    readme.push_str(&format!("{}\n\n", profile.description));

    // Match Criteria Summary (placeholder for human to fill)
    readme.push_str("## Match Criteria Summary\n\n");
    readme.push_str("*This section describes the characteristics that cause a document to match this profile. The following signals are considered:*\n\n");

    if let Some(hint) = profile.match_config.page_count_hint {
        readme.push_str(&format!("- **Page count hint**: {}\n", hint));
    }

    if !profile.match_config.text_patterns.is_empty() {
        readme.push_str("- **Text patterns**: ");
        for (i, pattern) in profile.match_config.text_patterns.iter().enumerate() {
            if i > 0 {
                readme.push_str(", ");
            }
            readme.push_str(&format!("`{}`", pattern));
        }
        readme.push('\n');
    }

    if !profile.match_config.structural.is_empty() {
        readme.push_str("- **Structural signals**: ");
        for (i, signal) in profile.match_config.structural.iter().enumerate() {
            if i > 0 {
                readme.push_str(", ");
            }
            readme.push_str(&format!("`{}`", signal));
        }
        readme.push('\n');
    }

    readme.push_str("\n*Additional heuristics and confidence scoring are applied during classification.*\n\n");

    // Extracted Fields
    readme.push_str("## Extracted Fields\n\n");
    readme.push_str("| Field | Type | Description | Example Value | Source Hint |\n");
    readme.push_str("|-------|------|-------------|----------------|-------------|\n");

    for (field_name, field) in &profile.profile_fields {
        let description = format!("Extracted from page text using pattern matching");
        let example = match field.field_type.as_str() {
            "string" => "\"example value\"",
            "decimal" => "123.45",
            "date" => "2024-01-15",
            "int" => "42",
            "array" => "[...]",
            _ => "N/A",
        };
        let source = "regex patterns in profile YAML";
        readme.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            field_name, field.field_type, description, example, source
        ));
    }

    if profile.profile_fields.is_empty() {
        readme.push_str("| *(none)* | - | *This profile has no field extractors* | - | - |\n");
    }

    readme.push('\n');

    // Known Limitations
    readme.push_str("## Known Limitations\n\n");
    readme.push_str("*This section documents known edge cases and failure modes. Contributions to improve extraction quality are welcome.*\n\n");
    readme.push_str("- *Document limitations and edge cases to be added by profile author*\n\n");

    // Sample Input Pointer
    readme.push_str("## Sample Input\n\n");
    readme.push_str(&format!("Example fixtures demonstrating this profile are available in `tests/fixtures/profiles/{}/`.\n\n", profile_name));
    readme.push_str("*See the classifier corpus for representative documents.*\n\n");

    // Configuration Tips
    readme.push_str("## Configuration Tips\n\n");
    readme.push_str("To override this profile:\n\n");
    readme.push_str("```bash\n");
    readme.push_str(&format!("pdftract profiles export {} > my-profile.yaml\n", profile_name));
    readme.push_str("# Edit my-profile.yaml to customize match criteria, fields, or extraction patterns\n");
    readme.push_str(&format!("pdftract extract --profile my-profile.yaml document.pdf\n", profile_name));
    readme.push_str("```\n\n");

    // Footer
    readme.push_str(&format!("---\n\n*This README was auto-generated from `profile.yaml`. Update the Match Criteria Summary and Known Limitations sections with profile-specific guidance.*\n"));

    fs::write(&readme_path, readme)?;
    println!("Generated README for {} at {}", profile_name, readme_path.display());

    Ok(())
}
