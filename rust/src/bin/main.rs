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
use pm_encoder::core::{ContextEngine, ZoomConfig, ZoomTarget, ContextStore, DEFAULT_ALPHA};
use pm_encoder::server::McpServer;
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

    // ═══════════════════════════════════════════════════════════════════════════
    // FRACTAL PROTOCOL v2: ZOOM SESSIONS (v1.1.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Manage zoom sessions. Actions: create, load, list, delete, show
    /// Examples:
    ///   --zoom-session create:debug-investigation
    ///   --zoom-session load:my-session
    ///   --zoom-session list
    ///   --zoom-session delete:old-session
    ///   --zoom-session show
    #[arg(long = "zoom-session", value_name = "ACTION:NAME")]
    zoom_session: Option<String>,

    /// Collapse a zoomed target back to structure view.
    /// Opposite of --zoom (bidirectional zoom).
    /// Example: --zoom-collapse function=main
    #[arg(long = "zoom-collapse", value_name = "TARGET")]
    zoom_collapse: Option<String>,

    /// Undo the last zoom action in the active session
    #[arg(long = "zoom-undo")]
    zoom_undo: bool,

    /// Redo the last undone zoom action in the active session
    #[arg(long = "zoom-redo")]
    zoom_redo: bool,

    /// Show Context Health summary after serialization
    #[arg(long = "health")]
    health: bool,

    // ═══════════════════════════════════════════════════════════════════════════
    // CONTEXT STORE / LEARNING (v2.2.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Report utility for a file to train the learning system.
    /// Format: "path/to/file:score:reason" where score is 0.0-1.0.
    /// Example: --report-utility "src/lib.rs:0.95:core logic"
    #[arg(long = "report-utility", value_name = "FILE:SCORE:REASON")]
    report_utility: Option<String>,

    /// Enable privacy hashing for context store paths.
    /// When enabled, file paths are hashed before storing.
    #[arg(long = "store-privacy")]
    store_privacy: bool,

    // ═══════════════════════════════════════════════════════════════════════════
    // MCP SERVER MODE (v2.3.0)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Run as MCP (Model Context Protocol) server over stdio.
    /// Speaks JSON-RPC 2.0: reads requests from stdin, writes responses to stdout.
    /// All logging redirected to stderr.
    #[arg(long = "server")]
    server: bool,
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

/// Parse a report-utility string into (path, score, reason).
/// Format: "path/to/file:score:reason" where score is 0.0-1.0.
fn parse_report_utility(s: &str) -> Result<(String, f64, String), String> {
    // Split by ':' but be careful - file paths might contain colons on Windows
    // We expect: path:score:reason
    // Find the last two colons (reason:score are at the end)
    let parts: Vec<&str> = s.rsplitn(3, ':').collect();

    if parts.len() < 2 {
        return Err(format!(
            "Invalid report-utility format: '{}'. Expected 'path:score:reason' or 'path:score'",
            s
        ));
    }

    // parts are in reverse order: [reason, score, path] or [score, path]
    let (path, score_str, reason) = if parts.len() == 3 {
        (parts[2].to_string(), parts[1], parts[0].to_string())
    } else {
        (parts[1].to_string(), parts[0], "manual report".to_string())
    };

    let score: f64 = score_str.parse().map_err(|_| {
        format!("Invalid utility score: '{}'. Expected a number between 0.0 and 1.0", score_str)
    })?;

    if score < 0.0 || score > 1.0 {
        return Err(format!(
            "Utility score must be between 0.0 and 1.0, got: {}",
            score
        ));
    }

    Ok((path, score, reason))
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

    // Handle MCP Server Mode (v2.3.0)
    // When --server is set, run as JSON-RPC server over stdio
    if cli.server {
        let project_root = match &cli.project_root {
            Some(path) => path.clone(),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };

        if !project_root.exists() || !project_root.is_dir() {
            eprintln!("Error: Project root '{}' must be a valid directory", project_root.display());
            std::process::exit(1);
        }

        // Note: No startup logs here - MCP clients expect clean stdio
        let mut server = McpServer::new(project_root);
        if let Err(e) = server.run() {
            eprintln!("MCP server error: {}", e);
            std::process::exit(1);
        }
        return;
    }

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

    // Handle --report-utility command (Context Store v2.2.0)
    if let Some(utility_str) = &cli.report_utility {
        match parse_report_utility(utility_str) {
            Ok((path, score, reason)) => {
                // Load or create context store
                let store_path = ContextStore::default_path(&project_root);
                let mut store = if cli.store_privacy {
                    let mut s = ContextStore::load_from_file(&store_path);
                    s.paths_hashed = true;
                    s
                } else {
                    ContextStore::load_from_file(&store_path)
                };

                // Report the utility
                store.report_utility(&path, score, DEFAULT_ALPHA);

                // Save the store
                match store.save_to_file(&store_path) {
                    Ok(_) => {
                        eprintln!("Utility reported: {} = {:.2} ({})", path, score, reason);
                        eprintln!("Store saved to: {}", store_path.display());
                    }
                    Err(e) => {
                        eprintln!("Error saving context store: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
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

    // ═══════════════════════════════════════════════════════════════════════════
    // FRACTAL PROTOCOL v2: Zoom Session Management (v1.1.0)
    // ═══════════════════════════════════════════════════════════════════════════

    if let Some(session_cmd) = &cli.zoom_session {
        use pm_encoder::core::ZoomSessionStore;

        // Session store path (project-local)
        let session_store_path = ZoomSessionStore::default_path(&project_root);

        // Parse action:name format
        let parts: Vec<&str> = session_cmd.splitn(2, ':').collect();
        let action = parts[0];
        let name = parts.get(1).map(|s| *s);

        match action {
            "create" => {
                let name = name.unwrap_or("default");
                match ZoomSessionStore::with_persistence(&session_store_path, |store| {
                    store.create_session(name);
                    store.session_count()
                }) {
                    Ok(count) => {
                        eprintln!("Created zoom session: {}", name);
                        eprintln!("Total sessions: {}", count);
                        eprintln!("Use --zoom to add targets, --zoom-session show to view");
                    }
                    Err(e) => {
                        eprintln!("Error creating session: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "load" => {
                let name = name.unwrap_or("default");
                match ZoomSessionStore::with_persistence(&session_store_path, |store| {
                    store.set_active(name)
                }) {
                    Ok(Ok(())) => {
                        eprintln!("Loaded zoom session: {}", name);
                    }
                    Ok(Err(e)) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error loading sessions: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "list" => {
                match ZoomSessionStore::load(&session_store_path) {
                    Ok(store) => {
                        let sessions = store.list_sessions_with_meta();
                        if sessions.is_empty() {
                            eprintln!("No zoom sessions found.");
                            eprintln!("Use --zoom-session create:<name> to create one");
                        } else {
                            eprintln!("Zoom Sessions:");
                            for (name, is_active, last_accessed) in sessions {
                                let marker = if is_active { " *" } else { "" };
                                eprintln!("  {}{} (last: {})", name, marker, &last_accessed[..10]);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading sessions: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "show" => {
                match ZoomSessionStore::load(&session_store_path) {
                    Ok(store) => {
                        if let Some(session) = store.active() {
                            eprintln!("Active Session: {}", session.name);
                            if let Some(desc) = &session.description {
                                eprintln!("  Description: {}", desc);
                            }
                            eprintln!("  Created: {}", &session.created_at[..10]);
                            eprintln!("  Active zooms: {}", session.zoom_count());
                            for (target, depth) in &session.active_zooms {
                                eprintln!("    - {} ({:?})", target, depth);
                            }
                            if session.history.can_undo() {
                                eprintln!("  History: {} entries (undo available)", session.history.entries().len());
                            }
                        } else {
                            eprintln!("No active session.");
                            let names = store.list_sessions();
                            if !names.is_empty() {
                                eprintln!("Available: {:?}", names);
                                eprintln!("Use --zoom-session load:<name> to activate");
                            } else {
                                eprintln!("Use --zoom-session create:<name> to start");
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading sessions: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            "delete" => {
                let name = match name {
                    Some(n) => n,
                    None => {
                        eprintln!("Error: delete requires session name");
                        eprintln!("Usage: --zoom-session delete:<name>");
                        std::process::exit(1);
                    }
                };
                match ZoomSessionStore::with_persistence(&session_store_path, |store| {
                    store.delete_session(name)
                }) {
                    Ok(Ok(())) => {
                        eprintln!("Deleted session: {}", name);
                    }
                    Ok(Err(e)) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            _ => {
                eprintln!("Unknown zoom-session action: {}", action);
                eprintln!("Valid actions: create, load, list, delete, show");
                std::process::exit(1);
            }
        }
    }

    // Zoom undo/redo (Fractal v2)
    if cli.zoom_undo {
        use pm_encoder::core::ZoomSessionStore;
        let session_store_path = ZoomSessionStore::default_path(&project_root);

        match ZoomSessionStore::with_persistence(&session_store_path, |store| {
            if let Some(session) = store.active_mut() {
                if let Some(entry) = session.history.undo() {
                    eprintln!("Undo: {:?} {} on {}", entry.direction,
                        if matches!(entry.direction, pm_encoder::core::ZoomDirection::Expand) { "expand" } else { "collapse" },
                        entry.target);
                    true
                } else {
                    eprintln!("Nothing to undo");
                    false
                }
            } else {
                eprintln!("No active session");
                eprintln!("Use --zoom-session create:<name> to start a session");
                false
            }
        }) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    if cli.zoom_redo {
        use pm_encoder::core::ZoomSessionStore;
        let session_store_path = ZoomSessionStore::default_path(&project_root);

        match ZoomSessionStore::with_persistence(&session_store_path, |store| {
            if let Some(session) = store.active_mut() {
                if let Some(entry) = session.history.redo() {
                    eprintln!("Redo: {:?} on {}", entry.direction, entry.target);
                    true
                } else {
                    eprintln!("Nothing to redo");
                    false
                }
            } else {
                eprintln!("No active session");
                eprintln!("Use --zoom-session create:<name> to start a session");
                false
            }
        }) {
            Ok(_) => {}
            Err(e) => eprintln!("Error: {}", e),
        }
        return;
    }

    // Zoom collapse (bidirectional zoom)
    if let Some(collapse_str) = &cli.zoom_collapse {
        use pm_encoder::core::{ZoomTarget, ZoomSessionStore};
        let session_store_path = ZoomSessionStore::default_path(&project_root);

        match ZoomTarget::parse(collapse_str) {
            Ok(target) => {
                match ZoomSessionStore::with_persistence(&session_store_path, |store| {
                    if let Some(session) = store.active_mut() {
                        if session.remove_zoom(&target) {
                            eprintln!("Collapsed: {}", target);
                            true
                        } else {
                            eprintln!("Target not currently zoomed: {}", target);
                            false
                        }
                    } else {
                        eprintln!("No active session");
                        false
                    }
                }) {
                    Ok(_) => {}
                    Err(e) => eprintln!("Error: {}", e),
                }
                return;
            }
            Err(e) => {
                eprintln!("Error parsing collapse target: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Zoom mode (v2.0.0) - Fractal Protocol targeted context expansion
    if let Some(zoom_str) = &cli.zoom {
        let mut zoom_config = match parse_zoom_target(zoom_str) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        // ═══════════════════════════════════════════════════════════════════════════
        // FRACTAL PROTOCOL v2: Cross-File Symbol Resolution
        // ═══════════════════════════════════════════════════════════════════════════
        // Convert Function/Class/Module targets to File targets with resolved locations
        use pm_encoder::core::{SymbolResolver, SymbolType};

        // Track the original symbol name for excluding from suggestions
        let original_symbol_name: Option<String> = match &zoom_config.target {
            ZoomTarget::Function(name) | ZoomTarget::Class(name) => Some(name.clone()),
            _ => None,
        };

        let resolved_file: Option<String> = match &zoom_config.target {
            ZoomTarget::Function(name) => {
                let resolver = SymbolResolver::new()
                    .with_ignore(config.ignore_patterns.clone());

                match resolver.find_function(name, &project_root) {
                    Ok(loc) => {
                        eprintln!("Found {} at {}:{}-{}", name, loc.path, loc.start_line, loc.end_line);
                        eprintln!("  Signature: {}", loc.signature);

                        // Convert to file target with resolved lines
                        zoom_config.target = ZoomTarget::File {
                            path: loc.path.clone(),
                            start_line: Some(loc.start_line),
                            end_line: Some(loc.end_line),
                        };
                        Some(loc.path)
                    }
                    Err(e) => {
                        eprintln!("Symbol resolution failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ZoomTarget::Class(name) => {
                let resolver = SymbolResolver::new()
                    .with_ignore(config.ignore_patterns.clone());

                match resolver.find_class(name, &project_root) {
                    Ok(loc) => {
                        eprintln!("Found {} {} at {}:{}-{}",
                            loc.symbol_type, name, loc.path, loc.start_line, loc.end_line);
                        eprintln!("  Signature: {}", loc.signature);

                        zoom_config.target = ZoomTarget::File {
                            path: loc.path.clone(),
                            start_line: Some(loc.start_line),
                            end_line: Some(loc.end_line),
                        };
                        Some(loc.path)
                    }
                    Err(e) => {
                        eprintln!("Symbol resolution failed: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            ZoomTarget::Module(name) => {
                // Module resolution: find files matching the module name
                let module_patterns = vec![
                    format!("{}.rs", name),
                    format!("{}.py", name),
                    format!("{}/mod.rs", name),
                    format!("{}/__init__.py", name),
                ];
                eprintln!("Module zoom: Looking for files matching {:?}", module_patterns);
                None // Keep as-is, engine will handle module zoom
            }
            ZoomTarget::File { path, .. } => Some(path.clone()),
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
                // Apply Zoom Utility Bump (v2.2.0)
                // When a file is zoomed into, we bump its utility by +0.05
                // This teaches the system that zoomed files are likely relevant
                if !config.frozen {
                    if let Some(file_path) = &resolved_file {
                        let store_path = ContextStore::default_path(&project_root);
                        let mut store = ContextStore::load_from_file(&store_path);

                        const ZOOM_BUMP: f64 = 0.05;
                        store.bump_utility(file_path, ZOOM_BUMP, DEFAULT_ALPHA);

                        if let Err(e) = store.save_to_file(&store_path) {
                            eprintln!("Warning: Could not save zoom utility bump: {}", e);
                        }
                    }
                }

                // ═══════════════════════════════════════════════════════════════════════════
                // FRACTAL PROTOCOL v2: Call Graph Analysis & Zoom Suggestions
                // ═══════════════════════════════════════════════════════════════════════════
                use pm_encoder::core::{CallGraphAnalyzer, ZoomSuggestion};

                let call_analyzer = CallGraphAnalyzer::new().with_max_results(10);
                let resolver = SymbolResolver::new()
                    .with_ignore(config.ignore_patterns.clone());

                let valid_calls = call_analyzer.get_valid_calls(&output, &resolver, &project_root);

                // Generate zoom_menu if we found related functions
                let zoom_menu = if !valid_calls.is_empty() {
                    // Deduplicate by function name and exclude current target
                    let mut seen = std::collections::HashSet::new();
                    let suggestions: Vec<ZoomSuggestion> = valid_calls.iter()
                        .filter(|(call, _)| {
                            // Exclude the current zoom target
                            if let Some(ref orig) = original_symbol_name {
                                if &call.name == orig {
                                    return false;
                                }
                            }
                            seen.insert(call.name.clone())
                        })
                        .map(|(call, loc)| ZoomSuggestion::from_call(call, loc))
                        .collect();

                    let menu_items: Vec<String> = suggestions.iter()
                        .map(|s| format!("  {}", s.to_xml()))
                        .collect();

                    format!("\n<zoom_menu>\n{}\n</zoom_menu>", menu_items.join("\n"))
                } else {
                    String::new()
                };

                // Append zoom_menu to output
                let final_output = format!("{}{}", output, zoom_menu);

                if let Some(output_path) = cli.output {
                    match std::fs::write(&output_path, &final_output) {
                        Ok(_) => eprintln!("Zoom output written to: {}", output_path.display()),
                        Err(e) => {
                            eprintln!("Error writing output: {}", e);
                            std::process::exit(1);
                        }
                    }
                } else {
                    print!("{}", final_output);
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
            // Use streaming XmlWriter for ClaudeXml format with budget report (Fractal Protocol v2.0)
            // This includes hotspots/coldspots in attention_map from BudgetReport
            pm_encoder::serialize_entries_claude_xml_with_report(&config, &entries, &report)
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
