//! Nebula Namer Module
//!
//! Generates human-readable names for semantic clusters (Nebulae).
//! Uses dominant concepts and directory patterns to create meaningful names.
//!
//! # Naming Strategies
//!
//! 1. **Concept-based**: Uses dominant concept types (e.g., "Payment Processing")
//! 2. **Directory-based**: Falls back to primary directory (e.g., "src/handlers")
//! 3. **Pattern-based**: Uses common naming patterns (e.g., "Test Suite")

use std::collections::HashMap;
use std::path::Path;

use crate::core::fractal::semantic::UniversalConceptType;

// =============================================================================
// Nebula Name
// =============================================================================

/// A human-readable name for a nebula (semantic cluster).
#[derive(Debug, Clone)]
pub struct NebulaName {
    /// The display name
    pub name: String,
    /// Optional subtitle for additional context
    pub subtitle: Option<String>,
    /// How the name was derived
    pub strategy: NamingStrategy,
    /// Confidence in the name (0.0 - 1.0)
    pub confidence: f32,
}

impl NebulaName {
    /// Create a new nebula name.
    pub fn new(name: impl Into<String>, strategy: NamingStrategy) -> Self {
        Self {
            name: name.into(),
            subtitle: None,
            strategy,
            confidence: 0.8,
        }
    }

    /// Add a subtitle.
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set confidence.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    /// Get the full display name.
    pub fn display(&self) -> String {
        if let Some(ref subtitle) = self.subtitle {
            format!("{} ({})", self.name, subtitle)
        } else {
            self.name.clone()
        }
    }
}

/// How a nebula name was derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamingStrategy {
    /// Named after dominant concept type
    ConceptBased,
    /// Named after primary directory
    DirectoryBased,
    /// Named after common naming pattern
    PatternBased,
    /// Named after file type distribution
    FileTypeBased,
    /// Fallback to generic name
    Fallback,
}

// =============================================================================
// Nebula Namer
// =============================================================================

/// The Nebula Namer generates human-readable names for semantic clusters.
pub struct NebulaNamer {
    /// Concept type to name mapping
    concept_names: HashMap<UniversalConceptType, &'static str>,
    /// Directory keywords to semantic names
    directory_names: HashMap<&'static str, &'static str>,
}

impl Default for NebulaNamer {
    fn default() -> Self {
        Self::new()
    }
}

impl NebulaNamer {
    /// Create a new nebula namer.
    pub fn new() -> Self {
        let mut concept_names = HashMap::new();
        concept_names.insert(UniversalConceptType::Calculation, "Data Processing");
        concept_names.insert(UniversalConceptType::Validation, "Input Validation");
        concept_names.insert(UniversalConceptType::Transformation, "Data Transformation");
        concept_names.insert(UniversalConceptType::Decision, "Business Logic");
        concept_names.insert(UniversalConceptType::DataStructure, "Data Models");
        concept_names.insert(UniversalConceptType::Service, "Service Layer");
        concept_names.insert(UniversalConceptType::Endpoint, "API Endpoints");
        concept_names.insert(UniversalConceptType::DatabaseOperation, "Data Persistence");
        concept_names.insert(UniversalConceptType::Integration, "External Integration");
        concept_names.insert(UniversalConceptType::ErrorHandling, "Error Management");
        concept_names.insert(UniversalConceptType::Infrastructure, "Core Infrastructure");
        concept_names.insert(UniversalConceptType::Configuration, "Configuration");
        concept_names.insert(UniversalConceptType::Observability, "Observability");
        concept_names.insert(UniversalConceptType::Testing, "Test Suite");
        concept_names.insert(UniversalConceptType::Unknown, "Miscellaneous");

        let mut directory_names = HashMap::new();
        // Common source directories
        directory_names.insert("src", "Source Core");
        directory_names.insert("lib", "Library");
        directory_names.insert("pkg", "Package");
        directory_names.insert("internal", "Internal");
        directory_names.insert("core", "Core Engine");
        directory_names.insert("app", "Application");

        // Feature directories
        directory_names.insert("api", "API Layer");
        directory_names.insert("handlers", "Request Handlers");
        directory_names.insert("routes", "Route Definitions");
        directory_names.insert("controllers", "Controllers");
        directory_names.insert("services", "Service Layer");
        directory_names.insert("models", "Data Models");
        directory_names.insert("entities", "Domain Entities");
        directory_names.insert("repos", "Data Repositories");
        directory_names.insert("repositories", "Data Repositories");
        directory_names.insert("db", "Database Layer");
        directory_names.insert("database", "Database Layer");
        directory_names.insert("migrations", "Schema Migrations");
        directory_names.insert("utils", "Utilities");
        directory_names.insert("helpers", "Helper Functions");
        directory_names.insert("common", "Shared Components");
        directory_names.insert("shared", "Shared Components");

        // Config directories
        directory_names.insert("config", "Configuration");
        directory_names.insert("configs", "Configuration");
        directory_names.insert("settings", "Settings");

        // Test directories
        directory_names.insert("test", "Test Suite");
        directory_names.insert("tests", "Test Suite");
        directory_names.insert("spec", "Specifications");
        directory_names.insert("specs", "Specifications");
        directory_names.insert("__tests__", "Test Suite");

        // UI directories
        directory_names.insert("components", "UI Components");
        directory_names.insert("views", "View Layer");
        directory_names.insert("pages", "Page Components");
        directory_names.insert("templates", "Templates");
        directory_names.insert("layouts", "Layouts");
        directory_names.insert("styles", "Styling");

        // Domain directories
        directory_names.insert("auth", "Authentication");
        directory_names.insert("users", "User Management");
        directory_names.insert("payments", "Payment Processing");
        directory_names.insert("orders", "Order Management");
        directory_names.insert("products", "Product Catalog");
        directory_names.insert("cart", "Shopping Cart");
        directory_names.insert("checkout", "Checkout Flow");
        directory_names.insert("notifications", "Notifications");
        directory_names.insert("email", "Email Service");
        directory_names.insert("messaging", "Messaging");

        // Infrastructure
        directory_names.insert("middleware", "Middleware");
        directory_names.insert("plugins", "Plugins");
        directory_names.insert("extensions", "Extensions");
        directory_names.insert("hooks", "Lifecycle Hooks");
        directory_names.insert("scripts", "Automation Scripts");
        directory_names.insert("bin", "Executables");
        directory_names.insert("cmd", "Commands");
        directory_names.insert("cli", "CLI Interface");

        Self {
            concept_names,
            directory_names,
        }
    }

    /// Name a nebula based on its files and dominant concepts.
    ///
    /// # Arguments
    /// * `files` - List of file paths in the nebula
    /// * `concept_counts` - Count of each concept type in the nebula
    ///
    /// # Returns
    /// A human-readable name for the nebula
    pub fn name_nebula(
        &self,
        files: &[String],
        concept_counts: &HashMap<UniversalConceptType, usize>,
    ) -> NebulaName {
        // Try concept-based naming first
        if let Some(name) = self.try_concept_name(concept_counts) {
            return name;
        }

        // Try directory-based naming
        if let Some(name) = self.try_directory_name(files) {
            return name;
        }

        // Try pattern-based naming
        if let Some(name) = self.try_pattern_name(files) {
            return name;
        }

        // Fallback
        NebulaName::new("Code Cluster", NamingStrategy::Fallback).with_confidence(0.3)
    }

    /// Try to name based on dominant concept type.
    fn try_concept_name(
        &self,
        concept_counts: &HashMap<UniversalConceptType, usize>,
    ) -> Option<NebulaName> {
        if concept_counts.is_empty() {
            return None;
        }

        let total: usize = concept_counts.values().sum();
        if total == 0 {
            return None;
        }

        // Find dominant concept
        let (dominant_type, dominant_count) =
            concept_counts.iter().max_by_key(|(_, count)| *count)?;

        // Must be at least 40% of total to be "dominant"
        let dominance = *dominant_count as f32 / total as f32;
        if dominance < 0.4 {
            return None;
        }

        // Skip Unknown type
        if *dominant_type == UniversalConceptType::Unknown {
            return None;
        }

        let name = self
            .concept_names
            .get(dominant_type)
            .copied()
            .unwrap_or("Code");

        Some(
            NebulaName::new(name, NamingStrategy::ConceptBased).with_confidence(dominance.min(1.0)),
        )
    }

    /// Try to name based on primary directory.
    fn try_directory_name(&self, files: &[String]) -> Option<NebulaName> {
        if files.is_empty() {
            return None;
        }

        // Count directory occurrences
        let mut dir_counts: HashMap<&str, usize> = HashMap::new();
        for file in files {
            let path = Path::new(file);
            // Get first meaningful directory component
            for component in path.components() {
                if let std::path::Component::Normal(os_str) = component {
                    if let Some(dir) = os_str.to_str() {
                        // Skip file names (has extension)
                        if dir.contains('.') {
                            continue;
                        }
                        *dir_counts.entry(dir).or_insert(0) += 1;
                    }
                }
            }
        }

        if dir_counts.is_empty() {
            return None;
        }

        // Find most common directory
        let (common_dir, count) = dir_counts.iter().max_by_key(|(_, count)| *count)?;

        // Must be present in at least 50% of files
        let coverage = *count as f32 / files.len() as f32;
        if coverage < 0.5 {
            return None;
        }

        // Look up semantic name
        let name = self
            .directory_names
            .get(*common_dir)
            .copied()
            .unwrap_or(*common_dir);

        // Capitalize if not already a semantic name
        let display_name = if name == *common_dir {
            capitalize_directory(name)
        } else {
            name.to_string()
        };

        Some(
            NebulaName::new(display_name, NamingStrategy::DirectoryBased)
                .with_subtitle(*common_dir)
                .with_confidence(coverage.min(0.9)),
        )
    }

    /// Try to name based on common file patterns.
    fn try_pattern_name(&self, files: &[String]) -> Option<NebulaName> {
        if files.is_empty() {
            return None;
        }

        // Check for test files
        let test_count = files
            .iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                lower.contains("test") || lower.contains("spec") || lower.starts_with("test_")
            })
            .count();

        if test_count > files.len() / 2 {
            return Some(
                NebulaName::new("Test Suite", NamingStrategy::PatternBased).with_confidence(0.9),
            );
        }

        // Check for config files
        let config_count = files
            .iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                lower.contains("config")
                    || lower.ends_with(".json")
                    || lower.ends_with(".yaml")
                    || lower.ends_with(".yml")
                    || lower.ends_with(".toml")
            })
            .count();

        if config_count > files.len() / 2 {
            return Some(
                NebulaName::new("Configuration", NamingStrategy::PatternBased)
                    .with_confidence(0.85),
            );
        }

        // Check for scripts
        let script_count = files
            .iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                lower.ends_with(".sh")
                    || lower.ends_with(".bash")
                    || lower.ends_with(".ps1")
                    || lower.ends_with(".bat")
            })
            .count();

        if script_count > files.len() / 2 {
            return Some(
                NebulaName::new("Automation Scripts", NamingStrategy::PatternBased)
                    .with_confidence(0.8),
            );
        }

        // Check for migration files
        let migration_count = files
            .iter()
            .filter(|f| {
                let lower = f.to_lowercase();
                lower.contains("migration") || lower.contains("migrate")
            })
            .count();

        if migration_count > files.len() / 2 {
            return Some(
                NebulaName::new("Schema Migrations", NamingStrategy::PatternBased)
                    .with_confidence(0.85),
            );
        }

        None
    }

    /// Get a name for a concept type.
    pub fn concept_type_name(&self, concept_type: UniversalConceptType) -> &str {
        self.concept_names
            .get(&concept_type)
            .copied()
            .unwrap_or("Code")
    }
}

/// Capitalize a directory name for display.
fn capitalize_directory(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut capitalize_next = true;

    for c in name.chars() {
        if c == '_' || c == '-' {
            result.push(' ');
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nebula_namer_concept_based() {
        let namer = NebulaNamer::new();

        let mut concepts = HashMap::new();
        concepts.insert(UniversalConceptType::Validation, 10);
        concepts.insert(UniversalConceptType::ErrorHandling, 2);

        let name = namer.name_nebula(&[], &concepts);
        assert_eq!(name.name, "Input Validation");
        assert_eq!(name.strategy, NamingStrategy::ConceptBased);
    }

    #[test]
    fn test_nebula_namer_directory_based() {
        let namer = NebulaNamer::new();

        // Files only in "handlers" directory (no conflicting parent directory)
        let files = vec![
            "handlers/user.rs".to_string(),
            "handlers/order.rs".to_string(),
            "handlers/payment.rs".to_string(),
        ];

        let name = namer.name_nebula(&files, &HashMap::new());
        assert_eq!(name.name, "Request Handlers");
        assert_eq!(name.strategy, NamingStrategy::DirectoryBased);
    }

    #[test]
    fn test_nebula_namer_pattern_based() {
        let namer = NebulaNamer::new();

        // Files with test pattern as flat files (no directory)
        // This skips directory-based naming and triggers pattern-based
        let files = vec![
            "test_user.py".to_string(),
            "test_order.py".to_string(),
            "test_payment.py".to_string(),
        ];

        let name = namer.name_nebula(&files, &HashMap::new());
        assert_eq!(name.name, "Test Suite");
        assert_eq!(name.strategy, NamingStrategy::PatternBased);
    }

    #[test]
    fn test_nebula_namer_tests_directory_uses_directory_based() {
        let namer = NebulaNamer::new();

        // Files in "tests" directory use directory-based naming
        let files = vec![
            "tests/test_user.py".to_string(),
            "tests/test_order.py".to_string(),
            "tests/test_payment.py".to_string(),
        ];

        let name = namer.name_nebula(&files, &HashMap::new());
        assert_eq!(name.name, "Test Suite");
        // Uses DirectoryBased because "tests" is a known directory
        assert_eq!(name.strategy, NamingStrategy::DirectoryBased);
    }

    #[test]
    fn test_nebula_namer_fallback() {
        let namer = NebulaNamer::new();

        let files = vec!["a.rs".to_string(), "b.py".to_string(), "c.js".to_string()];

        let name = namer.name_nebula(&files, &HashMap::new());
        assert_eq!(name.strategy, NamingStrategy::Fallback);
    }

    #[test]
    fn test_capitalize_directory() {
        assert_eq!(capitalize_directory("user_handlers"), "User Handlers");
        assert_eq!(capitalize_directory("api-routes"), "Api Routes");
        assert_eq!(capitalize_directory("models"), "Models");
    }

    #[test]
    fn test_nebula_name_display() {
        let name = NebulaName::new("Service Layer", NamingStrategy::ConceptBased)
            .with_subtitle("services");

        assert_eq!(name.display(), "Service Layer (services)");
    }
}
