//! Universal Spectrograph Integration Tests
//!
//! These tests verify the Ancient Star recognition capabilities of the
//! Universal Spectrograph. From the first procedures of Simula to the
//! modern safety of Rust, no code goes unobserved.
//!
//! # The Ancient Stars Test
//!
//! Verifies that legacy languages (Simula, Logo, Tcl) are correctly
//! identified and their Stars are counted in a survey.

use pm_encoder::core::{
    PatternFallbackAnalyzer, StellarLibrary, Hemisphere, STELLAR_LIBRARY,
    GalaxyCensus,
};
use tempfile::TempDir;
use std::fs;

// =============================================================================
// ANCIENT STARS VERIFICATION
// =============================================================================

/// Test that Simula (the OO pioneer!) is correctly recognized
#[test]
fn test_simula_star_recognition() {
    let analyzer = PatternFallbackAnalyzer::new();

    // Classic Simula OOP - class Point with procedure Draw
    let simula_source = r#"
class Point;
begin
    real x, y;

    procedure Init(px, py);
    real px, py;
    begin
        x := px;
        y := py;
    end;

    procedure Draw;
    begin
        ! Draw the point at coordinates
        OutText("Point at ");
        OutFix(x, 2, 10);
        OutText(", ");
        OutFix(y, 2, 10);
    end;
end;

class ColoredPoint;
begin
    ref(Color) myColor;

    procedure SetColor(c);
    ref(Color) c;
    begin
        myColor :- c;
    end;
end;
"#;

    let metrics = analyzer.analyze_source("simula", simula_source);

    // Should find: class Point, procedure Init, procedure Draw,
    // class ColoredPoint, procedure SetColor
    assert!(metrics.stars.count >= 3, "Should find at least 3 Simula stars, found {}", metrics.stars.count);
    assert!(metrics.stars.types >= 2, "Should find at least 2 classes, found {}", metrics.stars.types);

    // Verify hemisphere classification
    assert_eq!(analyzer.get_hemisphere("simula"), Some(Hemisphere::Logic));
    assert_eq!(analyzer.display_name("simula"), Some("Simula"));
}

/// Test that Logo (Turtle Graphics!) is correctly recognized
#[test]
fn test_logo_star_recognition() {
    let analyzer = PatternFallbackAnalyzer::new();

    // Classic Logo turtle graphics procedures
    let logo_source = r#"
; Draw a square with the turtle
to square :size
  repeat 4 [forward :size right 90]
end

; Draw a circle approximation
to circle :radius
  repeat 360 [forward :radius * 3.14159 / 180 right 1]
end

; Draw a star shape
to star :size
  repeat 5 [forward :size right 144]
end

; Draw a spiral pattern
to spiral :size :angle
  if :size > 100 [stop]
  forward :size
  right :angle
  spiral :size + 2 :angle
end

; Main program
to main
  clearscreen
  pendown
  square 100
  penup
  forward 150
  pendown
  circle 50
end
"#;

    let metrics = analyzer.analyze_source("logo", logo_source);

    // Should find: to square, to circle, to star, to spiral, to main
    assert_eq!(metrics.stars.count, 5, "Should find exactly 5 Logo procedures, found {}", metrics.stars.count);
    assert_eq!(metrics.stars.functions, 5, "All Logo stars should be functions");

    // Verify hemisphere classification
    assert_eq!(analyzer.get_hemisphere("logo"), Some(Hemisphere::Logic));
    assert_eq!(analyzer.display_name("logo"), Some("Logo"));
}

/// Test that Tcl is correctly recognized
#[test]
fn test_tcl_star_recognition() {
    let analyzer = PatternFallbackAnalyzer::new();

    // Classic Tcl/Tk script with procedures
    let tcl_source = r#"
# Simple calculator implementation in Tcl

proc add {a b} {
    return [expr {$a + $b}]
}

proc subtract {a b} {
    return [expr {$a - $b}]
}

proc multiply {a b} {
    return [expr {$a * $b}]
}

proc divide {a b} {
    if {$b == 0} {
        error "Division by zero"
    }
    return [expr {$a / $b}]
}

proc calculate {operation a b} {
    switch $operation {
        "add"      { return [add $a $b] }
        "subtract" { return [subtract $a $b] }
        "multiply" { return [multiply $a $b] }
        "divide"   { return [divide $a $b] }
        default    { error "Unknown operation: $operation" }
    }
}

proc main {} {
    puts "Calculator Demo"
    puts "5 + 3 = [add 5 3]"
    puts "10 - 4 = [subtract 10 4]"
    puts "6 * 7 = [multiply 6 7]"
    puts "15 / 3 = [divide 15 3]"
}

# Run the main procedure
main
"#;

    let metrics = analyzer.analyze_source("tcl", tcl_source);

    // Should find: proc add, proc subtract, proc multiply, proc divide, proc calculate, proc main
    assert_eq!(metrics.stars.count, 6, "Should find exactly 6 Tcl procedures, found {}", metrics.stars.count);

    // Verify hemisphere classification
    assert_eq!(analyzer.get_hemisphere("tcl"), Some(Hemisphere::Automation));
    assert_eq!(analyzer.display_name("tcl"), Some("Tcl"));
}

// =============================================================================
// GALAXY CENSUS WITH ANCIENT STARS
// =============================================================================

/// Test that Ancient Stars are correctly counted in a Galaxy Census survey
#[test]
fn test_galaxy_census_with_ancient_stars() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create Simula file
    let simula_path = temp_dir.path().join("point.sim");
    fs::write(&simula_path, r#"
class Point;
begin
    real x, y;
    procedure Draw;
    begin
        ! Draw the point
    end;
end;
"#).expect("Failed to write Simula file");

    // Create Logo file
    let logo_path = temp_dir.path().join("turtle.logo");
    fs::write(&logo_path, r#"
to square :size
  repeat 4 [forward :size right 90]
end

to circle :radius
  repeat 360 [forward :radius right 1]
end
"#).expect("Failed to write Logo file");

    // Create Tcl file
    let tcl_path = temp_dir.path().join("utils.tcl");
    fs::write(&tcl_path, r#"
proc helper1 {} {
    puts "Helper 1"
}

proc helper2 {} {
    puts "Helper 2"
}

proc main {} {
    helper1
    helper2
}
"#).expect("Failed to write Tcl file");

    // Analyze each file using the PatternFallbackAnalyzer
    let analyzer = PatternFallbackAnalyzer::new();
    let mut galaxy = GalaxyCensus::new(temp_dir.path().to_string_lossy().to_string());

    // Analyze Simula
    if let Some(metrics) = analyzer.analyze_file(&simula_path) {
        galaxy.add_file(&simula_path.to_string_lossy(), metrics);
    }

    // Analyze Logo
    if let Some(metrics) = analyzer.analyze_file(&logo_path) {
        galaxy.add_file(&logo_path.to_string_lossy(), metrics);
    }

    // Analyze Tcl
    if let Some(metrics) = analyzer.analyze_file(&tcl_path) {
        galaxy.add_file(&tcl_path.to_string_lossy(), metrics);
    }

    galaxy.finalize();

    // Verify the galaxy census
    assert_eq!(galaxy.total_files, 3, "Should have analyzed 3 files");

    // Total stars: Simula (2: class + procedure) + Logo (2: to square, to circle) + Tcl (3: helper1, helper2, main)
    let total_stars = galaxy.totals.stars.count;
    assert!(total_stars >= 6, "Should find at least 6 total stars, found {}", total_stars);
}

// =============================================================================
// EXTENSION DETECTION
// =============================================================================

/// Test that file extensions are correctly mapped to languages
#[test]
fn test_extension_detection() {
    // Simula
    assert!(STELLAR_LIBRARY.get_by_extension("sim").is_some(), "Should recognize .sim extension");

    // Logo
    assert!(STELLAR_LIBRARY.get_by_extension("logo").is_some(), "Should recognize .logo extension");
    assert!(STELLAR_LIBRARY.get_by_extension("lg").is_some(), "Should recognize .lg extension");

    // Tcl
    assert!(STELLAR_LIBRARY.get_by_extension("tcl").is_some(), "Should recognize .tcl extension");
    assert!(STELLAR_LIBRARY.get_by_extension("tk").is_some(), "Should recognize .tk extension");

    // COBOL
    assert!(STELLAR_LIBRARY.get_by_extension("cob").is_some(), "Should recognize .cob extension");
    assert!(STELLAR_LIBRARY.get_by_extension("cbl").is_some(), "Should recognize .cbl extension");

    // Fortran
    assert!(STELLAR_LIBRARY.get_by_extension("f90").is_some(), "Should recognize .f90 extension");
    assert!(STELLAR_LIBRARY.get_by_extension("f95").is_some(), "Should recognize .f95 extension");
}

// =============================================================================
// HEMISPHERE CLASSIFICATION
// =============================================================================

/// Test that languages are correctly classified into hemispheres
#[test]
fn test_hemisphere_classification() {
    let library = StellarLibrary::new();

    // Logic hemisphere - Programming languages
    let logic_langs = ["rust", "python", "javascript", "java", "simula", "logo", "fortran", "lisp", "prolog"];
    for lang in logic_langs {
        let sig = library.get(lang).expect(&format!("{} should exist", lang));
        assert_eq!(sig.hemisphere, Hemisphere::Logic, "{} should be in Logic hemisphere", lang);
    }

    // Automation hemisphere - Scripting/DevOps
    let automation_langs = ["bash", "tcl", "powershell", "makefile", "dockerfile"];
    for lang in automation_langs {
        let sig = library.get(lang).expect(&format!("{} should exist", lang));
        assert_eq!(sig.hemisphere, Hemisphere::Automation, "{} should be in Automation hemisphere", lang);
    }

    // Interface hemisphere - UI/Markup
    let interface_langs = ["html", "css", "markdown", "latex"];
    for lang in interface_langs {
        let sig = library.get(lang).expect(&format!("{} should exist", lang));
        assert_eq!(sig.hemisphere, Hemisphere::Interface, "{} should be in Interface hemisphere", lang);
    }

    // Data hemisphere - Schema/Query languages
    let data_langs = ["sql", "json", "yaml", "graphql", "protobuf"];
    for lang in data_langs {
        let sig = library.get(lang).expect(&format!("{} should exist", lang));
        assert_eq!(sig.hemisphere, Hemisphere::Data, "{} should be in Data hemisphere", lang);
    }
}

// =============================================================================
// STELLAR LIBRARY COVERAGE
// =============================================================================

/// Test that all 6 tiers of languages are represented
#[test]
fn test_stellar_library_tiers() {
    let library = StellarLibrary::new();

    // Tier 1: The Giants (Modern Core)
    let tier1 = ["rust", "python", "javascript", "typescript", "java", "csharp", "cpp", "go", "ruby", "kotlin"];
    for lang in tier1 {
        assert!(library.get(lang).is_some(), "Tier 1 Giant '{}' should be supported", lang);
    }

    // Tier 2: Infrastructure & Automation
    let tier2 = ["bash", "tcl", "powershell", "makefile", "dockerfile", "hcl", "yaml"];
    for lang in tier2 {
        assert!(library.get(lang).is_some(), "Tier 2 Infrastructure '{}' should be supported", lang);
    }

    // Tier 3: Ancient Stars (Legacy Kings)
    let tier3 = ["cobol", "simula", "logo", "fortran", "pascal", "lisp", "prolog", "ada"];
    for lang in tier3 {
        assert!(library.get(lang).is_some(), "Tier 3 Ancient Star '{}' should be supported", lang);
    }

    // Tier 4: Functional & Logic
    let tier4 = ["haskell", "elixir", "erlang", "clojure", "scala", "fsharp", "ocaml"];
    for lang in tier4 {
        assert!(library.get(lang).is_some(), "Tier 4 Functional '{}' should be supported", lang);
    }

    // Tier 5: Stellar Nurseries (Emerging)
    let tier5 = ["zig", "nim", "gleam", "solidity", "mojo"];
    for lang in tier5 {
        assert!(library.get(lang).is_some(), "Tier 5 Emerging '{}' should be supported", lang);
    }

    // Tier 6: Scientific & Scripting
    let tier6 = ["lua", "perl", "r", "julia", "matlab", "graphql", "protobuf"];
    for lang in tier6 {
        assert!(library.get(lang).is_some(), "Tier 6 Scientific '{}' should be supported", lang);
    }
}

/// Test the total language count
#[test]
fn test_stellar_library_language_count() {
    let library = StellarLibrary::new();
    let count = library.language_count();

    // We should have 60+ languages
    assert!(count >= 60, "Should have at least 60 languages, found {}", count);

    // Verify the global singleton matches
    assert_eq!(STELLAR_LIBRARY.language_count(), count);
}

// =============================================================================
// PATTERN VALIDATION
// =============================================================================

/// Test that all star patterns are valid regex
#[test]
fn test_all_star_patterns_valid() {
    let library = StellarLibrary::new();

    for lang in library.languages() {
        let sig = library.get(lang).unwrap();
        let result = regex::Regex::new(sig.star_pattern);
        assert!(result.is_ok(), "Invalid star pattern for {}: {} - Error: {:?}",
            lang, sig.star_pattern, result.err());
    }
}

/// Test that comment patterns are valid regex
#[test]
fn test_all_comment_patterns_valid() {
    let library = StellarLibrary::new();

    for lang in library.languages() {
        let sig = library.get(lang).unwrap();

        // Single-line comment pattern
        let single_result = regex::Regex::new(sig.comment_single);
        assert!(single_result.is_ok(), "Invalid single-line comment pattern for {}: {}",
            lang, sig.comment_single);

        // Multi-line comment patterns (start)
        let multi_start_result = regex::Regex::new(sig.comment_multi_start);
        assert!(multi_start_result.is_ok(), "Invalid multi-line comment start for {}: {}",
            lang, sig.comment_multi_start);

        // Multi-line comment patterns (end)
        let multi_end_result = regex::Regex::new(sig.comment_multi_end);
        assert!(multi_end_result.is_ok(), "Invalid multi-line comment end for {}: {}",
            lang, sig.comment_multi_end);
    }
}
