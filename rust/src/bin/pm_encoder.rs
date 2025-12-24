//! pm_encoder CLI - Compatibility wrapper
//!
//! This binary maintains backwards compatibility with the original `pm_encoder` command.
//! The primary command is now `vo` (Voyager Observatory).

// Exclude from coverage - CLI binary tested via integration tests
#![cfg_attr(tarpaulin, ignore)]

#[path = "vo.rs"]
mod vo;

fn main() {
    // Check if running as 'pm_encoder' and show a gentle hint (only on first run)
    if std::env::var("PM_ENCODER_NO_HINT").is_err() {
        if let Some(name) = std::env::args().next() {
            if name.ends_with("pm_encoder") || name.ends_with("pm_encoder.exe") {
                eprintln!("💡 Tip: The primary command is now 'vo' (Voyager Observatory)");
                eprintln!("   Set PM_ENCODER_NO_HINT=1 to suppress this message.");
                eprintln!();
            }
        }
    }

    // Delegate to vo's run function
    vo::run();
}
