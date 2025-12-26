//! Celestial Census - Code Health Metrics
//!
//! This module implements the "Spectrograph" for Voyager Observatory,
//! providing astronomical measurements of code composition and health.
//!
//! # Celestial Terminology
//!
//! - **Stars (Logic)**: Functions, methods, procedures - the executable units
//! - **Nebulae (Knowledge)**: Documentation and comments - the explanatory matter
//! - **Dark Matter (Technical Debt)**: Unparsed regions, deep nesting - hidden complexity
//! - **Stellar Density**: Stars per 1,000 lines of code
//! - **Nebula Ratio**: Percentage of documentation relative to logic
//!
//! # Fallback Pattern Analysis
//!
//! When AST parsing is unavailable for a language, the census uses the
//! Universal Spectrograph's `StellarLibrary` for fallback pattern matching.
//! This allows star counting even for legacy languages like COBOL, Simula, and Logo.
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::census::{CelestialCensus, CensusMetrics, PatternFallbackAnalyzer};
//!
//! // AST-based analysis
//! let census = CelestialCensus::new();
//! let file: voyager_ast::ir::File = /* ... */;
//! let metrics = census.analyze(&file);
//!
//! // Fallback pattern-based analysis for unsupported languages
//! let fallback = PatternFallbackAnalyzer::new();
//! let metrics = fallback.analyze_source("simula", source_code);
//!
//! println!("Stars: {}", metrics.stars);
//! println!("Nebulae: {}", metrics.nebulae);
//! println!("Dark Matter: {}", metrics.dark_matter);
//! ```

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use voyager_ast::ir::{
    CommentKind, Declaration, DeclarationKind, File, Span,
};

use super::metrics::{MetricCollector, MetricRegistry, MetricResult};
use super::spectrograph::{STELLAR_LIBRARY, Hemisphere};

// =============================================================================
// Census Result Types
// =============================================================================

/// Complete census metrics for a file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CensusMetrics {
    /// Stars (Logic) - executable units
    pub stars: StarMetrics,
    /// Nebulae (Knowledge) - documentation
    pub nebulae: NebulaeMetrics,
    /// Dark Matter (Technical Debt) - hidden complexity
    pub dark_matter: DarkMatterMetrics,
    /// Derived ratios and densities
    pub derived: DerivedMetrics,
    /// Line count for context
    pub total_lines: usize,
}

/// Star (Logic) metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StarMetrics {
    /// Total number of stars (functions, methods, procedures)
    pub count: usize,
    /// Functions count
    pub functions: usize,
    /// Methods count
    pub methods: usize,
    /// Classes/structs count
    pub types: usize,
    /// Constants count
    pub constants: usize,
}

/// Nebulae (Knowledge) metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NebulaeMetrics {
    /// Total documentation lines
    pub doc_lines: usize,
    /// Total comment lines (non-doc)
    pub comment_lines: usize,
    /// Number of documented declarations
    pub documented_stars: usize,
    /// Total declarations (for ratio calculation)
    pub total_stars: usize,
}

/// Dark Matter (Technical Debt) metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DarkMatterMetrics {
    /// Unparsed/unknown regions count
    pub unknown_regions: usize,
    /// Total bytes in unknown regions
    pub unknown_bytes: usize,
    /// Volcanic regions (nesting > 4 levels)
    pub volcanic_regions: usize,
    /// Maximum nesting depth found
    pub max_nesting_depth: usize,
    /// Functions/methods with excessive parameters (> 5)
    pub parameter_heavy: usize,
}

/// Derived metrics (ratios and densities)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DerivedMetrics {
    /// Stars per 1,000 lines of code
    pub stellar_density: f64,
    /// Documentation coverage (0.0 - 1.0)
    pub nebula_ratio: f64,
    /// Dark matter percentage (0.0 - 1.0)
    pub dark_matter_ratio: f64,
    /// Health score (0.0 - 1.0, higher is better)
    pub health_score: f64,
}

/// Health rating for a constellation (directory/module)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthRating {
    /// Balanced star/nebula ratio
    Healthy,
    /// Low dark matter
    Stable,
    /// Significant unparsed or complex regions
    HighDarkMatter,
    /// Red Giants detected (large files with issues)
    Critical,
}

impl HealthRating {
    /// Get the celestial indicator
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::Healthy => "star",
            Self::Stable => "checkmark",
            Self::HighDarkMatter => "warning",
            Self::Critical => "alert",
        }
    }

    /// Get the description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Healthy => "Healthy Density",
            Self::Stable => "Stable System",
            Self::HighDarkMatter => "High Dark Matter",
            Self::Critical => "Critical Complexity",
        }
    }
}

// =============================================================================
// Celestial Census - Main Analyzer
// =============================================================================

/// The Celestial Census analyzer - measures code composition
pub struct CelestialCensus {
    /// Threshold for volcanic nesting (default: 4)
    volcanic_threshold: usize,
    /// Threshold for parameter-heavy functions (default: 5)
    param_threshold: usize,
}

impl Default for CelestialCensus {
    fn default() -> Self {
        Self::new()
    }
}

impl CelestialCensus {
    /// Create a new census analyzer with default thresholds
    pub fn new() -> Self {
        Self {
            volcanic_threshold: 4,
            param_threshold: 5,
        }
    }

    /// Create with custom thresholds
    pub fn with_thresholds(volcanic_threshold: usize, param_threshold: usize) -> Self {
        Self {
            volcanic_threshold,
            param_threshold,
        }
    }

    /// Analyze a file and produce census metrics
    pub fn analyze(&self, file: &File) -> CensusMetrics {
        let total_lines = self.count_lines(file);
        let stars = self.count_stars(file);
        let nebulae = self.count_nebulae(file);
        let dark_matter = self.count_dark_matter(file);

        // Calculate derived metrics
        let derived = self.calculate_derived(&stars, &nebulae, &dark_matter, total_lines);

        CensusMetrics {
            stars,
            nebulae,
            dark_matter,
            derived,
            total_lines,
        }
    }

    /// Determine health rating from metrics
    pub fn rate_health(&self, metrics: &CensusMetrics) -> HealthRating {
        // Critical: High dark matter AND low documentation
        if metrics.derived.dark_matter_ratio > 0.1 && metrics.derived.nebula_ratio < 0.1 {
            return HealthRating::Critical;
        }

        // High Dark Matter: Significant unparsed or deep nesting
        if metrics.dark_matter.unknown_regions > 0
            || metrics.dark_matter.volcanic_regions > 3
            || metrics.derived.dark_matter_ratio > 0.05
        {
            return HealthRating::HighDarkMatter;
        }

        // Healthy: Good documentation coverage
        if metrics.derived.nebula_ratio >= 0.2 && metrics.derived.stellar_density <= 30.0 {
            return HealthRating::Healthy;
        }

        // Default: Stable
        HealthRating::Stable
    }

    /// Count total lines in file
    fn count_lines(&self, file: &File) -> usize {
        if file.span.end_line > 0 {
            file.span.end_line
        } else {
            // Estimate from declarations if span not set
            file.declarations
                .iter()
                .map(|d| d.span.end_line)
                .max()
                .unwrap_or(0)
        }
    }

    /// Count stars (logic units)
    fn count_stars(&self, file: &File) -> StarMetrics {
        let mut metrics = StarMetrics::default();
        self.count_stars_recursive(&file.declarations, &mut metrics);
        metrics
    }

    fn count_stars_recursive(&self, decls: &[Declaration], metrics: &mut StarMetrics) {
        for decl in decls {
            match decl.kind {
                DeclarationKind::Function => {
                    metrics.functions += 1;
                    metrics.count += 1;
                }
                DeclarationKind::Method => {
                    metrics.methods += 1;
                    metrics.count += 1;
                }
                DeclarationKind::Class
                | DeclarationKind::Struct
                | DeclarationKind::Interface
                | DeclarationKind::Trait
                | DeclarationKind::Enum => {
                    metrics.types += 1;
                }
                DeclarationKind::Constant => {
                    metrics.constants += 1;
                }
                _ => {}
            }
            // Recurse into nested declarations
            self.count_stars_recursive(&decl.children, metrics);
        }
    }

    /// Count nebulae (documentation/comments)
    fn count_nebulae(&self, file: &File) -> NebulaeMetrics {
        let mut metrics = NebulaeMetrics::default();

        // Count file-level comments
        for comment in &file.comments {
            let lines = self.count_span_lines(&comment.span);
            match comment.kind {
                CommentKind::Doc => metrics.doc_lines += lines,
                CommentKind::Line | CommentKind::Block => metrics.comment_lines += lines,
            }
        }

        // Count documentation and total stars
        self.count_nebulae_recursive(&file.declarations, &mut metrics);

        metrics
    }

    fn count_nebulae_recursive(&self, decls: &[Declaration], metrics: &mut NebulaeMetrics) {
        for decl in decls {
            // Count this as a star if it's a function/method
            if matches!(
                decl.kind,
                DeclarationKind::Function | DeclarationKind::Method
            ) {
                metrics.total_stars += 1;
                if decl.doc_comment.is_some() {
                    metrics.documented_stars += 1;
                    if let Some(ref doc) = decl.doc_comment {
                        metrics.doc_lines += self.count_span_lines(&doc.span);
                    }
                }
            }

            // Recurse into children
            self.count_nebulae_recursive(&decl.children, metrics);
        }
    }

    /// Count dark matter (technical debt indicators)
    fn count_dark_matter(&self, file: &File) -> DarkMatterMetrics {
        let mut metrics = DarkMatterMetrics::default();

        // Count unknown regions
        metrics.unknown_regions = file.unknown_regions.len();
        metrics.unknown_bytes = file
            .unknown_regions
            .iter()
            .map(|r| r.span.len())
            .sum();

        // Analyze nesting depth in declarations (start at depth 1 for top-level)
        for decl in &file.declarations {
            self.analyze_nesting_depth(decl, 1, &mut metrics);

            // Check for parameter-heavy functions
            if decl.parameters.len() > self.param_threshold {
                metrics.parameter_heavy += 1;
            }
        }

        metrics
    }

    fn analyze_nesting_depth(
        &self,
        decl: &Declaration,
        current_depth: usize,
        metrics: &mut DarkMatterMetrics,
    ) {
        metrics.max_nesting_depth = metrics.max_nesting_depth.max(current_depth);

        if current_depth > self.volcanic_threshold {
            metrics.volcanic_regions += 1;
        }

        // Check children
        for child in &decl.children {
            self.analyze_nesting_depth(child, current_depth + 1, metrics);
        }
    }

    /// Calculate derived metrics
    fn calculate_derived(
        &self,
        stars: &StarMetrics,
        nebulae: &NebulaeMetrics,
        dark_matter: &DarkMatterMetrics,
        total_lines: usize,
    ) -> DerivedMetrics {
        let total_lines_f = total_lines.max(1) as f64;

        // Stellar density: stars per 1000 lines
        let stellar_density = (stars.count as f64 / total_lines_f) * 1000.0;

        // Nebula ratio: documented stars / total stars
        let nebula_ratio = if nebulae.total_stars > 0 {
            nebulae.documented_stars as f64 / nebulae.total_stars as f64
        } else {
            0.0
        };

        // Dark matter ratio: unknown bytes / total content estimate
        let total_bytes_estimate = total_lines * 50; // Rough estimate
        let dark_matter_ratio = if total_bytes_estimate > 0 {
            dark_matter.unknown_bytes as f64 / total_bytes_estimate as f64
        } else {
            0.0
        };

        // Health score: composite metric (0.0 - 1.0)
        let health_score = self.calculate_health_score(
            stellar_density,
            nebula_ratio,
            dark_matter_ratio,
            dark_matter.volcanic_regions,
        );

        DerivedMetrics {
            stellar_density,
            nebula_ratio,
            dark_matter_ratio,
            health_score,
        }
    }

    fn calculate_health_score(
        &self,
        stellar_density: f64,
        nebula_ratio: f64,
        dark_matter_ratio: f64,
        volcanic_regions: usize,
    ) -> f64 {
        let mut score = 1.0;

        // Penalize high dark matter
        score -= dark_matter_ratio.min(0.3);

        // Penalize volcanic regions
        score -= (volcanic_regions as f64 * 0.05).min(0.2);

        // Penalize low documentation
        if nebula_ratio < 0.1 {
            score -= 0.2;
        } else if nebula_ratio < 0.3 {
            score -= 0.1;
        }

        // Penalize extreme stellar density
        if stellar_density > 50.0 {
            score -= 0.15;
        } else if stellar_density > 30.0 {
            score -= 0.05;
        }

        score.clamp(0.0, 1.0)
    }

    fn count_span_lines(&self, span: &Span) -> usize {
        if span.end_line >= span.start_line {
            span.end_line - span.start_line + 1
        } else {
            1
        }
    }
}

// =============================================================================
// MetricCollector Implementations
// =============================================================================

/// Counts stars (functions, methods, procedures)
pub struct StarCountMetric;

impl MetricCollector for StarCountMetric {
    fn name(&self) -> &str {
        "star_count"
    }

    fn description(&self) -> &str {
        "Total number of stars (functions, methods, procedures)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);

        MetricResult::confident(
            metrics.stars.count as f64,
            format!(
                "{} stars detected ({} functions, {} methods)",
                metrics.stars.count, metrics.stars.functions, metrics.stars.methods
            ),
        )
    }
}

/// Counts nebulae (documentation lines)
pub struct NebulaeCountMetric;

impl MetricCollector for NebulaeCountMetric {
    fn name(&self) -> &str {
        "nebulae_count"
    }

    fn description(&self) -> &str {
        "Total documentation and comment lines (nebulae)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);
        let total = metrics.nebulae.doc_lines + metrics.nebulae.comment_lines;

        MetricResult::confident(
            total as f64,
            format!(
                "{} nebulae lines ({} doc, {} comments)",
                total, metrics.nebulae.doc_lines, metrics.nebulae.comment_lines
            ),
        )
    }
}

/// Counts dark matter (unknown regions and volcanic nesting)
pub struct DarkMatterMetric;

impl MetricCollector for DarkMatterMetric {
    fn name(&self) -> &str {
        "dark_matter"
    }

    fn description(&self) -> &str {
        "Technical debt indicators (unknown regions, deep nesting)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);
        let score = metrics.dark_matter.unknown_regions + metrics.dark_matter.volcanic_regions;

        let explanation = if score == 0 {
            "No dark matter detected - clean code".to_string()
        } else {
            format!(
                "{} dark matter regions ({} unknown, {} volcanic depth > 4)",
                score,
                metrics.dark_matter.unknown_regions,
                metrics.dark_matter.volcanic_regions
            )
        };

        MetricResult::new(score as f64, if score == 0 { 1.0 } else { 0.7 }, explanation)
    }
}

/// Calculates stellar density (stars per 1000 LOC)
pub struct StellarDensityMetric;

impl MetricCollector for StellarDensityMetric {
    fn name(&self) -> &str {
        "stellar_density"
    }

    fn description(&self) -> &str {
        "Stars per 1,000 lines of code"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);

        let explanation = if metrics.derived.stellar_density <= 15.0 {
            format!(
                "Low density ({:.1} stars/1k LOC) - well-factored",
                metrics.derived.stellar_density
            )
        } else if metrics.derived.stellar_density <= 30.0 {
            format!(
                "Moderate density ({:.1} stars/1k LOC)",
                metrics.derived.stellar_density
            )
        } else {
            format!(
                "High density ({:.1} stars/1k LOC) - consider refactoring",
                metrics.derived.stellar_density
            )
        };

        MetricResult::confident(metrics.derived.stellar_density, explanation)
    }
}

/// Calculates nebula ratio (documentation coverage)
pub struct NebulaRatioMetric;

impl MetricCollector for NebulaRatioMetric {
    fn name(&self) -> &str {
        "nebula_ratio"
    }

    fn description(&self) -> &str {
        "Percentage of documented stars (documentation coverage)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);
        let percentage = metrics.derived.nebula_ratio * 100.0;

        let explanation = if percentage >= 80.0 {
            format!("{:.0}% coverage - excellent documentation", percentage)
        } else if percentage >= 50.0 {
            format!("{:.0}% coverage - good documentation", percentage)
        } else if percentage >= 20.0 {
            format!("{:.0}% coverage - needs more documentation", percentage)
        } else {
            format!("{:.0}% coverage - sparse documentation", percentage)
        };

        MetricResult::confident(percentage, explanation)
    }
}

/// Overall health score
pub struct HealthScoreMetric;

impl MetricCollector for HealthScoreMetric {
    fn name(&self) -> &str {
        "health_score"
    }

    fn description(&self) -> &str {
        "Overall code health score (0-100)"
    }

    fn analyze(&self, file: &File) -> MetricResult {
        let census = CelestialCensus::new();
        let metrics = census.analyze(file);
        let score = metrics.derived.health_score * 100.0;
        let rating = census.rate_health(&metrics);

        MetricResult::confident(
            score,
            format!("{} - {} ({:.0}/100)", rating.indicator(), rating.description(), score),
        )
    }
}

// =============================================================================
// Registry Builder
// =============================================================================

/// Build a registry with all census metrics
pub fn build_census_registry() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    registry.register(Box::new(StarCountMetric));
    registry.register(Box::new(NebulaeCountMetric));
    registry.register(Box::new(DarkMatterMetric));
    registry.register(Box::new(StellarDensityMetric));
    registry.register(Box::new(NebulaRatioMetric));
    registry.register(Box::new(HealthScoreMetric));
    registry
}

// =============================================================================
// Constellation Aggregation
// =============================================================================

/// Aggregated census for a constellation (directory)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstellationCensus {
    /// Path to the constellation (directory)
    pub path: String,
    /// Number of files in this constellation
    pub file_count: usize,
    /// Aggregated metrics
    pub totals: CensusMetrics,
    /// Health rating
    pub rating: Option<HealthRating>,
    /// Files flagged as "Red Giants"
    pub red_giants: Vec<String>,
    /// Files flagged as "Stellar Nurseries" (high activity)
    pub nurseries: Vec<String>,
}

/// Galaxy-level census (entire project)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GalaxyCensus {
    /// Root path
    pub root: String,
    /// Total file count
    pub total_files: usize,
    /// Aggregated totals
    pub totals: CensusMetrics,
    /// Constellations by path (BTreeMap for determinism)
    pub constellations: BTreeMap<String, ConstellationCensus>,
    /// Overall health rating
    pub rating: Option<HealthRating>,
}

impl GalaxyCensus {
    /// Create a new galaxy census
    pub fn new(root: String) -> Self {
        Self {
            root,
            ..Default::default()
        }
    }

    /// Add a file's census to the appropriate constellation
    pub fn add_file(&mut self, file_path: &str, metrics: CensusMetrics) {
        self.total_files += 1;

        // Aggregate to totals
        self.aggregate_metrics(&metrics);

        // Find constellation (parent directory)
        let constellation_path = std::path::Path::new(file_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string());

        // Get or create constellation
        let constellation = self
            .constellations
            .entry(constellation_path.clone())
            .or_insert_with(|| ConstellationCensus {
                path: constellation_path,
                ..Default::default()
            });

        constellation.file_count += 1;
        constellation.aggregate_metrics(&metrics);

        // Check for Red Giant (large file with issues)
        if metrics.total_lines > 500
            && (metrics.derived.dark_matter_ratio > 0.05 || metrics.derived.nebula_ratio < 0.1)
        {
            constellation.red_giants.push(file_path.to_string());
        }
    }

    fn aggregate_metrics(&mut self, metrics: &CensusMetrics) {
        self.totals.stars.count += metrics.stars.count;
        self.totals.stars.functions += metrics.stars.functions;
        self.totals.stars.methods += metrics.stars.methods;
        self.totals.stars.types += metrics.stars.types;
        self.totals.stars.constants += metrics.stars.constants;

        self.totals.nebulae.doc_lines += metrics.nebulae.doc_lines;
        self.totals.nebulae.comment_lines += metrics.nebulae.comment_lines;
        self.totals.nebulae.documented_stars += metrics.nebulae.documented_stars;
        self.totals.nebulae.total_stars += metrics.nebulae.total_stars;

        self.totals.dark_matter.unknown_regions += metrics.dark_matter.unknown_regions;
        self.totals.dark_matter.unknown_bytes += metrics.dark_matter.unknown_bytes;
        self.totals.dark_matter.volcanic_regions += metrics.dark_matter.volcanic_regions;
        self.totals.dark_matter.max_nesting_depth = self
            .totals
            .dark_matter
            .max_nesting_depth
            .max(metrics.dark_matter.max_nesting_depth);
        self.totals.dark_matter.parameter_heavy += metrics.dark_matter.parameter_heavy;

        self.totals.total_lines += metrics.total_lines;
    }

    /// Finalize and compute ratings
    pub fn finalize(&mut self) {
        let census = CelestialCensus::new();

        // Recalculate derived metrics for totals
        self.totals.derived = census.calculate_derived(
            &self.totals.stars,
            &self.totals.nebulae,
            &self.totals.dark_matter,
            self.totals.total_lines,
        );
        self.rating = Some(census.rate_health(&self.totals));

        // Finalize each constellation
        for constellation in self.constellations.values_mut() {
            constellation.totals.derived = census.calculate_derived(
                &constellation.totals.stars,
                &constellation.totals.nebulae,
                &constellation.totals.dark_matter,
                constellation.totals.total_lines,
            );
            constellation.rating = Some(census.rate_health(&constellation.totals));
        }
    }
}

impl ConstellationCensus {
    fn aggregate_metrics(&mut self, metrics: &CensusMetrics) {
        self.totals.stars.count += metrics.stars.count;
        self.totals.stars.functions += metrics.stars.functions;
        self.totals.stars.methods += metrics.stars.methods;
        self.totals.stars.types += metrics.stars.types;
        self.totals.stars.constants += metrics.stars.constants;

        self.totals.nebulae.doc_lines += metrics.nebulae.doc_lines;
        self.totals.nebulae.comment_lines += metrics.nebulae.comment_lines;
        self.totals.nebulae.documented_stars += metrics.nebulae.documented_stars;
        self.totals.nebulae.total_stars += metrics.nebulae.total_stars;

        self.totals.dark_matter.unknown_regions += metrics.dark_matter.unknown_regions;
        self.totals.dark_matter.unknown_bytes += metrics.dark_matter.unknown_bytes;
        self.totals.dark_matter.volcanic_regions += metrics.dark_matter.volcanic_regions;
        self.totals.dark_matter.max_nesting_depth = self
            .totals
            .dark_matter
            .max_nesting_depth
            .max(metrics.dark_matter.max_nesting_depth);
        self.totals.dark_matter.parameter_heavy += metrics.dark_matter.parameter_heavy;

        self.totals.total_lines += metrics.total_lines;
    }
}

// =============================================================================
// Pattern Fallback Analyzer (Universal Spectrograph Integration)
// =============================================================================

/// Pattern-based fallback analyzer for languages without AST support.
///
/// Uses the Universal Spectrograph's StellarLibrary to count stars
/// via regex patterns when Tree-sitter parsing is unavailable.
///
/// # Example
///
/// ```rust,ignore
/// use pm_encoder::core::census::PatternFallbackAnalyzer;
///
/// let analyzer = PatternFallbackAnalyzer::new();
///
/// // Analyze a Simula file (no AST support)
/// let source = r#"
/// class Point;
/// begin
///     real x, y;
///     procedure Draw;
///     begin
///         ! draw the point
///     end;
/// end;
/// "#;
///
/// let metrics = analyzer.analyze_source("simula", source);
/// assert_eq!(metrics.stars.count, 2); // class + procedure
/// ```
pub struct PatternFallbackAnalyzer;

impl Default for PatternFallbackAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternFallbackAnalyzer {
    /// Create a new fallback analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze source code using pattern matching
    ///
    /// Returns CensusMetrics with star counts based on regex patterns.
    /// This is used for languages without Tree-sitter support.
    pub fn analyze_source(&self, language: &str, source: &str) -> CensusMetrics {
        let mut metrics = CensusMetrics::default();

        // Get the spectral signature for this language
        let signature = match STELLAR_LIBRARY.get(language) {
            Some(sig) => sig,
            None => return metrics, // Unknown language, return empty metrics
        };

        // Count total lines
        metrics.total_lines = source.lines().count();

        // Count stars using the star pattern
        if let Ok(star_regex) = regex::Regex::new(signature.star_pattern) {
            for cap in star_regex.captures_iter(source) {
                metrics.stars.count += 1;
                // Try to classify the star based on capture groups
                // For most patterns, group 1 is the name
                if cap.get(0).map(|m| m.as_str().contains("class")).unwrap_or(false)
                    || cap.get(0).map(|m| m.as_str().contains("struct")).unwrap_or(false)
                    || cap.get(0).map(|m| m.as_str().contains("type")).unwrap_or(false)
                    || cap.get(0).map(|m| m.as_str().contains("interface")).unwrap_or(false)
                {
                    metrics.stars.types += 1;
                } else {
                    metrics.stars.functions += 1;
                }
                metrics.nebulae.total_stars += 1;
            }
        }

        // Count single-line comments (nebulae)
        if signature.comment_single != "$^" {
            if let Ok(comment_regex) = regex::Regex::new(signature.comment_single) {
                for _cap in comment_regex.find_iter(source) {
                    metrics.nebulae.comment_lines += 1;
                }
            }
        }

        // Calculate derived metrics
        let total_lines_f = metrics.total_lines.max(1) as f64;
        metrics.derived.stellar_density = (metrics.stars.count as f64 / total_lines_f) * 1000.0;
        metrics.derived.nebula_ratio = if metrics.nebulae.total_stars > 0 {
            metrics.nebulae.documented_stars as f64 / metrics.nebulae.total_stars as f64
        } else {
            0.0
        };
        metrics.derived.health_score = 0.7; // Moderate confidence for pattern-based analysis

        metrics
    }

    /// Analyze a file by extension, reading from path
    ///
    /// Determines language from file extension and analyzes the source.
    pub fn analyze_file(&self, path: &std::path::Path) -> Option<CensusMetrics> {
        let ext = path.extension()?.to_str()?;
        // Verify the extension is supported
        let _ = STELLAR_LIBRARY.get_by_extension(ext)?;
        let source = std::fs::read_to_string(path).ok()?;

        // Find the language name from extension
        let language = STELLAR_LIBRARY.languages()
            .into_iter()
            .find(|lang| {
                STELLAR_LIBRARY.get(lang)
                    .map(|s| s.extensions.contains(&ext))
                    .unwrap_or(false)
            })?;

        Some(self.analyze_source(language, &source))
    }

    /// Get the hemisphere classification for a language
    pub fn get_hemisphere(&self, language: &str) -> Option<Hemisphere> {
        STELLAR_LIBRARY.get(language).map(|s| s.hemisphere)
    }

    /// Check if a language is supported for fallback analysis
    pub fn is_supported(&self, language: &str) -> bool {
        STELLAR_LIBRARY.get(language).is_some()
    }

    /// Get the display name for a language
    pub fn display_name(&self, language: &str) -> Option<&'static str> {
        STELLAR_LIBRARY.get(language).map(|s| s.display_name)
    }

    /// Analyze by extension string
    pub fn analyze_by_extension(&self, ext: &str, source: &str) -> Option<CensusMetrics> {
        // Verify the extension is supported
        let _ = STELLAR_LIBRARY.get_by_extension(ext)?;

        // Find language name by extension
        let language = STELLAR_LIBRARY.languages()
            .into_iter()
            .find(|lang| {
                STELLAR_LIBRARY.get(lang)
                    .map(|s| s.extensions.contains(&ext))
                    .unwrap_or(false)
            })?;

        Some(self.analyze_source(language, source))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use voyager_ast::ir::{LanguageId, Visibility};

    fn make_test_file() -> File {
        let mut file = File::new("test.rs".to_string(), LanguageId::Rust);
        file.span = Span::new(0, 1000, 1, 50);

        // Add some declarations
        let mut func1 = Declaration::new(
            "documented_func".to_string(),
            DeclarationKind::Function,
            Span::new(0, 100, 1, 10),
        );
        func1.doc_comment = Some(voyager_ast::ir::Comment {
            text: "A documented function".to_string(),
            kind: CommentKind::Doc,
            span: Span::new(0, 25, 1, 1),
            attached_to: None,
        });
        func1.visibility = Visibility::Public;

        let func2 = Declaration::new(
            "undocumented_func".to_string(),
            DeclarationKind::Function,
            Span::new(100, 200, 11, 20),
        );

        let method = Declaration::new(
            "some_method".to_string(),
            DeclarationKind::Method,
            Span::new(200, 300, 21, 30),
        );

        file.declarations.push(func1);
        file.declarations.push(func2);
        file.declarations.push(method);

        file
    }

    #[test]
    fn test_star_count() {
        let file = make_test_file();
        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);

        assert_eq!(metrics.stars.count, 3); // 2 functions + 1 method
        assert_eq!(metrics.stars.functions, 2);
        assert_eq!(metrics.stars.methods, 1);
    }

    #[test]
    fn test_nebulae_count() {
        let file = make_test_file();
        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);

        assert_eq!(metrics.nebulae.documented_stars, 1);
        assert_eq!(metrics.nebulae.total_stars, 3);
        assert!(metrics.nebulae.doc_lines > 0);
    }

    #[test]
    fn test_dark_matter_with_unknown_regions() {
        let mut file = make_test_file();
        file.unknown_regions.push(voyager_ast::ir::UnknownNode {
            span: Span::new(400, 450, 35, 38),
            reason: Some("Syntax error".to_string()),
            raw_text: None,
        });

        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);

        assert_eq!(metrics.dark_matter.unknown_regions, 1);
        assert_eq!(metrics.dark_matter.unknown_bytes, 50);
    }

    #[test]
    fn test_stellar_density() {
        let file = make_test_file();
        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);

        // 3 stars in 50 lines = 60 stars per 1000 LOC
        assert!(metrics.derived.stellar_density > 0.0);
    }

    #[test]
    fn test_nebula_ratio() {
        let file = make_test_file();
        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);

        // 1 out of 3 documented = ~33%
        assert!((metrics.derived.nebula_ratio - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_health_rating() {
        let file = make_test_file();
        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);
        let rating = census.rate_health(&metrics);

        // Clean file should be Stable or Healthy
        assert!(rating == HealthRating::Stable || rating == HealthRating::Healthy);
    }

    #[test]
    fn test_critical_rating() {
        let mut file = make_test_file();
        // Add lots of unknown regions (high dark matter)
        for i in 0..10 {
            file.unknown_regions.push(voyager_ast::ir::UnknownNode {
                span: Span::new(i * 100, i * 100 + 50, i, i + 1),
                reason: Some("Error".to_string()),
                raw_text: None,
            });
        }

        let census = CelestialCensus::new();
        let metrics = census.analyze(&file);
        let rating = census.rate_health(&metrics);

        assert_eq!(rating, HealthRating::HighDarkMatter);
    }

    #[test]
    fn test_metric_collectors() {
        let file = make_test_file();

        let star_metric = StarCountMetric;
        let result = star_metric.analyze(&file);
        assert_eq!(result.value, 3.0);

        let nebulae_metric = NebulaeCountMetric;
        let result = nebulae_metric.analyze(&file);
        assert!(result.value > 0.0);

        let dark_matter_metric = DarkMatterMetric;
        let result = dark_matter_metric.analyze(&file);
        assert_eq!(result.value, 0.0); // No dark matter in clean file
    }

    #[test]
    fn test_galaxy_census() {
        let file1 = make_test_file();
        let file2 = make_test_file();

        let census = CelestialCensus::new();
        let metrics1 = census.analyze(&file1);
        let metrics2 = census.analyze(&file2);

        let mut galaxy = GalaxyCensus::new(".".to_string());
        galaxy.add_file("src/test1.rs", metrics1);
        galaxy.add_file("src/test2.rs", metrics2);
        galaxy.finalize();

        assert_eq!(galaxy.total_files, 2);
        assert_eq!(galaxy.totals.stars.count, 6); // 3 + 3
        assert!(galaxy.constellations.contains_key("src"));
    }

    #[test]
    fn test_census_registry() {
        let registry = build_census_registry();
        assert_eq!(registry.collectors().len(), 6);
        assert!(registry.find("star_count").is_some());
        assert!(registry.find("nebulae_count").is_some());
        assert!(registry.find("dark_matter").is_some());
    }

    // =========================================================================
    // Pattern Fallback Analyzer Tests (Universal Spectrograph Integration)
    // =========================================================================

    #[test]
    fn test_fallback_analyzer_creation() {
        let analyzer = PatternFallbackAnalyzer::new();
        assert!(analyzer.is_supported("rust"));
        assert!(analyzer.is_supported("python"));
        assert!(analyzer.is_supported("simula"));
        assert!(analyzer.is_supported("logo"));
        assert!(analyzer.is_supported("tcl"));
    }

    #[test]
    fn test_fallback_simula_analysis() {
        let analyzer = PatternFallbackAnalyzer::new();

        let source = r#"
class Point;
begin
    real x, y;
    procedure Draw;
    begin
        ! draw the point
    end;
end;
"#;

        let metrics = analyzer.analyze_source("simula", source);
        // Should find: class Point, procedure Draw
        assert!(metrics.stars.count >= 2, "Should find at least 2 stars in Simula code");
        assert_eq!(metrics.stars.types, 1, "Should find 1 class");
        assert!(metrics.stars.functions >= 1, "Should find at least 1 procedure");
    }

    #[test]
    fn test_fallback_logo_analysis() {
        let analyzer = PatternFallbackAnalyzer::new();

        let source = r#"
to square :size
  repeat 4 [forward :size right 90]
end

to circle :radius
  repeat 360 [forward :radius right 1]
end
"#;

        let metrics = analyzer.analyze_source("logo", source);
        // Should find: to square, to circle
        assert_eq!(metrics.stars.count, 2, "Should find 2 Logo procedures");
        assert_eq!(metrics.stars.functions, 2, "All Logo stars are functions");
    }

    #[test]
    fn test_fallback_tcl_analysis() {
        let analyzer = PatternFallbackAnalyzer::new();

        let source = r#"
proc calculateTotal {a b} {
    return [expr {$a + $b}]
}

proc greet {name} {
    puts "Hello, $name!"
}

proc main {} {
    set result [calculateTotal 5 10]
    greet "World"
}
"#;

        let metrics = analyzer.analyze_source("tcl", source);
        // Should find: proc calculateTotal, proc greet, proc main
        assert_eq!(metrics.stars.count, 3, "Should find 3 Tcl procedures");
    }

    #[test]
    fn test_fallback_cobol_analysis() {
        let analyzer = PatternFallbackAnalyzer::new();

        let source = r#"
       IDENTIFICATION DIVISION.
       PROGRAM-ID. HELLO-WORLD.
       PROCEDURE DIVISION.
       MAIN-PROCEDURE SECTION.
           DISPLAY "Hello, World!".
           STOP RUN.
"#;

        let metrics = analyzer.analyze_source("cobol", source);
        // Should find procedure division, section
        assert!(metrics.stars.count >= 1, "Should find at least 1 COBOL section");
    }

    #[test]
    fn test_fallback_hemisphere_classification() {
        let analyzer = PatternFallbackAnalyzer::new();

        assert_eq!(analyzer.get_hemisphere("rust"), Some(Hemisphere::Logic));
        assert_eq!(analyzer.get_hemisphere("html"), Some(Hemisphere::Interface));
        assert_eq!(analyzer.get_hemisphere("bash"), Some(Hemisphere::Automation));
        assert_eq!(analyzer.get_hemisphere("sql"), Some(Hemisphere::Data));
        assert_eq!(analyzer.get_hemisphere("simula"), Some(Hemisphere::Logic));
    }

    #[test]
    fn test_fallback_display_names() {
        let analyzer = PatternFallbackAnalyzer::new();

        assert_eq!(analyzer.display_name("simula"), Some("Simula"));
        assert_eq!(analyzer.display_name("logo"), Some("Logo"));
        assert_eq!(analyzer.display_name("tcl"), Some("Tcl"));
        assert_eq!(analyzer.display_name("cobol"), Some("COBOL"));
        assert_eq!(analyzer.display_name("fortran"), Some("Fortran"));
    }

    #[test]
    fn test_fallback_analyze_by_extension() {
        let analyzer = PatternFallbackAnalyzer::new();

        let rust_source = "fn main() { println!(\"Hello\"); }";
        let metrics = analyzer.analyze_by_extension("rs", rust_source);

        assert!(metrics.is_some());
        let m = metrics.unwrap();
        assert!(m.stars.count >= 1, "Should find main function");
    }

    #[test]
    fn test_fallback_unsupported_language() {
        let analyzer = PatternFallbackAnalyzer::new();

        // Unknown language returns empty metrics
        let metrics = analyzer.analyze_source("unknown_language_xyz", "some code");
        assert_eq!(metrics.stars.count, 0);
        assert_eq!(metrics.total_lines, 0);
    }

    #[test]
    fn test_fallback_comment_counting() {
        let analyzer = PatternFallbackAnalyzer::new();

        let source = r#"
# This is a comment
def hello():
    # Another comment
    pass
# Final comment
"#;

        let metrics = analyzer.analyze_source("python", source);
        // Should count some comments
        assert!(metrics.nebulae.comment_lines >= 1, "Should count Python comments");
    }
}
