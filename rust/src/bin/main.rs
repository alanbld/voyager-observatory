//! pm_encoder CLI - Command-line interface for the Rust engine
//!
//! This binary is a thin wrapper around the pm_encoder library.
//! All core logic lives in lib.rs, making it reusable for WASM/Python bindings.
//!
//! # Design Philosophy
//!
//! This CLI follows the "Thin Interface" pattern:
//! - Minimal logic in main()
//! - Delegates to the library for all actual work
//! - Only handles argument parsing and output formatting
//!
//! This ensures the library remains testable and reusable.

use pm_encoder; // Import our own library
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Check for arguments
    if args.len() > 1 {
        let path = &args[1];

        // Call the library function
        match pm_encoder::serialize_project(path) {
            Ok(output) => {
                // Write output to stdout without extra newline
                print!("{}", output);
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Show usage
        eprintln!("pm_encoder-rs v{}", pm_encoder::version());
        eprintln!("Usage: pm_encoder <path>");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  pm_encoder .              # Serialize current directory");
        eprintln!("  pm_encoder /path/to/repo  # Serialize specified directory");
        std::process::exit(1);
    }
}
