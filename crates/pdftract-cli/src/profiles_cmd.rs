//! Profile management CLI subcommand.
//!
//! This module implements the `pdftract profiles` command family for managing
//! document type profiles (list, show, export, install, validate).

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Arguments for the profiles subcommand.
pub struct ProfilesArgs {
    /// Subcommand to run
    pub command: ProfilesCommand,
}

/// Profiles subcommands.
#[derive(Debug, Clone)]
pub enum ProfilesCommand {
    /// List all available profiles
    List,
    /// Show a profile's YAML content
    Show { name_or_path: String },
    /// Export a built-in profile to stdout
    Export { name: String },
    /// Install a profile to the user config directory
    Install { path: PathBuf },
    /// Validate a profile file
    Validate { path: PathBuf },
}

/// Run the profiles subcommand.
pub fn run_profiles(args: ProfilesArgs) -> Result<()> {
    match args.command {
        ProfilesCommand::List => run_list(),
        ProfilesCommand::Show { name_or_path } => run_show(&name_or_path),
        ProfilesCommand::Export { name } => run_export(&name),
        ProfilesCommand::Install { path } => run_install(&path),
        ProfilesCommand::Validate { path } => run_validate(&path),
    }
}

/// List all available profiles.
fn run_list() -> Result<()> {
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::extraction_loader;

        // Load all extraction profiles
        let profiles = extraction_loader::load_extraction_profiles(&[])?;

        if profiles.is_empty() {
            println!("No profiles available.");
            println!();
            println!("Built-in profiles may not be enabled. Build pdftract with:");
            println!("  cargo build --features profiles");
            return Ok(());
        }

        println!("Available profiles ({} total):", profiles.len());
        println!();

        // Group by origin
        let mut builtin = Vec::new();
        let mut community = Vec::new();
        let mut user = Vec::new();
        let mut custom = Vec::new();

        for source in &profiles {
            match source.source {
                extraction_loader::ProfileOrigin::BuiltIn => builtin.push(source),
                extraction_loader::ProfileOrigin::Community => community.push(source),
                extraction_loader::ProfileOrigin::User => user.push(source),
                extraction_loader::ProfileOrigin::Custom(_) => custom.push(source),
                extraction_loader::ProfileOrigin::System => {
                    // System profiles - add to a separate group or merge with user
                    user.push(source);
                }
            }
        }

        // Print built-in profiles
        if !builtin.is_empty() {
            println!("Built-in profiles:");
            for source in builtin {
                let profile = &source.profile;
                println!(
                    "  {} - Priority: {}{}",
                    profile.name,
                    profile.priority,
                    if source.overrides_builtin {
                        " (overrides built-in)"
                    } else {
                        ""
                    }
                );
                println!("    {}", profile.description);
            }
            println!();
        }

        // Print community profiles
        if !community.is_empty() {
            println!("Community profiles:");
            for source in community {
                let profile = &source.profile;
                println!(
                    "  {} - Priority: {}{}",
                    profile.name,
                    profile.priority,
                    if source.overrides_builtin {
                        " (overrides built-in)"
                    } else {
                        ""
                    }
                );
                println!("    {}", profile.description);
            }
            println!();
        }

        // Print user profiles
        if !user.is_empty() {
            println!("User profiles:");
            for source in user {
                let profile = &source.profile;
                println!(
                    "  {} - Priority: {}{}",
                    profile.name,
                    profile.priority,
                    if source.overrides_builtin {
                        " (overrides built-in)"
                    } else {
                        ""
                    }
                );
                println!("    {}", profile.description);
            }
            println!();
        }

        // Print custom profiles
        if !custom.is_empty() {
            println!("Custom profiles:");
            for source in custom {
                let profile = &source.profile;
                println!(
                    "  {} - Priority: {}",
                    profile.name, profile.priority
                );
                println!("    {}", profile.description);
            }
            println!();
        }
    }

    #[cfg(not(feature = "profiles"))]
    {
        println!("Profiles are not enabled.");
        println!();
        println!("Build pdftract with the profiles feature:");
        println!("  cargo build --features profiles");
    }

    Ok(())
}

/// Show a profile's YAML content.
fn run_show(name_or_path: &str) -> Result<()> {
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::extraction_loader;

        // Load all profiles to search by name
        let profiles = extraction_loader::load_extraction_profiles(&[])?;

        // Try to find the profile
        let profile = extraction_loader::find_profile(name_or_path, &profiles)?;

        // Serialize back to YAML
        let yaml = serde_yaml::to_string(&profile)
            .context("Failed to serialize profile to YAML")?;

        println!("{}", yaml);
    }

    #[cfg(not(feature = "profiles"))]
    {
        anyhow::bail!("Profiles feature is not enabled. Build with: --features profiles");
    }

    Ok(())
}

/// Export a built-in profile to stdout.
fn run_export(name: &str) -> Result<()> {
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::extraction_loader;

        // Load all profiles
        let profiles = extraction_loader::load_extraction_profiles(&[])?;

        // Find the built-in profile by name
        let profile = profiles
            .iter()
            .find(|s| s.profile.name == name && matches!(s.source, extraction_loader::ProfileOrigin::BuiltIn))
            .context(format!("Built-in profile '{}' not found", name))?;

        // Serialize to YAML
        let yaml = serde_yaml::to_string(&profile)
            .context("Failed to serialize profile to YAML")?;

        println!("{}", yaml);
    }

    #[cfg(not(feature = "profiles"))]
    {
        anyhow::bail!("Profiles feature is not enabled. Build with: --features profiles");
    }

    Ok(())
}

/// Install a profile to the user config directory.
fn run_install(path: &PathBuf) -> Result<()> {
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::extraction_loader;

        // Check if source file exists
        if !path.exists() {
            anyhow::bail!("Profile file not found: {}", path.display());
        }

        // Get XDG config directory
        let xdg_dir = extraction_loader::get_xdg_profile_dir()
            .context("Failed to determine XDG config directory")?;

        // Create directory if it doesn't exist
        fs::create_dir_all(&xdg_dir)
            .context(format!("Failed to create profile directory: {}", xdg_dir.display()))?;

        // Read the profile to get its name
        let content = fs::read_to_string(path)
            .context(format!("Failed to read profile file: {}", path.display()))?;

        // Parse to get the profile name
        let profile: pdftract_core::profiles::ExtractionProfile = serde_yaml::from_str(&content)
            .context("Failed to parse profile YAML")?;

        // Destination path
        let dest = xdg_dir.join(format!("{}.yaml", profile.name));

        // Copy file
        fs::copy(path, &dest)
            .context(format!("Failed to copy profile to: {}", dest.display()))?;

        println!("Installed profile '{}' to: {}", profile.name, dest.display());
        println!();
        println!("You can now use this profile with:");
        println!("  pdftract extract --profile {}", profile.name);
    }

    #[cfg(not(feature = "profiles"))]
    {
        anyhow::bail!("Profiles feature is not enabled. Build with: --features profiles");
    }

    Ok(())
}

/// Validate a profile file.
fn run_validate(path: &PathBuf) -> Result<()> {
    #[cfg(feature = "profiles")]
    {
        use pdftract_core::profiles::extraction_loader;

        // Check if file exists
        if !path.exists() {
            anyhow::bail!("Profile file not found: {}", path.display());
        }

        // Validate the profile
        match extraction_loader::validate_profile_file(path) {
            Ok(()) => {
                println!("Profile '{}' is valid.", path.display());
                return Ok(());
            }
            Err(e) => {
                anyhow::bail!("Profile validation failed: {}", e);
            }
        }
    }

    #[cfg(not(feature = "profiles"))]
    {
        anyhow::bail!("Profiles feature is not enabled. Build with: --features profiles");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiles_command_enum() {
        let command = ProfilesCommand::List;
        assert!(matches!(command, ProfilesCommand::List));

        let show = ProfilesCommand::Show {
            name_or_path: "invoice".to_string(),
        };
        assert!(matches!(show, ProfilesCommand::Show { .. }));

        let export = ProfilesCommand::Export {
            name: "invoice".to_string(),
        };
        assert!(matches!(export, ProfilesCommand::Export { .. }));
    }
}
