//! Init-prompt module - Generates instruction files for AI assistants
//!
//! This module implements the "Split Brain" architecture (v1.4.0):
//! - Instruction file (CLAUDE.md / GEMINI_INSTRUCTIONS.txt): Commands, tree, stats, pointer
//! - Context file (CONTEXT.txt): Serialized codebase (separate file)
//!
//! The instruction file does NOT contain code, only a pointer to CONTEXT.txt.

use crate::python_style_split;
use std::fs;
use std::path::Path;

/// Detect common project commands based on project files
///
/// Scans the project root for common build system files and returns
/// appropriate commands for each detected system.
///
/// Note: Must match Python's detect_project_commands exactly for parity.
pub fn detect_project_commands(root: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let root_path = Path::new(root);

    // Rust: Cargo.toml
    if root_path.join("Cargo.toml").exists() {
        commands.push("cargo build".to_string());
        commands.push("cargo test".to_string());
    }

    // Node.js: package.json (Python uses npm test, npm start - NOT npm install)
    if root_path.join("package.json").exists() {
        commands.push("npm test".to_string());
        commands.push("npm start".to_string());
    }

    // Make: Makefile
    if root_path.join("Makefile").exists() {
        commands.push("make".to_string());
        commands.push("make test".to_string());
    }

    // Python: requirements.txt only (Python doesn't check pyproject.toml)
    if root_path.join("requirements.txt").exists() {
        commands.push("pip install -r requirements.txt".to_string());
    }

    commands
}

/// Generate a directory tree representation
///
/// Creates an ASCII tree structure showing the project layout.
/// Respects ignore patterns and max depth.
///
/// Note: Must match Python's generate_directory_tree exactly for parity:
/// - Skips hidden files (starting with '.')
/// - Sorts: directories first, then alphabetically by lowercase name
/// - No root directory line
pub fn generate_directory_tree(
    root: &str,
    ignore_patterns: &[String],
    max_depth: usize,
) -> Vec<String> {
    let mut lines = Vec::new();
    let root_path = Path::new(root);

    // Build tree recursively (no root line - matches Python)
    build_tree_recursive(root_path, &mut lines, "", ignore_patterns, max_depth);

    lines
}

fn build_tree_recursive(
    current: &Path,
    lines: &mut Vec<String>,
    prefix: &str,
    ignore_patterns: &[String],
    max_depth: usize,
) {
    if max_depth == 0 {
        return;
    }

    // Read directory entries
    let mut entries: Vec<_> = match fs::read_dir(current) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    // Sort entries: directories first, then by lowercase name (matches Python)
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_name = a.file_name().to_string_lossy().to_lowercase();
                let b_name = b.file_name().to_string_lossy().to_lowercase();
                a_name.cmp(&b_name)
            }
        }
    });

    // Filter out hidden files and ignored entries (matches Python)
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Skip hidden files (Python skips these)
            if name_str.starts_with('.') {
                return false;
            }

            // Check against ignore patterns
            for pattern in ignore_patterns {
                // Use fnmatch-style matching
                if pattern.contains('*') {
                    // Simple glob pattern
                    if pattern.starts_with("*.") {
                        let ext = &pattern[1..];
                        if name_str.ends_with(ext) {
                            return false;
                        }
                    }
                } else if name_str == pattern.as_str() {
                    return false;
                }
            }
            true
        })
        .collect();

    let count = entries.len();

    for (i, entry) in entries.into_iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            lines.push(format!("{}{}{}/", prefix, connector, name_str));
            build_tree_recursive(
                &entry.path(),
                lines,
                &format!("{}{}", prefix, child_prefix),
                ignore_patterns,
                max_depth - 1,
            );
        } else {
            lines.push(format!("{}{}{}", prefix, connector, name_str));
        }
    }
}

/// Generate the .pm_encoder_meta header content
///
/// Matches Python's lens_manager.get_meta_content() output exactly.
fn generate_meta_header(lens_name: &str, description: &str) -> String {
    use chrono::Utc;

    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string();

    let mut content = String::new();
    content.push_str(&format!("Context generated with lens: \"{}\"\n", lens_name));
    content.push_str(&format!("Focus: {}\n", description));
    content.push('\n');

    // For architecture lens, add truncation info
    if lens_name == "architecture" {
        content.push_str("Implementation details truncated using structure mode\n");
        content.push_str("Output shows only:\n");
        content.push_str("  - Import/export statements\n");
        content.push_str("  - Class and function signatures\n");
        content.push_str("  - Type definitions and interfaces\n");
        content.push_str("  - Module-level documentation\n");
        content.push('\n');
    }

    content.push_str(&format!("Generated: {}\n", timestamp));
    content.push_str(&format!("pm_encoder version: {}\n", crate::VERSION));

    // Calculate MD5 of content
    let checksum = crate::calculate_md5(&content);

    // Format as Plus/Minus file entry
    format!(
        "++++++++++ .pm_encoder_meta ++++++++++\n{}\
---------- .pm_encoder_meta {} .pm_encoder_meta ----------\n",
        content, checksum
    )
}

/// Format a number with thousand separators (commas)
///
/// Matches Python's {:,} format specifier for parity.
fn format_with_commas(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Get the instruction file name for a target
fn get_instruction_filename(target: &str) -> &'static str {
    match target.to_lowercase().as_str() {
        "gemini" => "GEMINI_INSTRUCTIONS.txt",
        _ => "CLAUDE.md", // Default to Claude
    }
}

/// Initialize AI instruction files (Split Brain architecture)
///
/// Creates two files:
/// 1. Instruction file (CLAUDE.md or GEMINI_INSTRUCTIONS.txt): Commands, tree, stats
/// 2. Context file (CONTEXT.txt): Serialized codebase
///
/// The instruction file points to CONTEXT.txt, does NOT contain code.
pub fn init_prompt(root: &str, lens_name: &str, target: &str) -> Result<(String, String), String> {
    use crate::{serialize_project_with_config, EncoderConfig, LensManager};

    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("Directory not found: {}", root));
    }

    // Step 1: Detect project commands
    let commands = detect_project_commands(root);

    // Step 2: Generate directory tree (max_depth=3 matches Python)
    let tree_ignore = vec![
        ".git".to_string(),
        "target".to_string(),
        ".venv".to_string(),
        "__pycache__".to_string(),
        "node_modules".to_string(),
        "*.pyc".to_string(),
        // Exclude generated files (prevent recursion, matches Python)
        "CONTEXT.txt".to_string(),
        "CLAUDE.md".to_string(),
        "GEMINI_INSTRUCTIONS.txt".to_string(),
    ];
    let tree = generate_directory_tree(root, &tree_ignore, 3);

    // Step 3: Apply lens and serialize context
    let mut lens_manager = LensManager::new();
    let applied_lens = lens_manager.apply_lens(lens_name)?;

    // Start with default ignore patterns (matches Python's load_config behavior)
    let default_ignores = vec![
        ".git".to_string(),
        "target".to_string(),
        ".venv".to_string(),
        "__pycache__".to_string(),
        "*.pyc".to_string(),
        "*.swp".to_string(),
    ];

    // Merge default ignores with lens exclude patterns (matches Python)
    let mut merged_ignores = default_ignores;
    for pattern in &applied_lens.ignore_patterns {
        if !merged_ignores.contains(pattern) {
            merged_ignores.push(pattern.clone());
        }
    }

    // Apply all lens settings including truncation (matches Python)
    let config = EncoderConfig {
        ignore_patterns: merged_ignores,
        include_patterns: applied_lens.include_patterns.clone(),
        sort_by: applied_lens.sort_by.clone(),
        sort_order: applied_lens.sort_order.clone(),
        truncate_lines: applied_lens.truncate_lines,
        truncate_mode: applied_lens.truncate_mode.clone(),
        ..Default::default()
    };

    // Generate meta header (matches Python's lens_manager.get_meta_content())
    let meta_header = generate_meta_header(lens_name, &applied_lens.description);

    let serialized_content = serialize_project_with_config(root, &config)?;

    // Prepend meta header to context (matches Python behavior)
    let context = format!("{}{}", meta_header, serialized_content);
    let context_lines = python_style_split(&context).len();
    let context_bytes = context.len();

    // Step 4: Write CONTEXT.txt
    let context_path = root_path.join("CONTEXT.txt");
    fs::write(&context_path, &context)
        .map_err(|e| format!("Failed to write CONTEXT.txt: {}", e))?;

    // Step 5: Generate instruction file content
    let instruction_filename = get_instruction_filename(target);

    // Get project name from directory - handle "." by canonicalizing first
    let canonical_path = root_path
        .canonicalize()
        .unwrap_or_else(|_| root_path.to_path_buf());
    let project_name = canonical_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    let instructions = generate_instruction_content(
        project_name,
        lens_name,
        &commands,
        &tree,
        context_lines,
        context_bytes,
    );

    // Step 6: Write instruction file
    let instruction_path = root_path.join(instruction_filename);
    fs::write(&instruction_path, &instructions)
        .map_err(|e| format!("Failed to write {}: {}", instruction_filename, e))?;

    Ok((
        instruction_path.to_string_lossy().to_string(),
        context_path.to_string_lossy().to_string(),
    ))
}

/// Generate the content for the instruction file
fn generate_instruction_content(
    project_name: &str,
    lens_name: &str,
    commands: &[String],
    tree: &[String],
    _context_lines: usize,
    context_bytes: usize,
) -> String {
    let mut content = String::new();

    // Header
    content.push_str(&format!("# {}\n\n", project_name));
    content.push_str("This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.\n\n");

    // Project Overview
    content.push_str("## Project Overview\n\n");
    content.push_str(&format!(
        "{} - Context automatically generated by pm_encoder\n\n",
        project_name
    ));

    // Quick Start
    content.push_str("## Quick Start\n\n");
    content.push_str(&format!(
        "This is the project context serialized using the `{}` lens for optimal AI understanding.\n\n",
        lens_name
    ));

    // Commands
    if !commands.is_empty() {
        content.push_str("## Commands\n\n");
        content.push_str("Common commands detected for this project:\n");
        for cmd in commands {
            content.push_str(&format!("- `{}`\n", cmd));
        }
        content.push('\n');
    }

    // Project Structure (matches Python format: project_name/ followed by tree)
    content.push_str("## Project Structure\n\n");
    content.push_str("```\n");
    content.push_str(&format!("{}/\n", project_name));
    for line in tree {
        content.push_str(line);
        content.push('\n');
    }
    content.push_str("```\n\n");

    // Statistics (matches Python format)
    // Note: Python uses file_count from tree, we use context_lines as approximation
    let file_count = tree.iter().filter(|line| !line.ends_with('/')).count();
    content.push_str("**Statistics:**\n");
    content.push_str(&format!("- Files: {}\n", file_count));
    // Format bytes with thousand separators (matches Python's {:,})
    let bytes_str = format_with_commas(context_bytes);
    content.push_str(&format!(
        "- Context size: {} bytes ({:.1} KB)\n\n",
        bytes_str,
        context_bytes as f64 / 1024.0
    ));

    // Pointer to CONTEXT.txt
    content.push_str("For the complete codebase context, see `CONTEXT.txt` in this directory.\n\n");

    // Footer
    content.push_str("---\n\n");
    content.push_str("**Regenerate these files:**\n");
    content.push_str("```bash\n");
    content.push_str(&format!(
        "./pm_encoder.py . --init-prompt --init-lens {} --target claude\n",
        lens_name
    ));
    content.push_str("```\n\n");
    content.push_str(&format!(
        "*Generated by pm_encoder v{} using the '{}' lens*\n",
        crate::VERSION,
        lens_name
    ));

    content
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_project_commands_makefile() {
        let temp = std::env::temp_dir().join("pm_test_commands_makefile");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("Makefile"), "all:\n\techo test").unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(
            commands.contains(&"make".to_string()),
            "Should detect make command"
        );
        assert!(
            commands.contains(&"make test".to_string()),
            "Should detect make test command"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_project_commands_cargo() {
        let temp = std::env::temp_dir().join("pm_test_commands_cargo");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(commands.contains(&"cargo build".to_string()));
        assert!(commands.contains(&"cargo test".to_string()));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_project_commands_npm() {
        let temp = std::env::temp_dir().join("pm_test_commands_npm");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("package.json"), "{}").unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(commands.contains(&"npm test".to_string()));
        assert!(commands.contains(&"npm start".to_string()));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_project_commands_multiple() {
        let temp = std::env::temp_dir().join("pm_test_commands_multi");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("Makefile"), "test:").unwrap();
        fs::write(temp.join("Cargo.toml"), "[package]").unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(commands.contains(&"make".to_string()));
        assert!(commands.contains(&"cargo build".to_string()));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree() {
        let temp = std::env::temp_dir().join("pm_test_tree");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::create_dir_all(temp.join("src")).unwrap();
        fs::write(temp.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(temp.join("Cargo.toml"), "[package]").unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 3);

        // Check structure
        assert!(!tree.is_empty());
        let tree_str = tree.join("\n");
        assert!(tree_str.contains("src/"), "Tree should contain src/");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_respects_ignore() {
        let temp = std::env::temp_dir().join("pm_test_tree_ignore");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::create_dir_all(temp.join(".git")).unwrap();
        fs::create_dir_all(temp.join("src")).unwrap();
        fs::write(temp.join(".git/config"), "").unwrap();
        fs::write(temp.join("src/main.rs"), "").unwrap();

        let ignore = vec![".git".to_string()];
        let tree = generate_directory_tree(temp.to_str().unwrap(), &ignore, 3);

        let tree_str = tree.join("\n");
        assert!(!tree_str.contains(".git"), "Tree should not contain .git");
        assert!(tree_str.contains("src"), "Tree should contain src");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_init_prompt_creates_split_files() {
        let temp = std::env::temp_dir().join("pm_test_init_prompt");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("main.py"), "print('hello')").unwrap();

        let result = init_prompt(temp.to_str().unwrap(), "architecture", "claude");

        if let Ok((instruction_path, context_path)) = result {
            // Verify CLAUDE.md exists and has correct content
            assert!(
                Path::new(&instruction_path).exists(),
                "CLAUDE.md should exist"
            );
            let instruction_content = fs::read_to_string(&instruction_path).unwrap();
            assert!(
                instruction_content.contains("CONTEXT.txt"),
                "Should point to CONTEXT.txt"
            );
            assert!(
                !instruction_content.contains("print('hello')"),
                "CLAUDE.md should NOT contain code"
            );

            // Verify CONTEXT.txt exists and has code
            assert!(
                Path::new(&context_path).exists(),
                "CONTEXT.txt should exist"
            );
            let context_content = fs::read_to_string(&context_path).unwrap();
            assert!(
                context_content.contains("print('hello')"),
                "CONTEXT.txt should contain code"
            );
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_init_prompt_gemini_target() {
        let temp = std::env::temp_dir().join("pm_test_init_gemini");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("main.py"), "x = 1").unwrap();

        let result = init_prompt(temp.to_str().unwrap(), "architecture", "gemini");

        if let Ok((instruction_path, _)) = result {
            assert!(
                instruction_path.contains("GEMINI_INSTRUCTIONS.txt"),
                "Should create GEMINI_INSTRUCTIONS.txt for gemini target"
            );
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_get_instruction_filename() {
        assert_eq!(get_instruction_filename("claude"), "CLAUDE.md");
        assert_eq!(get_instruction_filename("Claude"), "CLAUDE.md");
        assert_eq!(get_instruction_filename("CLAUDE"), "CLAUDE.md");
        assert_eq!(
            get_instruction_filename("gemini"),
            "GEMINI_INSTRUCTIONS.txt"
        );
        assert_eq!(
            get_instruction_filename("Gemini"),
            "GEMINI_INSTRUCTIONS.txt"
        );
        assert_eq!(get_instruction_filename("unknown"), "CLAUDE.md"); // Default
    }

    #[test]
    fn test_init_prompt_nonexistent_directory() {
        let result = init_prompt("/nonexistent/path/xyz", "architecture", "claude");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // TDD TEST FOR PYTHON PARITY (Gap #3: Tree includes CONTEXT.txt)
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_claude_md_tree_includes_context_txt() {
        // Python behavior: CLAUDE.md tree shows CONTEXT.txt in the project structure
        // because CONTEXT.txt is generated BEFORE the tree, so it appears in the listing

        let temp = std::env::temp_dir().join("pm_test_tree_context");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("main.py"), "x = 1").unwrap();

        let result = init_prompt(temp.to_str().unwrap(), "architecture", "claude");

        if let Ok((instruction_path, _)) = result {
            let claude_md = fs::read_to_string(&instruction_path).unwrap();

            // The tree in CLAUDE.md should show CONTEXT.txt (Python parity)
            assert!(
                claude_md.contains("CONTEXT.txt"),
                "CLAUDE.md tree should include CONTEXT.txt. Got:\n{}",
                &claude_md
            );
        } else {
            panic!("init_prompt should succeed");
        }

        let _ = fs::remove_dir_all(&temp);
    }

    // =========================================================================
    // Additional Coverage Tests
    // =========================================================================

    #[test]
    fn test_format_with_commas() {
        assert_eq!(format_with_commas(0), "0");
        assert_eq!(format_with_commas(1), "1");
        assert_eq!(format_with_commas(12), "12");
        assert_eq!(format_with_commas(123), "123");
        assert_eq!(format_with_commas(1234), "1,234");
        assert_eq!(format_with_commas(12345), "12,345");
        assert_eq!(format_with_commas(123456), "123,456");
        assert_eq!(format_with_commas(1234567), "1,234,567");
        assert_eq!(format_with_commas(1000000), "1,000,000");
    }

    #[test]
    fn test_detect_project_commands_requirements_txt() {
        let temp = std::env::temp_dir().join("pm_test_commands_requirements");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("requirements.txt"), "flask==2.0\nrequests").unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(commands.contains(&"pip install -r requirements.txt".to_string()));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_detect_project_commands_empty_directory() {
        let temp = std::env::temp_dir().join("pm_test_commands_empty");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let commands = detect_project_commands(temp.to_str().unwrap());
        assert!(commands.is_empty());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_max_depth_zero() {
        let temp = std::env::temp_dir().join("pm_test_tree_depth_zero");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::create_dir_all(temp.join("src")).unwrap();
        fs::write(temp.join("src/main.rs"), "").unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 0);
        assert!(tree.is_empty(), "Tree with max_depth=0 should be empty");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_glob_pattern_ignore() {
        let temp = std::env::temp_dir().join("pm_test_tree_glob");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("main.py"), "").unwrap();
        fs::write(temp.join("test.pyc"), "").unwrap();
        fs::write(temp.join("cache.pyc"), "").unwrap();

        let ignore = vec!["*.pyc".to_string()];
        let tree = generate_directory_tree(temp.to_str().unwrap(), &ignore, 3);

        let tree_str = tree.join("\n");
        assert!(tree_str.contains("main.py"), "Should contain main.py");
        assert!(!tree_str.contains(".pyc"), "Should not contain .pyc files");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_hidden_files() {
        let temp = std::env::temp_dir().join("pm_test_tree_hidden");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("visible.txt"), "").unwrap();
        fs::write(temp.join(".hidden"), "").unwrap();
        fs::create_dir_all(temp.join(".hidden_dir")).unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 3);

        let tree_str = tree.join("\n");
        assert!(
            tree_str.contains("visible.txt"),
            "Should contain visible.txt"
        );
        assert!(
            !tree_str.contains(".hidden"),
            "Should not contain hidden files"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_sorting() {
        let temp = std::env::temp_dir().join("pm_test_tree_sort");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::create_dir_all(temp.join("zebra")).unwrap();
        fs::create_dir_all(temp.join("alpha")).unwrap();
        fs::write(temp.join("beta.txt"), "").unwrap();
        fs::write(temp.join("gamma.txt"), "").unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 3);

        // Directories first (alpha, zebra), then files (beta, gamma)
        let alpha_idx = tree.iter().position(|l| l.contains("alpha/"));
        let zebra_idx = tree.iter().position(|l| l.contains("zebra/"));
        let beta_idx = tree.iter().position(|l| l.contains("beta.txt"));
        let gamma_idx = tree.iter().position(|l| l.contains("gamma.txt"));

        // Dirs should come before files
        assert!(alpha_idx.unwrap() < beta_idx.unwrap());
        assert!(zebra_idx.unwrap() < beta_idx.unwrap());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_nested() {
        let temp = std::env::temp_dir().join("pm_test_tree_nested");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("a/b/c")).unwrap();
        fs::write(temp.join("a/b/c/deep.txt"), "").unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 5);

        let tree_str = tree.join("\n");
        assert!(tree_str.contains("a/"), "Should contain a/");
        assert!(tree_str.contains("b/"), "Should contain b/");
        assert!(tree_str.contains("c/"), "Should contain c/");
        assert!(tree_str.contains("deep.txt"), "Should contain deep.txt");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_depth_limit() {
        let temp = std::env::temp_dir().join("pm_test_tree_depth_limit");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("a/b/c/d")).unwrap();
        fs::write(temp.join("a/b/c/d/deep.txt"), "").unwrap();

        // Depth 2 should not reach d/
        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 2);
        let tree_str = tree.join("\n");

        assert!(tree_str.contains("a/"), "Should contain a/");
        assert!(tree_str.contains("b/"), "Should contain b/");
        assert!(!tree_str.contains("c/"), "Should not contain c/ at depth 2");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_meta_header_architecture() {
        let header = generate_meta_header("architecture", "Architecture and structure");

        assert!(header.contains("lens: \"architecture\""));
        assert!(header.contains("Focus: Architecture"));
        assert!(header.contains("Implementation details truncated"));
        assert!(header.contains("Import/export statements"));
        assert!(header.contains("pm_encoder version:"));
        assert!(header.contains(".pm_encoder_meta"));
    }

    #[test]
    fn test_generate_meta_header_debug() {
        let header = generate_meta_header("debug", "Debug lens");

        assert!(header.contains("lens: \"debug\""));
        assert!(header.contains("Focus: Debug"));
        // Debug lens should NOT have truncation info
        assert!(!header.contains("Implementation details truncated"));
    }

    #[test]
    fn test_generate_instruction_content() {
        let content = generate_instruction_content(
            "test_project",
            "architecture",
            &["cargo build".to_string(), "cargo test".to_string()],
            &["├── src/".to_string(), "│   └── main.rs".to_string()],
            100,
            5000,
        );

        assert!(content.contains("# test_project"));
        assert!(content.contains("architecture"));
        assert!(content.contains("cargo build"));
        assert!(content.contains("cargo test"));
        assert!(content.contains("src/"));
        assert!(content.contains("main.rs"));
        assert!(content.contains("CONTEXT.txt"));
        assert!(content.contains("Statistics"));
    }

    #[test]
    fn test_generate_instruction_content_no_commands() {
        let content = generate_instruction_content(
            "empty_project",
            "debug",
            &[],
            &["├── readme.txt".to_string()],
            10,
            500,
        );

        assert!(content.contains("# empty_project"));
        // Should not have "Commands" section if no commands
        assert!(!content.contains("## Commands"));
    }

    #[test]
    fn test_generate_instruction_content_large_bytes() {
        let content =
            generate_instruction_content("big_project", "minimal", &[], &[], 10000, 1234567);

        // Should have comma-formatted bytes
        assert!(content.contains("1,234,567 bytes"));
    }

    #[test]
    fn test_init_prompt_invalid_lens() {
        let temp = std::env::temp_dir().join("pm_test_invalid_lens");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        fs::write(temp.join("test.py"), "x = 1").unwrap();

        let result = init_prompt(temp.to_str().unwrap(), "nonexistent_lens", "claude");
        assert!(result.is_err());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_empty_directory() {
        let temp = std::env::temp_dir().join("pm_test_tree_empty");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let tree = generate_directory_tree(temp.to_str().unwrap(), &vec![], 3);
        assert!(tree.is_empty(), "Empty directory should produce empty tree");

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn test_generate_directory_tree_nonexistent() {
        let tree = generate_directory_tree("/nonexistent/path/xyz", &vec![], 3);
        assert!(
            tree.is_empty(),
            "Nonexistent directory should produce empty tree"
        );
    }
}
