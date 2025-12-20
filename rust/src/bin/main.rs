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
use pm_encoder::{self, EncoderConfig, LensManager, OutputFormat, parse_token_budget, apply_token_budget};
use pm_encoder::core::{ContextEngine, ZoomConfig, ZoomTarget};
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
    // SORTING & STREAMING
    // ═══════════════════════════════════════════════════════════════════════════

    /// Sort files by 'name' (default), 'mtime' (modification time), or 'ctime' (creation time).
    #[arg(long = "sort-by", value_enum, default_value = "name")]
    sort_by: SortBy,

    /// Sort order: 'asc' (ascending, default) or 'desc' (descending).
    #[arg(long = "sort-order", value_enum, default_value = "asc")]
    sort_order: SortOrder,

    /// Enable streaming mode: output files immediately as they're found.
    /// Disables global sorting for lower Time-To-First-Byte (TTFB).
    #[arg(long = "stream")]
    stream: bool,

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
    // TOKEN BUDGETING (v0.7.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Maximum token budget (e.g., 100000, 100k, 2M). Files are included by priority until budget is reached.
    #[arg(long = "token-budget", value_name = "BUDGET")]
    token_budget: Option<String>,

    /// Budget enforcement strategy: 'drop' (skip files), 'truncate' (force structure mode), or 'hybrid' (auto-truncate large files)
    #[arg(long = "budget-strategy", value_enum, default_value = "drop")]
    budget_strategy: BudgetStrategy,

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

    // ═══════════════════════════════════════════════════════════════════════════
    // OUTPUT FORMAT (v0.10.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Output format: 'plus_minus' (default), 'xml', 'markdown', or 'claude-xml'
    #[arg(long = "format", value_enum, default_value = "plus-minus")]
    format: OutputFormatArg,

    // ═══════════════════════════════════════════════════════════════════════════
    // DETERMINISM & PRIVACY (v2.0.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Frozen mode: bypass context store for deterministic output.
    /// Enables byte-identical output for CI/CD pipelines and reproducible tests.
    #[arg(long = "frozen")]
    frozen: bool,

    /// Allow sensitive metadata in output (session notes, absolute paths).
    /// Default behavior excludes PII for privacy protection.
    #[arg(long = "allow-sensitive")]
    allow_sensitive: bool,

    // ═══════════════════════════════════════════════════════════════════════════
    // ZOOM / FRACTAL PROTOCOL (v2.0.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Zoom into a specific target for detailed context.
    /// Format: <TYPE>=<TARGET>
    /// Types:
    ///   fn=<name>           - Zoom to function definition
    ///   class=<name>        - Zoom to class/struct definition
    ///   mod=<name>          - Zoom to module
    ///   file=<path>         - Zoom to entire file
    ///   file=<path>:L1-L2   - Zoom to file lines L1 to L2
    #[arg(long = "zoom", value_name = "TARGET")]
    zoom: Option<String>,

    /// Show Context Health summary after serialization
    #[arg(long = "health")]
    health: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormatArg {
    #[value(name = "plus-minus", alias = "pm")]
    PlusMinus,
    Xml,
    #[value(alias = "md")]
    Markdown,
    /// Claude-optimized XML with CDATA sections and semantic attributes
    #[value(name = "claude-xml")]
    ClaudeXml,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
enum BudgetStrategy {
    Drop,
    Truncate,
    Hybrid,
}

/// Parse a zoom target string into ZoomConfig.
/// Formats:
///   fn=<name>           - Zoom to function
///   class=<name>        - Zoom to class/struct
///   mod=<name>          - Zoom to module
///   file=<path>         - Zoom to entire file
///   file=<path>:L1-L2   - Zoom to file lines L1 to L2
fn parse_zoom_target(s: &str) -> Result<ZoomConfig, String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid zoom format: '{}'. Expected <TYPE>=<TARGET> (e.g., fn=main, file=src/lib.rs:10-50)",
            s
        ));
    }

    let zoom_type = parts[0].to_lowercase();
    let target_str = parts[1];

    let target = match zoom_type.as_str() {
        "fn" | "function" => ZoomTarget::Function(target_str.to_string()),
        "class" | "struct" => ZoomTarget::Class(target_str.to_string()),
        "mod" | "module" => ZoomTarget::Module(target_str.to_string()),
        "file" => {
            // Check for line range: file=path:L1-L2
            if let Some(colon_pos) = target_str.rfind(':') {
                let path = &target_str[..colon_pos];
                let range = &target_str[colon_pos + 1..];

                // Parse line range (e.g., "10-50" or "10")
                let (start, end) = if let Some(dash_pos) = range.find('-') {
                    let start: usize = range[..dash_pos]
                        .parse()
                        .map_err(|_| format!("Invalid start line in range: '{}'", range))?;
                    let end: usize = range[dash_pos + 1..]
                        .parse()
                        .map_err(|_| format!("Invalid end line in range: '{}'", range))?;
                    (Some(start), Some(end))
                } else {
                    // Single line number means start from that line
                    let line: usize = range
                        .parse()
                        .map_err(|_| format!("Invalid line number: '{}'", range))?;
                    (Some(line), None)
                };

                ZoomTarget::File {
                    path: path.to_string(),
                    start_line: start,
                    end_line: end,
                }
            } else {
                // No line range, zoom to entire file
                ZoomTarget::File {
                    path: target_str.to_string(),
                    start_line: None,
                    end_line: None,
                }
            }
        }
        _ => {
            return Err(format!(
                "Unknown zoom type: '{}'. Valid types: fn, class, mod, file",
                zoom_type
            ));
        }
    };

    Ok(ZoomConfig {
        target,
        budget: None,
        depth: pm_encoder::core::ZoomDepth::Full,
        include_tests: false,
        context_lines: 5,
    })
}

/// Print Context Health summary to stderr
fn print_context_health(output: &str, file_count: usize) {
    // Calculate total tokens (rough estimate: 4 chars per token)
    let total_tokens = output.len() / 4;

    // Count zoom affordances
    let zoom_count = output.matches("ZOOM_AFFORDANCE").count();

    // Estimate content tokens (exclude markers and metadata)
    // Content is roughly the actual file content vs formatting overhead
    let marker_overhead = output.matches("+++++++++").count() * 20 +
                         output.matches("---------").count() * 20 +
                         output.matches("TRUNCATED").count() * 50 +
                         output.matches("<file").count() * 30 +
                         output.matches("</file>").count() * 10;
    let content_tokens = total_tokens.saturating_sub(marker_overhead / 4);

    // Token efficiency (content / total)
    let efficiency = if total_tokens > 0 {
        (content_tokens as f64 / total_tokens as f64 * 100.0).round() as u32
    } else {
        100
    };

    // Zoom density (affordances per file)
    let zoom_density = if file_count > 0 {
        zoom_count as f64 / file_count as f64
    } else {
        0.0
    };

    eprintln!();
    eprintln!("=== Context Health ===");
    eprintln!("  Files:            {}", file_count);
    eprintln!("  Total Tokens:     ~{}", total_tokens);
    eprintln!("  Token Efficiency: {}%", efficiency);
    eprintln!("  Zoom Affordances: {}", zoom_count);
    if zoom_count > 0 {
        eprintln!("  Zoom Density:     {:.2} per file", zoom_density);
    }
    eprintln!("======================");
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

    config.stream = cli.stream;

    // Apply truncation settings
    config.truncate_lines = cli.truncate;
    config.truncate_mode = match cli.truncate_mode {
        TruncateMode::Simple => "simple".to_string(),
        TruncateMode::Smart => "smart".to_string(),
        TruncateMode::Structure => "structure".to_string(),
    };
    config.truncate_summary = cli.truncate_summary && !cli.no_truncate_summary;
    config.truncate_exclude = cli.truncate_exclude.clone();
    config.truncate_stats = cli.truncate_stats;

    // Apply output format
    config.output_format = match cli.format {
        OutputFormatArg::PlusMinus => OutputFormat::PlusMinus,
        OutputFormatArg::Xml => OutputFormat::Xml,
        OutputFormatArg::Markdown => OutputFormat::Markdown,
        OutputFormatArg::ClaudeXml => OutputFormat::ClaudeXml,
    };

    // Apply determinism and privacy settings (v2.0.0)
    config.frozen = cli.frozen;
    config.allow_sensitive = cli.allow_sensitive;
    config.active_lens = cli.lens.clone();

    // Streaming mode warning for file output
    if cli.stream && cli.output.is_some() {
        eprintln!("Warning: --stream mode writes directly to stdout, ignoring -o/--output");
    }

    // Zoom mode (v2.0.0) - Fractal Protocol targeted context expansion
    if let Some(zoom_str) = &cli.zoom {
        let zoom_config = match parse_zoom_target(zoom_str) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        // Build engine with current config
        let engine = ContextEngine::with_config(pm_encoder::core::EncoderConfig {
            ignore_patterns: config.ignore_patterns.clone(),
            include_patterns: config.include_patterns.clone(),
            max_file_size: config.max_file_size,
            truncate_lines: config.truncate_lines,
            truncate_mode: config.truncate_mode.clone(),
            sort_by: config.sort_by.clone(),
            sort_order: config.sort_order.clone(),
            stream: config.stream,
            truncate_summary: config.truncate_summary,
            truncate_exclude: config.truncate_exclude.clone(),
            truncate_stats: config.truncate_stats,
            output_format: match config.output_format {
                OutputFormat::PlusMinus => pm_encoder::core::OutputFormat::PlusMinus,
                OutputFormat::Xml => pm_encoder::core::OutputFormat::Xml,
                OutputFormat::Markdown => pm_encoder::core::OutputFormat::Markdown,
                OutputFormat::ClaudeXml => pm_encoder::core::OutputFormat::ClaudeXml,
            },
            frozen: config.frozen,
            allow_sensitive: config.allow_sensitive,
            active_lens: config.active_lens.clone(),
            token_budget: config.token_budget,
        });

        match engine.zoom(project_root.to_str().unwrap(), &zoom_config) {
            Ok(output) => {
                if let Some(output_path) = cli.output {
                    match std::fs::write(&output_path, &output) {
                        Ok(_) => eprintln!("Zoom output written to: {}", output_path.display()),
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
                eprintln!("Zoom error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Init-prompt mode (v0.9.0) - Generate CLAUDE.md/GEMINI_INSTRUCTIONS.txt + CONTEXT.txt
    if cli.init_prompt {
        let target_str = match cli.target {
            TargetAI::Claude => "claude",
            TargetAI::Gemini => "gemini",
        };

        match pm_encoder::init::init_prompt(
            project_root.to_str().unwrap(),
            &cli.init_lens,
            target_str,
        ) {
            Ok((instruction_path, context_path)) => {
                eprintln!("Generated: {}", instruction_path);
                eprintln!("Generated: {}", context_path);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    // Token budgeting mode (v0.7.0)
    if let Some(budget_str) = &cli.token_budget {
        // Parse budget
        let budget = match parse_token_budget(budget_str) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        // Store token budget in config for metadata injection (v2.0.0)
        config.token_budget = Some(budget);

        // Budgeting requires batch mode
        if cli.stream {
            eprintln!("Warning: --token-budget requires batch mode, ignoring --stream");
        }

        // Get lens manager for priority resolution
        let mut lens_manager = LensManager::new();

        // Apply CLI lens if present (for priority groups)
        if let Some(lens_name) = &cli.lens {
            // Store active lens for metadata injection (v2.0.0)
            config.active_lens = Some(lens_name.clone());

            match lens_manager.apply_lens(lens_name) {
                Ok(applied) => {
                    // Merge lens patterns into config
                    config.ignore_patterns.extend(applied.ignore_patterns);
                    if !applied.include_patterns.is_empty() {
                        config.include_patterns = applied.include_patterns;
                    }
                    eprintln!("[LENS: {}] Priority groups active", lens_name);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // Walk directory and collect files
        let entries = match pm_encoder::walk_directory(
            project_root.to_str().unwrap(),
            &config.ignore_patterns,
            &config.include_patterns,
            config.max_file_size,
        ) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        // Convert to (path, content) tuples
        let files: Vec<(String, String)> = entries
            .into_iter()
            .map(|e| (e.path, e.content))
            .collect();

        // Apply token budget
        let strategy_str = match cli.budget_strategy {
            BudgetStrategy::Drop => "drop",
            BudgetStrategy::Truncate => "truncate",
            BudgetStrategy::Hybrid => "hybrid",
        };
        let (selected, report) = apply_token_budget(files, budget, &lens_manager, strategy_str);

        // Print budget report to stderr
        report.print_report();

        // Build file entries for serialization
        let entries: Vec<pm_encoder::FileEntry> = selected
            .iter()
            .map(|(path, content)| pm_encoder::FileEntry {
                path: path.clone(),
                content: content.clone(),
                md5: pm_encoder::calculate_md5(content),
                mtime: 0,
                ctime: 0,
            })
            .collect();

        // Serialize selected files with configured format and truncation
        let output = if config.output_format == OutputFormat::ClaudeXml {
            // Use streaming XmlWriter for ClaudeXml format (Fractal Protocol v2.0)
            pm_encoder::serialize_entries_claude_xml(&config, &entries)
                .unwrap_or_else(|e| {
                    eprintln!("Error serializing XML: {}", e);
                    std::process::exit(1);
                })
        } else {
            // Use standard serialization for other formats
            let mut output = String::new();
            for entry in &entries {
                output.push_str(&pm_encoder::serialize_file_with_format(
                    entry,
                    config.truncate_lines,
                    &config.truncate_mode,
                    config.output_format,
                ));
            }
            output
        };

        // Write output
        if let Some(output_path) = cli.output.clone() {
            match std::fs::write(&output_path, &output) {
                Ok(_) => eprintln!("Output written to: {}", output_path.display()),
                Err(e) => {
                    eprintln!("Error writing output: {}", e);
                    std::process::exit(1);
                }
            }
        } else {
            print!("{}", output);
        }

        // Print Context Health if requested
        if cli.health {
            print_context_health(&output, entries.len());
        }
        return;
    }

    // Serialize the project (non-budgeted mode)
    match pm_encoder::serialize_project_with_config(project_root.to_str().unwrap(), &config) {
        Ok(output) => {
            // In streaming mode, output was already written directly to stdout
            if cli.stream {
                // Nothing more to do - streaming already wrote to stdout
                return;
            }

            // Batch mode: write to file or stdout
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

            // Print Context Health if requested
            if cli.health {
                // Count files in output (each file starts with "++++++++++ ")
                let file_count = output.matches("++++++++++ ").count();
                print_context_health(&output, file_count);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
