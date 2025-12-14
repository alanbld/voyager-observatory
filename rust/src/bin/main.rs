//! pm_encoder CLI - Command-line interface for the Rust engine
//!
//! This binary uses Clap for argument parsing, mirroring Python's argparse behavior.
//! All core logic lives in lib.rs, making it reusable for WASM/Python bindings.
//!
//! # Design Philosophy
//!
//! This CLI follows the "Thin Interface" pattern:
//! - Clap handles argument parsing and --help/--version
//! - Delegates to the library for all actual work
//! - Maintains interface parity with Python implementation

use clap::{Parser, ValueEnum};
use pm_encoder::{self, EncoderConfig};
use std::path::PathBuf;

/// Serialize project files into the Plus/Minus format with intelligent truncation.
#[derive(Parser, Debug)]
#[command(name = "pm_encoder")]
#[command(version = pm_encoder::VERSION)]
#[command(about = "Serialize project files into the Plus/Minus format with intelligent truncation.")]
#[command(after_help = "Examples:
  # Basic serialization
  pm_encoder . -o output.txt

  # With truncation (500 lines per file)
  pm_encoder . --truncate 500 -o output.txt

  # Apply a lens
  pm_encoder . --lens architecture
")]
struct Cli {
    // ═══════════════════════════════════════════════════════════════════════════
    // CORE ARGUMENTS
    // ═══════════════════════════════════════════════════════════════════════════

    /// The root directory of the project to serialize
    #[arg(value_name = "PROJECT_ROOT")]
    project_root: Option<PathBuf>,

    /// Output file path. Defaults to standard output.
    #[arg(short = 'o', long = "output", value_name = "OUTPUT")]
    output: Option<PathBuf>,

    /// Path to a JSON configuration file for ignore/include patterns.
    /// Defaults to ./.pm_encoder_config.json
    #[arg(short = 'c', long = "config", value_name = "CONFIG")]
    config: Option<PathBuf>,

    // ═══════════════════════════════════════════════════════════════════════════
    // FILTERING
    // ═══════════════════════════════════════════════════════════════════════════

    /// One or more glob patterns for files to include. Overrides config includes.
    #[arg(long = "include", value_name = "PATTERN", num_args = 0..)]
    include: Vec<String>,

    /// One or more glob patterns for files/dirs to exclude. Adds to config excludes.
    #[arg(long = "exclude", value_name = "PATTERN", num_args = 0..)]
    exclude: Vec<String>,

    // ═══════════════════════════════════════════════════════════════════════════
    // SORTING
    // ═══════════════════════════════════════════════════════════════════════════

    /// Sort files by 'name' (default), 'mtime' (modification time), or 'ctime' (creation time).
    #[arg(long = "sort-by", value_enum, default_value = "name")]
    sort_by: SortBy,

    /// Sort order: 'asc' (ascending, default) or 'desc' (descending).
    #[arg(long = "sort-order", value_enum, default_value = "asc")]
    sort_order: SortOrder,

    // ═══════════════════════════════════════════════════════════════════════════
    // TRUNCATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Truncate files exceeding N lines (default: 0 = no truncation)
    #[arg(long = "truncate", value_name = "N", default_value = "0")]
    truncate: usize,

    /// Truncation strategy: 'simple' (keep first N lines), 'smart' (language-aware), or 'structure' (signatures only)
    #[arg(long = "truncate-mode", value_enum, default_value = "simple")]
    truncate_mode: TruncateMode,

    /// Include analysis summary in truncation marker (default: true)
    #[arg(long = "truncate-summary", default_value = "true")]
    truncate_summary: bool,

    /// Disable truncation summary
    #[arg(long = "no-truncate-summary")]
    no_truncate_summary: bool,

    /// Never truncate files matching these patterns
    #[arg(long = "truncate-exclude", value_name = "PATTERN", num_args = 0..)]
    truncate_exclude: Vec<String>,

    /// Show detailed truncation statistics report
    #[arg(long = "truncate-stats")]
    truncate_stats: bool,

    // ═══════════════════════════════════════════════════════════════════════════
    // LENSES
    // ═══════════════════════════════════════════════════════════════════════════

    /// Apply a context lens (architecture|debug|security|onboarding|custom)
    #[arg(long = "lens", value_name = "NAME")]
    lens: Option<String>,

    // ═══════════════════════════════════════════════════════════════════════════
    // PLUGINS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Custom language analyzer plugins directory
    #[arg(long = "language-plugins", value_name = "DIR")]
    language_plugins: Option<PathBuf>,

    /// Generate a plugin template for LANGUAGE and exit
    #[arg(long = "create-plugin", value_name = "LANGUAGE")]
    create_plugin: Option<String>,

    /// Generate an AI prompt to create a plugin for LANGUAGE and exit
    #[arg(long = "plugin-prompt", value_name = "LANGUAGE")]
    plugin_prompt: Option<String>,

    // ═══════════════════════════════════════════════════════════════════════════
    // INIT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Generate instruction file and CONTEXT.txt for AI CLI integration and exit
    #[arg(long = "init-prompt")]
    init_prompt: bool,

    /// Lens to use with --init-prompt (default: architecture)
    #[arg(long = "init-lens", value_name = "LENS", default_value = "architecture")]
    init_lens: String,

    /// Target AI for --init-prompt: 'claude' (CLAUDE.md) or 'gemini' (GEMINI_INSTRUCTIONS.txt)
    #[arg(long = "target", value_enum, default_value = "claude")]
    target: TargetAI,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SortBy {
    Name,
    Mtime,
    Ctime,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TruncateMode {
    Simple,
    Smart,
    Structure,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TargetAI {
    Claude,
    Gemini,
}

fn main() {
    let cli = Cli::parse();

    // If no project root provided, show usage
    let project_root = match cli.project_root {
        Some(path) => path,
        None => {
            eprintln!("Error: PROJECT_ROOT argument is required");
            eprintln!("Usage: pm_encoder <PROJECT_ROOT>");
            eprintln!("\nTry 'pm_encoder --help' for more information.");
            std::process::exit(1);
        }
    };

    // Validate project root exists
    if !project_root.exists() {
        eprintln!("Error: Path '{}' does not exist", project_root.display());
        std::process::exit(1);
    }

    if !project_root.is_dir() {
        eprintln!("Error: Path '{}' is not a directory", project_root.display());
        std::process::exit(1);
    }

    // Build config from CLI args
    let mut config = if let Some(config_path) = cli.config {
        match EncoderConfig::from_file(&config_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Warning: Could not load config file: {}", e);
                EncoderConfig::default()
            }
        }
    } else {
        // Try default config path
        let default_config = project_root.join(".pm_encoder_config.json");
        if default_config.exists() {
            EncoderConfig::from_file(&default_config).unwrap_or_default()
        } else {
            EncoderConfig::default()
        }
    };

    // Apply CLI overrides
    if !cli.include.is_empty() {
        config.include_patterns = cli.include;
    }

    if !cli.exclude.is_empty() {
        config.ignore_patterns.extend(cli.exclude);
    }

    config.sort_by = match cli.sort_by {
        SortBy::Name => "name".to_string(),
        SortBy::Mtime => "mtime".to_string(),
        SortBy::Ctime => "ctime".to_string(),
    };

    config.sort_order = match cli.sort_order {
        SortOrder::Asc => "asc".to_string(),
        SortOrder::Desc => "desc".to_string(),
    };

    // Serialize the project
    match pm_encoder::serialize_project_with_config(project_root.to_str().unwrap(), &config) {
        Ok(output) => {
            // Write to file or stdout
            if let Some(output_path) = cli.output {
                match std::fs::write(&output_path, &output) {
                    Ok(_) => {
                        eprintln!("Output written to: {}", output_path.display());
                    }
                    Err(e) => {
                        eprintln!("Error writing output: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                print!("{}", output);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
