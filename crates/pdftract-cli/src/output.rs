use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;

/// Output format type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Json,
    Markdown,
    Text,
    Ndjson,
}

impl Format {
    /// Parse a format string into a Format
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(Format::Json),
            "markdown" | "md" => Ok(Format::Markdown),
            "text" | "txt" => Ok(Format::Text),
            "ndjson" => Ok(Format::Ndjson),
            _ => Err(anyhow!(
                "unknown format: '{}', expected one of: json, markdown, text, ndjson",
                s
            )),
        }
    }

    /// Get the file extension for this format
    pub fn extension(&self) -> &'static str {
        match self {
            Format::Json => ".json",
            Format::Markdown => ".md",
            Format::Text => ".txt",
            Format::Ndjson => ".ndjson",
        }
    }

    /// Get the format name for error messages
    pub fn name(&self) -> &'static str {
        match self {
            Format::Json => "json",
            Format::Markdown => "markdown",
            Format::Text => "text",
            Format::Ndjson => "ndjson",
        }
    }

    /// Get the flag name for this format
    pub fn flag_name(&self) -> &'static str {
        match self {
            Format::Json => "--json",
            Format::Markdown => "--md",
            Format::Text => "--text",
            Format::Ndjson => "--ndjson",
        }
    }
}

/// Output destination
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    File(PathBuf),
    Stdout,
}

impl Destination {
    /// Create a Destination from a path string
    /// "-" is interpreted as stdout
    pub fn from_path(path: PathBuf) -> Self {
        if path.as_os_str() == "-" {
            Destination::Stdout
        } else {
            Destination::File(path)
        }
    }

    /// Check if this destination is stdout
    pub fn is_stdout(&self) -> bool {
        matches!(self, Destination::Stdout)
    }
}

/// Output specification combining format and destination
#[derive(Debug, Clone)]
pub struct OutputSpec {
    pub format: Format,
    pub dest: Destination,
}

impl OutputSpec {
    /// Create a new OutputSpec
    pub fn new(format: Format, dest: Destination) -> Self {
        OutputSpec { format, dest }
    }

    /// Create an OutputSpec from a format and a path
    pub fn from_path(format: Format, path: PathBuf) -> Self {
        OutputSpec::new(format, Destination::from_path(path))
    }

    /// Create an OutputSpec for a format with auto-named file
    pub fn auto_named(format: Format, base: &PathBuf) -> Self {
        let mut filename = base.clone();
        filename.set_extension(format.extension().trim_start_matches('.'));
        OutputSpec::new(format, Destination::File(filename))
    }
}

/// Tracks the source of a format specification for better error messages
#[derive(Debug, Clone, PartialEq, Eq)]
enum FormatSource {
    /// Specified via --json, --md, or --text flag
    Flag(&'static str),
    /// Specified via --format list
    FormatList,
}

/// Parsed output configuration from CLI flags
#[derive(Debug, Clone, Default)]
pub struct OutputConfig {
    pub json: Vec<PathBuf>,
    pub md: Vec<PathBuf>,
    pub text: Vec<PathBuf>,
    pub ndjson: bool,
    pub format_list: Vec<String>,
    pub output_base: Option<PathBuf>,
}

impl OutputConfig {
    /// Check if no output flags were specified
    pub fn is_empty(&self) -> bool {
        self.json.is_empty()
            && self.md.is_empty()
            && self.text.is_empty()
            && !self.ndjson
            && self.format_list.is_empty()
    }

    /// Build OutputSpecs from the configuration with validation
    pub fn build_specs(&self) -> Result<Vec<OutputSpec>> {
        let mut specs = Vec::new();

        // Track which formats have been specified and from where
        let mut format_sources: HashMap<Format, FormatSource> = HashMap::new();
        let mut stdout_count = 0;
        let mut stdout_spec = None;

        // Handle individual format flags
        if !self.json.is_empty() {
            let format = Format::Json;
            if self.json.len() > 1 {
                return Err(Self::duplicate_format_error(
                    format,
                    &FormatSource::Flag("--json"),
                    &FormatSource::Flag("--json"),
                ));
            }
            if let Some(existing) = format_sources.get(&format) {
                return Err(Self::duplicate_format_error(
                    format,
                    existing,
                    &FormatSource::Flag("--json"),
                ));
            }
            format_sources.insert(format, FormatSource::Flag("--json"));
            let dest = Destination::from_path(self.json[0].clone());
            if dest.is_stdout() {
                stdout_count += 1;
                stdout_spec = Some(OutputSpec::new(format, dest));
            } else {
                specs.push(OutputSpec::new(format, dest));
            }
        }

        if !self.md.is_empty() {
            let format = Format::Markdown;
            if self.md.len() > 1 {
                return Err(Self::duplicate_format_error(
                    format,
                    &FormatSource::Flag("--md"),
                    &FormatSource::Flag("--md"),
                ));
            }
            if let Some(existing) = format_sources.get(&format) {
                return Err(Self::duplicate_format_error(
                    format,
                    existing,
                    &FormatSource::Flag("--md"),
                ));
            }
            format_sources.insert(format, FormatSource::Flag("--md"));
            let dest = Destination::from_path(self.md[0].clone());
            if dest.is_stdout() {
                stdout_count += 1;
                stdout_spec = Some(OutputSpec::new(format, dest));
            } else {
                specs.push(OutputSpec::new(format, dest));
            }
        }

        if !self.text.is_empty() {
            let format = Format::Text;
            if self.text.len() > 1 {
                return Err(Self::duplicate_format_error(
                    format,
                    &FormatSource::Flag("--text"),
                    &FormatSource::Flag("--text"),
                ));
            }
            if let Some(existing) = format_sources.get(&format) {
                return Err(Self::duplicate_format_error(
                    format,
                    existing,
                    &FormatSource::Flag("--text"),
                ));
            }
            format_sources.insert(format, FormatSource::Flag("--text"));
            let dest = Destination::from_path(self.text[0].clone());
            if dest.is_stdout() {
                stdout_count += 1;
                stdout_spec = Some(OutputSpec::new(format, dest));
            } else {
                specs.push(OutputSpec::new(format, dest));
            }
        }

        if self.ndjson {
            let format = Format::Ndjson;
            if let Some(existing) = format_sources.get(&format) {
                return Err(Self::duplicate_format_error(
                    format,
                    existing,
                    &FormatSource::Flag("--ndjson"),
                ));
            }
            format_sources.insert(format, FormatSource::Flag("--ndjson"));
            stdout_spec = Some(OutputSpec::new(format, Destination::Stdout));
            stdout_count += 1;
        }

        // Handle --format + -o auto-naming
        if !self.format_list.is_empty() {
            let base = self
                .output_base
                .as_ref()
                .ok_or_else(|| anyhow!("--format requires -o (output base path)"))?;

            for format_str in &self.format_list {
                let format = Format::from_str(format_str)?;
                if let Some(existing) = format_sources.get(&format) {
                    return Err(Self::duplicate_format_error(
                        format,
                        existing,
                        &FormatSource::FormatList,
                    ));
                }
                format_sources.insert(format, FormatSource::FormatList);

                let spec = OutputSpec::auto_named(format, base);
                if spec.dest.is_stdout() {
                    stdout_count += 1;
                    stdout_spec = Some(spec);
                } else {
                    specs.push(spec);
                }
            }
        }

        // Validation: at most one stdout
        if stdout_count > 1 {
            return Err(anyhow!(
                "at most one output may be stdout (-); multiple formats cannot all write to stdout"
            ));
        }

        // Validation: ndjson is exclusive
        if format_sources.contains_key(&Format::Ndjson)
            && specs.len() + stdout_spec.is_some() as usize > 1
        {
            return Err(anyhow!(
                "--ndjson cannot be combined with other output formats"
            ));
        }

        // Default: single JSON to stdout if no specs
        if specs.is_empty() && stdout_spec.is_none() {
            return Ok(vec![OutputSpec::new(Format::Json, Destination::Stdout)]);
        }

        // Put stdout spec first if present
        if let Some(stdout) = stdout_spec {
            let mut result = vec![stdout];
            result.extend(specs);
            Ok(result)
        } else {
            Ok(specs)
        }
    }

    /// Generate a helpful error message for duplicate format specifications
    fn duplicate_format_error(
        format: Format,
        existing: &FormatSource,
        new: &FormatSource,
    ) -> anyhow::Error {
        match (existing, new) {
            (FormatSource::Flag(existing_flag), FormatSource::Flag(new_flag)) => {
                anyhow!(
                    "duplicate format: {} and {} both specify {} output",
                    existing_flag,
                    new_flag,
                    format.name()
                )
            }
            (FormatSource::Flag(flag), FormatSource::FormatList) => {
                anyhow!(
                    "duplicate format: {} and --format {} both specify {} output",
                    flag,
                    format.name(),
                    format.name()
                )
            }
            (FormatSource::FormatList, FormatSource::Flag(flag)) => {
                anyhow!(
                    "duplicate format: --format {} and {} both specify {} output",
                    format.name(),
                    flag,
                    format.name()
                )
            }
            (FormatSource::FormatList, FormatSource::FormatList) => {
                anyhow!(
                    "duplicate format: --format {} specified more than once",
                    format.name()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_from_str() {
        assert_eq!(Format::from_str("json").unwrap(), Format::Json);
        assert_eq!(Format::from_str("markdown").unwrap(), Format::Markdown);
        assert_eq!(Format::from_str("md").unwrap(), Format::Markdown);
        assert_eq!(Format::from_str("text").unwrap(), Format::Text);
        assert_eq!(Format::from_str("txt").unwrap(), Format::Text);
        assert_eq!(Format::from_str("ndjson").unwrap(), Format::Ndjson);

        let err = Format::from_str("invalid").unwrap_err();
        assert!(err.to_string().contains("unknown format"));
        assert!(err.to_string().contains("invalid"));
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(Format::Json.extension(), ".json");
        assert_eq!(Format::Markdown.extension(), ".md");
        assert_eq!(Format::Text.extension(), ".txt");
        assert_eq!(Format::Ndjson.extension(), ".ndjson");
    }

    #[test]
    fn test_format_flag_name() {
        assert_eq!(Format::Json.flag_name(), "--json");
        assert_eq!(Format::Markdown.flag_name(), "--md");
        assert_eq!(Format::Text.flag_name(), "--text");
        assert_eq!(Format::Ndjson.flag_name(), "--ndjson");
    }

    #[test]
    fn test_destination_from_path() {
        assert_eq!(
            Destination::from_path(PathBuf::from("-")),
            Destination::Stdout
        );
        assert_eq!(
            Destination::from_path(PathBuf::from("out.json")),
            Destination::File(PathBuf::from("out.json"))
        );
        assert!(Destination::from_path(PathBuf::from("-")).is_stdout());
        assert!(!Destination::from_path(PathBuf::from("out.json")).is_stdout());
    }

    #[test]
    fn test_output_config_default() {
        let config = OutputConfig::default();
        assert!(config.is_empty());
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].format, Format::Json);
        assert_eq!(specs[0].dest, Destination::Stdout);
    }

    #[test]
    fn test_single_format_flag_json() {
        let mut config = OutputConfig::default();
        config.json = vec![PathBuf::from("out.json")];
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].format, Format::Json);
        assert!(matches!(specs[0].dest, Destination::File(_)));
    }

    #[test]
    fn test_single_format_flag_md() {
        let mut config = OutputConfig::default();
        config.md = vec![PathBuf::from("out.md")];
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].format, Format::Markdown);
        assert!(matches!(specs[0].dest, Destination::File(_)));
    }

    #[test]
    fn test_single_format_flag_text() {
        let mut config = OutputConfig::default();
        config.text = vec![PathBuf::from("out.txt")];
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].format, Format::Text);
        assert!(matches!(specs[0].dest, Destination::File(_)));
    }

    #[test]
    fn test_ndjson_flag() {
        let mut config = OutputConfig::default();
        config.ndjson = true;
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].format, Format::Ndjson);
        assert_eq!(specs[0].dest, Destination::Stdout);
    }

    #[test]
    fn test_multiple_format_flags() {
        let mut config = OutputConfig::default();
        config.json = vec![PathBuf::from("out.json")];
        config.md = vec![PathBuf::from("out.md")];
        config.text = vec![PathBuf::from("out.txt")];
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 3);
        assert_eq!(specs[0].format, Format::Json);
        assert_eq!(specs[1].format, Format::Markdown);
        assert_eq!(specs[2].format, Format::Text);
    }

    #[test]
    fn test_stdout_with_file() {
        let mut config = OutputConfig::default();
        config.md = vec![PathBuf::from("-")];
        config.json = vec![PathBuf::from("out.json")];
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 2);
        // MD goes to stdout
        assert_eq!(specs[0].format, Format::Markdown);
        assert_eq!(specs[0].dest, Destination::Stdout);
        // JSON goes to file
        assert_eq!(specs[1].format, Format::Json);
        assert!(matches!(specs[1].dest, Destination::File(_)));
    }

    #[test]
    fn test_multiple_stdout_rejected() {
        let mut config = OutputConfig::default();
        config.md = vec![PathBuf::from("-")];
        config.json = vec![PathBuf::from("-")];
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("at most one"));
        assert!(err.to_string().contains("stdout"));
    }

    #[test]
    fn test_ndjson_exclusive_with_md() {
        let mut config = OutputConfig::default();
        config.ndjson = true;
        config.md = vec![PathBuf::from("out.md")];
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("--ndjson cannot be combined"));
    }

    #[test]
    fn test_ndjson_exclusive_with_json() {
        let mut config = OutputConfig::default();
        config.ndjson = true;
        config.json = vec![PathBuf::from("out.json")];
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("--ndjson cannot be combined"));
    }

    #[test]
    fn test_ndjson_exclusive_with_text() {
        let mut config = OutputConfig::default();
        config.ndjson = true;
        config.text = vec![PathBuf::from("out.txt")];
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("--ndjson cannot be combined"));
    }

    #[test]
    fn test_format_with_base() {
        let mut config = OutputConfig::default();
        config.format_list = vec!["json".to_string(), "markdown".to_string()];
        config.output_base = Some(PathBuf::from("out"));
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].format, Format::Json);
        assert!(
            matches!(&specs[0].dest, Destination::File(p) if p.to_str().unwrap() == "out.json")
        );
        assert_eq!(specs[1].format, Format::Markdown);
        assert!(matches!(&specs[1].dest, Destination::File(p) if p.to_str().unwrap() == "out.md"));
    }

    #[test]
    fn test_format_with_all_formats() {
        let mut config = OutputConfig::default();
        config.format_list = vec!["json".to_string(), "md".to_string(), "text".to_string()];
        config.output_base = Some(PathBuf::from("output"));
        let specs = config.build_specs().unwrap();
        assert_eq!(specs.len(), 3);
        assert!(
            matches!(&specs[0].dest, Destination::File(p) if p.to_str().unwrap() == "output.json")
        );
        assert!(
            matches!(&specs[1].dest, Destination::File(p) if p.to_str().unwrap() == "output.md")
        );
        assert!(
            matches!(&specs[2].dest, Destination::File(p) if p.to_str().unwrap() == "output.txt")
        );
    }

    #[test]
    fn test_format_without_base_error() {
        let mut config = OutputConfig::default();
        config.format_list = vec!["json".to_string()];
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("--format requires -o"));
    }

    #[test]
    fn test_duplicate_format_json_flag_and_format_list() {
        let mut config = OutputConfig::default();
        config.json = vec![PathBuf::from("a.json")];
        config.format_list = vec!["json".to_string()];
        config.output_base = Some(PathBuf::from("out"));
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("duplicate format"));
        assert!(err.to_string().contains("--json"));
        assert!(err.to_string().contains("--format"));
    }

    #[test]
    fn test_duplicate_format_md_flag_and_format_list() {
        let mut config = OutputConfig::default();
        config.md = vec![PathBuf::from("a.md")];
        config.format_list = vec!["markdown".to_string()];
        config.output_base = Some(PathBuf::from("out"));
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("duplicate format"));
    }

    #[test]
    fn test_duplicate_format_text_flag_and_format_list() {
        let mut config = OutputConfig::default();
        config.text = vec![PathBuf::from("a.txt")];
        config.format_list = vec!["text".to_string()];
        config.output_base = Some(PathBuf::from("out"));
        let err = config.build_specs().unwrap_err();
        assert!(err.to_string().contains("duplicate format"));
    }

    #[test]
    fn test_output_spec_auto_named() {
        let base = PathBuf::from("output");
        let spec = OutputSpec::auto_named(Format::Json, &base);
        assert_eq!(spec.format, Format::Json);
        assert!(matches!(spec.dest, Destination::File(p) if p.to_str().unwrap() == "output.json"));

        let spec = OutputSpec::auto_named(Format::Markdown, &base);
        assert_eq!(spec.format, Format::Markdown);
        assert!(matches!(spec.dest, Destination::File(p) if p.to_str().unwrap() == "output.md"));

        let spec = OutputSpec::auto_named(Format::Text, &base);
        assert_eq!(spec.format, Format::Text);
        assert!(matches!(spec.dest, Destination::File(p) if p.to_str().unwrap() == "output.txt"));

        let spec = OutputSpec::auto_named(Format::Ndjson, &base);
        assert_eq!(spec.format, Format::Ndjson);
        assert!(
            matches!(spec.dest, Destination::File(p) if p.to_str().unwrap() == "output.ndjson")
        );
    }

    #[test]
    fn test_output_spec_from_path() {
        let spec = OutputSpec::from_path(Format::Json, PathBuf::from("out.json"));
        assert_eq!(spec.format, Format::Json);
        assert!(matches!(spec.dest, Destination::File(_)));

        let spec = OutputSpec::from_path(Format::Markdown, PathBuf::from("-"));
        assert_eq!(spec.format, Format::Markdown);
        assert_eq!(spec.dest, Destination::Stdout);
    }
}
