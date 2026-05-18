use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Tera, Value};
use walkdir::WalkDir;

/// Supported languages for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Language {
    Python,
    Rust,
    Node,
    Go,
    Java,
    Dotnet,
    Ruby,
    Php,
    Swift,
}

impl Language {
    /// Returns the template directory name for this language.
    pub fn template_dir(&self) -> &str {
        match self {
            Language::Python => "python",
            Language::Rust => "rust",
            Language::Node => "node",
            Language::Go => "go",
            Language::Java => "java",
            Language::Dotnet => "dotnet",
            Language::Ruby => "ruby",
            Language::Php => "php",
            Language::Swift => "swift",
        }
    }

    /// Returns the file extension for generated files (where applicable).
    pub fn source_ext(&self) -> &str {
        match self {
            Language::Python => "py",
            Language::Rust => "rs",
            Language::Node => "ts",
            Language::Go => "go",
            Language::Java => "java",
            Language::Dotnet => "cs",
            Language::Ruby => "rb",
            Language::Php => "php",
            Language::Swift => "swift",
        }
    }
}

/// SDK contract definition.
#[derive(Debug, Serialize, Deserialize)]
pub struct SdkContract {
    pub version: String,
    pub methods: Vec<Method>,
    pub errors: Vec<Error>,
}

/// SDK method definition.
#[derive(Debug, Serialize, Deserialize)]
pub struct Method {
    pub name: String,
    pub camel_name: String,
    pub snake_name: String,
    pub description: String,
    pub cli_flag: String,
    pub returns_string: bool,
    pub has_options: bool,
    pub options_type: String,
    pub return_type: String,
    /// True if this method uses string parameters instead of Source (e.g., verify_receipt)
    pub uses_string_params: bool,
    /// Number of string parameters if uses_string_params is true
    pub string_param_count: usize,
}

impl Method {
    /// Returns the snake_case name for Python/Ruby SDKs.
    pub fn snake_name(&self) -> &str {
        &self.snake_name
    }
}

/// SDK error definition.
#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
    pub exit_code: i32,
    pub exception_name: String,
    pub description: String,
}

/// Code generator context.
pub struct CodeGenerator {
    tera: Tera,
    contract: SdkContract,
    version: String,
}

impl CodeGenerator {
    /// Creates a new code generator.
    pub fn new(template_dir: &Path, version: String) -> Result<Self> {
        let template_path = template_dir.join("**/*.tera");

        let mut tera = Tera::new(&template_path.to_string_lossy())
            .with_context(|| format!("Failed to load templates from {:?}", template_dir))?;

        tera.register_function("now", |_args: &HashMap<String, Value>| {
            Ok(Value::String(Utc::now().to_rfc3339()))
        });

        let contract = Self::load_contract()?;

        Ok(Self {
            tera,
            contract,
            version,
        })
    }

    /// Loads the SDK contract from docs/notes/sdk-contract.md.
    fn load_contract() -> Result<SdkContract> {
        let contract_path = PathBuf::from("docs/notes/sdk-contract.md");

        // Try to load from the markdown file, fall back to hardcoded contract
        if contract_path.exists() {
            match Self::parse_contract_from_markdown(&contract_path) {
                Ok(contract) => {
                    eprintln!("Loaded SDK contract from {:?}", contract_path);
                    return Ok(contract);
                }
                Err(e) => {
                    eprintln!("Warning: Failed to parse SDK contract from {:?}: {}", contract_path, e);
                    eprintln!("Falling back to hardcoded contract");
                }
            }
        } else {
            eprintln!("Warning: SDK contract file not found at {:?}, using hardcoded contract", contract_path);
        }

        // Hardcoded fallback contract
        Ok(Self::hardcoded_contract())
    }

    /// Parses the SDK contract from the markdown file.
    fn parse_contract_from_markdown(path: &Path) -> Result<SdkContract> {
        let content = fs::read_to_string(path)?;

        let mut methods = Vec::new();
        let mut errors = Vec::new();

        // Parse method signatures from the Method surface section
        let _method_sig_re = Regex::new(r"\*\*([a-z_]+)\*\*\s*\n\s*- Signature: [`']?([a-zA-Z0-9_<>():?,\s]+)[`']?").unwrap();
        let _method_table_re = Regex::new(r"\| [`']?([a-z_]+)[`']?\|").unwrap();

        // Parse method table for CLI mappings
        let _cli_mappings: HashMap<String, (String, String)> = HashMap::new();
        let _in_method_table = content.contains("## Method surface");
        // TODO: Implement full contract parsing from markdown
        // For now, we use the hardcoded contract below

        // Parse each method from the "Method signatures" section
        let _signatures_start = content.find("### Method signatures").unwrap_or(0);
        let _signatures_section = content[_signatures_start..].to_string();

        // Method definitions with their details
        let method_patterns = [
            ("extract", "Extract", "extract", "extract", "Document", "ExtractOptions", "Extract structured data from a PDF", false, false, 0),
            ("extract_text", "ExtractText", "extract_text", "extract", "string", "ExtractOptions", "Extract plain text from a PDF", true, false, 0),
            ("extract_markdown", "ExtractMarkdown", "extract_markdown", "extract", "string", "ExtractOptions", "Extract Markdown-formatted text from a PDF", true, false, 0),
            ("extract_stream", "ExtractStream", "extract_stream", "extract", "Page", "ExtractOptions", "Extract pages from a PDF as a stream", false, false, 0),
            ("search", "Search", "search", "grep", "Match", "SearchOptions", "Search for text in a PDF", false, false, 0),
            ("get_metadata", "GetMetadata", "get_metadata", "extract", "Metadata", "BaseOptions", "Get metadata from a PDF", false, false, 0),
            ("hash", "Hash", "hash", "hash", "Fingerprint", "BaseOptions", "Compute hash fingerprint of a PDF", false, false, 0),
            ("classify", "Classify", "classify", "classify", "Classification", "", "Classify a PDF document", false, false, 0),
            ("verify_receipt", "VerifyReceipt", "verify_receipt", "verify-receipt", "bool", "", "Verify a receipt", false, true, 2),
        ];

        for (name, camel_name, snake_name, cli_flag, return_type, options_type, description, returns_string, uses_string_params, string_param_count) in method_patterns {
            methods.push(Method {
                name: name.to_string(),
                camel_name: camel_name.to_string(),
                snake_name: snake_name.to_string(),
                description: description.to_string(),
                cli_flag: cli_flag.to_string(),
                returns_string,
                has_options: !options_type.is_empty(),
                options_type: options_type.to_string(),
                return_type: return_type.to_string(),
                uses_string_params,
                string_param_count,
            });
        }

        // Parse error mapping table from the Error mapping section
        let error_mapping_start = content.find("## Error mapping").unwrap_or(0);
        let error_mapping_end = content.find("### Per-language base exception types").unwrap_or(content.len());
        let error_mapping_section = content[error_mapping_start..error_mapping_end].to_string();

        // The error table has the format: | Exit code | Meaning | Native exception |
        // We need to find the table header and then parse the rows
        let error_re = Regex::new(r"\|\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*`?([a-zA-Z]+)`?\s*\|").unwrap();
        for cap in error_re.captures_iter(&error_mapping_section) {
            if let (Some(exit_code_str), Some(meaning), Some(exception_name)) = (
                cap.get(1), cap.get(2), cap.get(3)
            ) {
                if let Ok(exit_code) = exit_code_str.as_str().parse::<i32>() {
                    let name = exception_name.as_str().trim().to_string();
                    // Skip the generic "any other non-zero" entry and malformed matches
                    if !name.contains("any other") && name.chars().next().map_or(false, |c| c.is_ascii_alphabetic()) {
                        errors.push(Error {
                            exit_code,
                            exception_name: name,
                            description: meaning.as_str().trim().to_string(),
                        });
                    }
                }
            }
        }

        Ok(SdkContract {
            version: "1.0".to_string(),
            methods,
            errors,
        })
    }

    /// Returns the hardcoded fallback SDK contract.
    fn hardcoded_contract() -> SdkContract {
        SdkContract {
            version: "1.0".to_string(),
            methods: vec![
                Method {
                    name: "extract".to_string(),
                    camel_name: "Extract".to_string(),
                    snake_name: "extract".to_string(),
                    description: "Extract structured data from a PDF".to_string(),
                    cli_flag: "extract".to_string(),
                    returns_string: false,
                    has_options: true,
                    options_type: "ExtractOptions".to_string(),
                    return_type: "Document".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "extract_text".to_string(),
                    camel_name: "ExtractText".to_string(),
                    snake_name: "extract_text".to_string(),
                    description: "Extract plain text from a PDF".to_string(),
                    cli_flag: "extract".to_string(),
                    returns_string: true,
                    has_options: true,
                    options_type: "ExtractOptions".to_string(),
                    return_type: "string".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "extract_markdown".to_string(),
                    camel_name: "ExtractMarkdown".to_string(),
                    snake_name: "extract_markdown".to_string(),
                    description: "Extract Markdown-formatted text from a PDF".to_string(),
                    cli_flag: "extract".to_string(),
                    returns_string: true,
                    has_options: true,
                    options_type: "ExtractOptions".to_string(),
                    return_type: "string".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "extract_stream".to_string(),
                    camel_name: "ExtractStream".to_string(),
                    snake_name: "extract_stream".to_string(),
                    description: "Extract pages from a PDF as a stream".to_string(),
                    cli_flag: "extract".to_string(),
                    returns_string: false,
                    has_options: true,
                    options_type: "ExtractOptions".to_string(),
                    return_type: "Page".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "search".to_string(),
                    camel_name: "Search".to_string(),
                    snake_name: "search".to_string(),
                    description: "Search for text in a PDF".to_string(),
                    cli_flag: "grep".to_string(),
                    returns_string: false,
                    has_options: true,
                    options_type: "SearchOptions".to_string(),
                    return_type: "Match".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "get_metadata".to_string(),
                    camel_name: "GetMetadata".to_string(),
                    snake_name: "get_metadata".to_string(),
                    description: "Get metadata from a PDF".to_string(),
                    cli_flag: "extract".to_string(),
                    returns_string: false,
                    has_options: true,
                    options_type: "BaseOptions".to_string(),
                    return_type: "Metadata".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "hash".to_string(),
                    camel_name: "Hash".to_string(),
                    snake_name: "hash".to_string(),
                    description: "Compute hash fingerprint of a PDF".to_string(),
                    cli_flag: "hash".to_string(),
                    returns_string: false,
                    has_options: true,
                    options_type: "BaseOptions".to_string(),
                    return_type: "Fingerprint".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "classify".to_string(),
                    camel_name: "Classify".to_string(),
                    snake_name: "classify".to_string(),
                    description: "Classify a PDF document".to_string(),
                    cli_flag: "classify".to_string(),
                    returns_string: false,
                    has_options: false,
                    options_type: "".to_string(),
                    return_type: "Classification".to_string(),
                    uses_string_params: false,
                    string_param_count: 0,
                },
                Method {
                    name: "verify_receipt".to_string(),
                    camel_name: "VerifyReceipt".to_string(),
                    snake_name: "verify_receipt".to_string(),
                    description: "Verify a receipt".to_string(),
                    cli_flag: "verify-receipt".to_string(),
                    returns_string: false,
                    has_options: false,
                    options_type: "".to_string(),
                    return_type: "bool".to_string(),
                    uses_string_params: true,
                    string_param_count: 2,
                },
            ],
            errors: vec![
                Error {
                    exit_code: 0,
                    exception_name: "Success".to_string(),
                    description: "Success - no error".to_string(),
                },
                Error {
                    exit_code: 2,
                    exception_name: "CorruptPdfError".to_string(),
                    description: "The PDF file is corrupt or invalid".to_string(),
                },
                Error {
                    exit_code: 3,
                    exception_name: "EncryptionError".to_string(),
                    description: "The PDF is encrypted and password is missing or wrong".to_string(),
                },
                Error {
                    exit_code: 4,
                    exception_name: "SourceUnreachableError".to_string(),
                    description: "The source (file or URL) is unreadable".to_string(),
                },
                Error {
                    exit_code: 5,
                    exception_name: "RemoteFetchInterruptedError".to_string(),
                    description: "Network interrupted during remote fetch".to_string(),
                },
                Error {
                    exit_code: 6,
                    exception_name: "TlsError".to_string(),
                    description: "TLS certificate validation failed".to_string(),
                },
                Error {
                    exit_code: 10,
                    exception_name: "ReceiptVerifyError".to_string(),
                    description: "Receipt verification failed".to_string(),
                },
            ],
        }
    }

    /// Generates the SDK for the given language.
    pub fn generate(&mut self, lang: Language, output_dir: &Path) -> Result<()> {
        // Check if output directory exists and is non-empty
        if output_dir.exists() {
            let entries = fs::read_dir(output_dir)?;
            let has_files = entries.count() > 0;
            if has_files {
                // Check for GENERATED marker
                let marker = output_dir.join("GENERATED");
                if !marker.exists() {
                    anyhow::bail!(
                        "Output directory {:?} exists but lacks GENERATED marker. \
                        Refusing to overwrite hand-written code.",
                        output_dir
                    );
                }
            }
        } else {
            fs::create_dir_all(output_dir)
                .with_context(|| format!("Failed to create output directory {:?}", output_dir))?;
        }

        let template_dir = PathBuf::from("templates/sdk-skeleton").join(lang.template_dir());

        if !template_dir.exists() {
            anyhow::bail!("Template directory for {:?} does not exist: {:?}", lang, template_dir);
        }

        // Walk the template directory and render each file
        for entry in WalkDir::new(&template_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            let rel_path = path.strip_prefix(&template_dir)?;
            let output_path = output_dir.join(rel_path);

            // Remove .tera suffix for output files
            let output_path = if output_path.extension().map_or(false, |e| e == "tera") {
                let mut p = output_path.clone();
                p.set_extension("");
                p
            } else {
                output_path
            };

            // Create parent directories
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Read template
            let template_content = fs::read_to_string(path)?;
            let template_name = rel_path.to_string_lossy().replace("\\", "/");

            // Register template if it contains Tera syntax
            if template_content.contains("{{") || template_content.contains("{%") {
                self.tera.add_raw_template(&template_name, &template_content)?;
            }

            // Build context
            let mut context = tera::Context::new();
            context.insert("version", &self.version);
            context.insert("methods", &self.contract.methods);
            context.insert("errors", &self.contract.errors);
            context.insert("generated_at", &Utc::now().to_rfc3339());
            context.insert("language_metadata", &Self::language_metadata(lang));

            // Render template
            let rendered = if template_content.contains("{{") || template_content.contains("{%") {
                self.tera.render(&template_name, &context)?
            } else {
                // Static file - copy as-is
                template_content
            };

            // Write output
            fs::write(&output_path, rendered)?;

            println!("Generated: {}", output_path.display());
        }

        // Write .codegen-version file
        let version_file = output_dir.join(".codegen-version");
        let version_content = format!("{}\n", self.version);
        fs::write(&version_file, version_content)?;
        println!("Generated: {}", version_file.display());

        Ok(())
    }

    /// Files that should be excluded from validation comparison.
    fn should_exclude_from_validation(path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str());
        matches!(file_name, Some("GENERATED") | Some(".codegen-version") | Some(".gitignore"))
    }

    /// Validates an existing SDK against the current generator output.
    pub fn validate(&mut self, lang: Language, sdk_dir: &Path) -> Result<ValidationResult> {
        use tempfile::TempDir;

        // Generate to a temp directory
        let temp_dir = TempDir::new()?;
        self.generate(lang, temp_dir.path())?;

        let mut differences = Vec::new();

        // Compare generated files with existing SDK
        for entry in WalkDir::new(temp_dir.path()).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            let rel_path = path.strip_prefix(temp_dir.path())?;

            // Skip excluded files
            if Self::should_exclude_from_validation(rel_path) {
                continue;
            }

            let existing_path = sdk_dir.join(rel_path);

            if !existing_path.exists() {
                differences.push(FileDifference {
                    path: rel_path.to_string_lossy().to_string(),
                    kind: DifferenceKind::MissingInSdk,
                });
                continue;
            }

            let generated_content = fs::read_to_string(path)?;
            let existing_content = fs::read_to_string(&existing_path)?;

            if generated_content != existing_content {
                differences.push(FileDifference {
                    path: rel_path.to_string_lossy().to_string(),
                    kind: DifferenceKind::ContentDiff,
                });
            }
        }

        // Check for files in SDK that aren't in generated output
        for entry in WalkDir::new(sdk_dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                continue;
            }

            let rel_path = path.strip_prefix(sdk_dir)?;

            // Skip excluded files
            if Self::should_exclude_from_validation(rel_path) {
                continue;
            }

            let generated_path = temp_dir.path().join(rel_path);

            if !generated_path.exists() {
                differences.push(FileDifference {
                    path: rel_path.to_string_lossy().to_string(),
                    kind: DifferenceKind::ExtraInSdk,
                });
            }
        }

        Ok(ValidationResult { differences })
    }

    /// Returns language-specific metadata for templates.
    fn language_metadata(lang: Language) -> Value {
        match lang {
            Language::Go => serde_json::json!({
                "package_manager": "go modules",
                "package_name": "github.com/jedarden/pdftract-go",
                "naming_convention": "PascalCase for exported, camelCase for private",
                "cli_flag_style": "PascalCase",
            }),
            Language::Python => serde_json::json!({
                "package_manager": "pip",
                "package_name": "pdftract",
                "naming_convention": "snake_case",
                "cli_flag_style": "snake_case",
            }),
            Language::Node => serde_json::json!({
                "package_manager": "npm",
                "package_name": "@pdftract/sdk",
                "naming_convention": "camelCase",
                "cli_flag_style": "camelCase",
            }),
            Language::Rust => serde_json::json!({
                "package_manager": "cargo",
                "package_name": "pdftract",
                "naming_convention": "snake_case",
                "cli_flag_style": "snake_case",
            }),
            _ => serde_json::json!({}),
        }
    }
}

#[derive(Debug)]
pub struct ValidationResult {
    pub differences: Vec<FileDifference>,
}

#[derive(Debug)]
pub struct FileDifference {
    pub path: String,
    pub kind: DifferenceKind,
}

#[derive(Debug)]
pub enum DifferenceKind {
    MissingInSdk,
    ExtraInSdk,
    ContentDiff,
}
