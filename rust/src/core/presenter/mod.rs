//! Intelligent Presenter Module
//!
//! Transforms raw analysis results into delightful, user-friendly output.
//! Uses emojis, progressive disclosure, and semantic transparency.
//!
//! # Design Philosophy
//!
//! - **No jargon by default**: Technical terms hidden unless requested
//! - **Progressive disclosure**: Start simple, reveal details on demand
//! - **Visual hierarchy**: Emojis guide the eye to what matters
//! - **Actionable output**: Always suggest next steps

pub mod emoji_formatter;
pub mod transparency;

pub use emoji_formatter::{EmojiFormatter, Theme};
pub use transparency::SemanticTransparency;

use crate::core::orchestrator::DetailLevel;
use crate::core::census::{GalaxyCensus, HealthRating, CensusMetrics};

// =============================================================================
// Drift Info (v1.1.0 - Stellar Drift)
// =============================================================================

/// Information about temporal drift for mission log display.
#[derive(Debug, Clone, Default)]
pub struct DriftInfo {
    /// Galaxy age in days
    pub galaxy_age_days: u64,
    /// Galaxy age in years
    pub galaxy_age_years: f64,
    /// Stellar drift rate per year (percentage)
    pub drift_rate_per_year: f64,
    /// Number of ancient stars (dormant > 2 years)
    pub ancient_stars: usize,
    /// Number of core ancient stars
    pub core_ancient_stars: usize,
    /// Number of new stars (< 90 days)
    pub new_stars: usize,
    /// New star percentage of total
    pub new_star_percentage: f64,
}

// =============================================================================
// Intelligent Presenter
// =============================================================================

/// The intelligent presenter transforms analysis into user-friendly output.
pub struct IntelligentPresenter {
    /// Emoji formatter for visual output
    emoji_formatter: EmojiFormatter,
    /// Semantic transparency for technical details
    transparency: SemanticTransparency,
    /// Current detail level
    detail_level: DetailLevel,
}

impl Default for IntelligentPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl IntelligentPresenter {
    /// Create a new presenter with default settings.
    pub fn new() -> Self {
        Self {
            emoji_formatter: EmojiFormatter::new(),
            transparency: SemanticTransparency::new(),
            detail_level: DetailLevel::Smart,
        }
    }

    /// Create a presenter with a specific detail level.
    pub fn with_detail_level(mut self, level: DetailLevel) -> Self {
        self.detail_level = level;
        self
    }

    /// Enable semantic transparency (technical details).
    pub fn with_transparency(mut self, enabled: bool) -> Self {
        self.transparency = if enabled {
            SemanticTransparency::new().with_details(true)
        } else {
            SemanticTransparency::new()
        };
        self
    }

    /// Format an exploration summary.
    pub fn format_exploration_summary(
        &self,
        intent: &str,
        file_count: usize,
        language_count: usize,
        analysis_time_ms: u64,
        confidence: f32,
    ) -> String {
        let mut output = String::new();

        // Header with intent
        output.push_str(&format!(
            "{} {} Exploration\n",
            self.emoji_formatter.intent_emoji(intent),
            capitalize_first(intent)
        ));

        // View indicator with confidence
        output.push_str(&format!(
            "{} View: Architecture Lens ({})\n",
            self.emoji_formatter.view_emoji(),
            self.emoji_formatter.confidence_indicator(confidence)
        ));

        // Analysis stats
        let time_str = if analysis_time_ms > 1000 {
            format!("{:.1}s", analysis_time_ms as f64 / 1000.0)
        } else {
            format!("{}ms", analysis_time_ms)
        };

        output.push_str(&format!(
            "{} Analyzed: {} files across {} language{} ({})\n",
            self.emoji_formatter.power_emoji(),
            file_count,
            language_count,
            if language_count == 1 { "" } else { "s" },
            time_str
        ));

        output
    }

    /// Format key insights.
    pub fn format_insights(&self, insights: &[String]) -> String {
        if insights.is_empty() {
            return String::new();
        }

        let mut output = format!("{} Key Insights:\n", self.emoji_formatter.insight_emoji());

        let max_insights = match self.detail_level {
            DetailLevel::Summary => 2,
            DetailLevel::Smart => 3,
            DetailLevel::Detailed => insights.len(),
        };

        for insight in insights.iter().take(max_insights) {
            output.push_str(&format!("  {} {}\n", self.emoji_formatter.bullet(), insight));
        }

        if insights.len() > max_insights {
            output.push_str(&format!(
                "  {} {} more insight{} available with --detail detailed\n",
                self.emoji_formatter.hint_emoji(),
                insights.len() - max_insights,
                if insights.len() - max_insights == 1 { "" } else { "s" }
            ));
        }

        output
    }

    /// Format a starting point recommendation.
    pub fn format_starting_point(&self, symbol: &str, reason: &str) -> String {
        format!(
            "{} Start with: {} - {}\n",
            self.emoji_formatter.navigation_emoji(),
            symbol,
            reason
        )
    }

    /// Format a tip for progressive disclosure.
    pub fn format_tip(&self, tip: &str) -> String {
        format!(
            "{} Tip: {}\n",
            self.emoji_formatter.hint_emoji(),
            tip
        )
    }

    /// Format technical details (only if transparency is enabled).
    pub fn format_technical_details(&self, details: &[(&str, &str)]) -> String {
        self.transparency.format_details(details)
    }

    /// Get the emoji formatter.
    pub fn emoji_formatter(&self) -> &EmojiFormatter {
        &self.emoji_formatter
    }

    /// Get the current detail level.
    pub fn detail_level(&self) -> DetailLevel {
        self.detail_level
    }

    // =========================================================================
    // Voyager Mission Log Format
    // =========================================================================

    /// Format a complete Voyager Mission Log summary.
    ///
    /// This creates the immersive "Observatory" experience with:
    /// - Telescope pointing at project
    /// - Two hemispheres detection (top languages)
    /// - Spectral filter (lens) status
    /// - Fuel gauge (token budget)
    /// - Points of interest
    /// - Transmission status
    pub fn format_mission_log(
        &self,
        project_name: &str,
        hemispheres: (&str, Option<&str>),
        lens: &str,
        confidence: f32,
        tokens_used: usize,
        token_budget: usize,
        poi_count: usize,
        nebula_name: Option<&str>,
    ) -> String {
        let mut output = String::new();

        // Line 1: Observatory pointing
        output.push_str(&format!(
            "{} Observatory pointed at {}.\n",
            self.emoji_formatter.telescope(),
            project_name
        ));

        // Line 2: Two hemispheres
        let hemisphere_str = match hemispheres.1 {
            Some(lang2) => format!("{} | {}", hemispheres.0, lang2),
            None => hemispheres.0.to_string(),
        };
        output.push_str(&format!(
            "{} Two hemispheres detected: {}.\n",
            self.emoji_formatter.notable_star(),
            hemisphere_str
        ));

        // Line 3: Spectral filter
        let confidence_label = if confidence > 0.8 {
            "High Confidence"
        } else if confidence > 0.5 {
            "Medium Confidence"
        } else {
            "Low Confidence"
        };
        output.push_str(&format!(
            "{} Spectral Filter '{}' applied ({}).\n",
            self.emoji_formatter.view_emoji(),
            capitalize_first(lens),
            confidence_label
        ));

        // Line 4: Fuel gauge
        let fuel_pct = if token_budget > 0 {
            (tokens_used as f64 / token_budget as f64 * 100.0) as usize
        } else {
            0
        };
        output.push_str(&format!(
            "{} Fuel: {} / {} tokens ({}%).\n",
            self.emoji_formatter.fuel(),
            format_number(tokens_used),
            format_number(token_budget),
            fuel_pct
        ));

        // Line 5: Points of interest
        if poi_count > 0 {
            let nebula_str = nebula_name.unwrap_or("primary cluster");
            output.push_str(&format!(
                "{} {} Points of Interest identified in the '{}'.\n",
                self.emoji_formatter.gem(),
                poi_count,
                nebula_str
            ));
        }

        // Line 6: Transmission
        output.push_str(&format!(
            "{} Teleporting context sample to LLM base...\n",
            self.emoji_formatter.transmit()
        ));

        output
    }

    /// Format an extended Voyager Mission Log with temporal drift metrics.
    ///
    /// This adds Galaxy Age, Stellar Drift, and Ancient Star information
    /// when temporal analysis is available.
    pub fn format_mission_log_with_drift(
        &self,
        project_name: &str,
        hemispheres: (&str, Option<&str>),
        lens: &str,
        confidence: f32,
        tokens_used: usize,
        token_budget: usize,
        poi_count: usize,
        nebula_name: Option<&str>,
        drift_info: Option<DriftInfo>,
    ) -> String {
        let mut output = self.format_mission_log(
            project_name,
            hemispheres,
            lens,
            confidence,
            tokens_used,
            token_budget,
            poi_count,
            nebula_name,
        );

        // Add temporal/drift section if available
        if let Some(drift) = drift_info {
            output.push_str("\n");
            output.push_str(&format!(
                "{} Temporal Analysis\n",
                self.emoji_formatter.insight_emoji()
            ));
            output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

            // Galaxy age
            let age_display = if drift.galaxy_age_years >= 1.0 {
                format!("{:.1} years", drift.galaxy_age_years)
            } else if drift.galaxy_age_days > 0 {
                format!("{} days", drift.galaxy_age_days)
            } else {
                "Unknown".to_string()
            };
            output.push_str(&format!(
                "  {} Galaxy Age: {}\n",
                self.emoji_formatter.notable_star(),
                age_display
            ));

            // Stellar drift
            let drift_health = if drift.drift_rate_per_year < 20.0 {
                ("✅", "Stable")
            } else if drift.drift_rate_per_year < 50.0 {
                ("📊", "Active")
            } else if drift.drift_rate_per_year < 100.0 {
                ("🚀", "Expanding")
            } else {
                ("🌋", "Volcanic")
            };
            output.push_str(&format!(
                "  {} Stellar Drift: {:.1}%/year {}\n",
                drift_health.0,
                drift.drift_rate_per_year,
                drift_health.1
            ));

            // Ancient stars
            if drift.ancient_stars > 0 {
                output.push_str(&format!(
                    "  {} Ancient Stars: {} discovered ({} core files)\n",
                    self.emoji_formatter.gem(),
                    drift.ancient_stars,
                    drift.core_ancient_stars
                ));
            }

            // New stars
            if drift.new_stars > 0 {
                output.push_str(&format!(
                    "  🌠 New Stars: {} ({:.0}% of logic units)\n",
                    drift.new_stars,
                    drift.new_star_percentage
                ));
            }
        }

        output
    }

    // =========================================================================
    // Governance Report (Phase 1C: Celestial Census)
    // =========================================================================

    /// Format a Governance Report from the Celestial Census.
    ///
    /// This adds health indicators to the Mission Log using celestial terminology:
    /// - ⭐ Healthy: Balanced star/nebula ratio
    /// - ✅ Stable: Low dark matter, good structure
    /// - ⚠️ High Dark Matter: Significant unparsed or complex regions
    /// - 🔴 Critical: Red Giants detected (large files with issues)
    pub fn format_governance_report(&self, galaxy: &GalaxyCensus) -> String {
        let mut output = String::new();

        // Header
        output.push_str("\n");
        output.push_str(&format!(
            "{} Governance Report\n",
            self.emoji_formatter.notable_star()
        ));
        output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

        // Overall health rating
        if let Some(rating) = &galaxy.rating {
            output.push_str(&format!(
                "{} Overall Health: {}\n",
                self.format_health_indicator(rating),
                rating.description()
            ));
        }

        // Summary metrics
        output.push_str("\n");
        output.push_str(&format!(
            "{} Stars (Logic): {} ({} functions, {} methods)\n",
            self.emoji_formatter.notable_star(),
            galaxy.totals.stars.count,
            galaxy.totals.stars.functions,
            galaxy.totals.stars.methods
        ));
        output.push_str(&format!(
            "{} Nebulae (Docs): {} lines ({:.0}% coverage)\n",
            self.emoji_formatter.gem(),
            galaxy.totals.nebulae.doc_lines + galaxy.totals.nebulae.comment_lines,
            galaxy.totals.derived.nebula_ratio * 100.0
        ));
        output.push_str(&format!(
            "{} Dark Matter: {} regions ({} volcanic)\n",
            self.format_dark_matter_indicator(&galaxy.totals),
            galaxy.totals.dark_matter.unknown_regions,
            galaxy.totals.dark_matter.volcanic_regions
        ));

        // Constellation breakdown (if detailed)
        if matches!(self.detail_level, DetailLevel::Detailed | DetailLevel::Smart) {
            output.push_str("\n");
            output.push_str(&format!(
                "{} Constellations ({}):\n",
                self.emoji_formatter.view_emoji(),
                galaxy.constellations.len()
            ));

            for (path, constellation) in &galaxy.constellations {
                let indicator = if let Some(rating) = &constellation.rating {
                    self.format_health_indicator(rating)
                } else {
                    "  ".to_string()
                };

                output.push_str(&format!(
                    "  {} {}: {} stars, {} files\n",
                    indicator,
                    path,
                    constellation.totals.stars.count,
                    constellation.file_count
                ));

                // Show Red Giants (if any)
                if !constellation.red_giants.is_empty() && matches!(self.detail_level, DetailLevel::Detailed) {
                    for rg in &constellation.red_giants {
                        output.push_str(&format!(
                            "      {} Red Giant: {}\n",
                            self.emoji_formatter.insight_emoji(),
                            rg
                        ));
                    }
                }
            }
        }

        // Recommendations
        output.push_str("\n");
        output.push_str(&format!(
            "{} Recommendations:\n",
            self.emoji_formatter.hint_emoji()
        ));

        // Generate recommendations based on metrics
        let recommendations = self.generate_recommendations(galaxy);
        for rec in recommendations.iter().take(3) {
            output.push_str(&format!("  {} {}\n", self.emoji_formatter.bullet(), rec));
        }

        output
    }

    /// Format a health indicator emoji for a rating.
    pub fn format_health_indicator(&self, rating: &HealthRating) -> String {
        match rating {
            HealthRating::Healthy => "⭐".to_string(),
            HealthRating::Stable => "✅".to_string(),
            HealthRating::HighDarkMatter => "⚠️".to_string(),
            HealthRating::Critical => "🔴".to_string(),
        }
    }

    /// Format dark matter indicator based on severity.
    fn format_dark_matter_indicator(&self, metrics: &CensusMetrics) -> String {
        if metrics.dark_matter.unknown_regions == 0 && metrics.dark_matter.volcanic_regions == 0 {
            "✨".to_string()  // Clean
        } else if metrics.derived.dark_matter_ratio < 0.05 {
            "🌑".to_string()  // Minor dark matter
        } else {
            "⚫".to_string()  // Significant dark matter
        }
    }

    /// Generate recommendations based on census metrics.
    fn generate_recommendations(&self, galaxy: &GalaxyCensus) -> Vec<String> {
        let mut recs = Vec::new();

        // Check documentation coverage
        if galaxy.totals.derived.nebula_ratio < 0.2 {
            recs.push("Increase documentation coverage (currently below 20%)".to_string());
        }

        // Check for volcanic regions
        if galaxy.totals.dark_matter.volcanic_regions > 5 {
            recs.push(format!(
                "Review {} volcanic regions (deep nesting > 4 levels)",
                galaxy.totals.dark_matter.volcanic_regions
            ));
        }

        // Check for unknown regions
        if galaxy.totals.dark_matter.unknown_regions > 0 {
            recs.push(format!(
                "Investigate {} unparsed regions (possible syntax issues)",
                galaxy.totals.dark_matter.unknown_regions
            ));
        }

        // Check stellar density
        if galaxy.totals.derived.stellar_density > 30.0 {
            recs.push(format!(
                "Consider refactoring - high stellar density ({:.1} stars/1k LOC)",
                galaxy.totals.derived.stellar_density
            ));
        }

        // Count red giants
        let red_giant_count: usize = galaxy.constellations.values()
            .map(|c| c.red_giants.len())
            .sum();
        if red_giant_count > 0 {
            recs.push(format!(
                "Review {} Red Giants (large files with high complexity or low docs)",
                red_giant_count
            ));
        }

        // Default recommendation if all looks good
        if recs.is_empty() {
            recs.push("Codebase health is good - continue current practices".to_string());
        }

        recs
    }

    // =========================================================================
    // Phase 2: Temporal Narrative (Chronos Engine)
    // =========================================================================

    /// Format a temporal narrative for the Mission Log.
    ///
    /// Adds the "Geological Strata" story to the output with celestial terminology:
    /// - ⏳ Temporal Scan: History depth and observer count
    /// - 🌋 Volcanic Activity: High churn regions (Supernovas, Tectonic Shifts)
    /// - 📜 Ancient Stars: Dormant core files in deep strata
    ///
    /// # Arguments
    /// * `galaxy_age_days` - Total age of the repository in days
    /// * `total_observations` - Total number of chronos events (commits)
    /// * `observer_count` - Number of unique observers (contributors)
    /// * `supernovas` - Files with extreme recent activity
    /// * `tectonic_shifts` - High-risk files (churn + complexity)
    /// * `ancient_stars` - Dormant but core files
    #[cfg(feature = "temporal")]
    pub fn format_temporal_narrative(
        &self,
        galaxy_age_days: u64,
        total_observations: usize,
        observer_count: usize,
        supernovas: &[crate::core::temporal::Supernova],
        tectonic_shifts: &[crate::core::temporal::TectonicShift],
        ancient_stars: &[crate::core::temporal::AncientStar],
    ) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        // Header
        writeln!(output).ok();
        writeln!(output, "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━").ok();

        // Line 1: Temporal Scan summary
        let years = galaxy_age_days as f64 / 365.0;
        let years_str = if years >= 1.0 {
            format!("{:.1} years", years)
        } else {
            format!("{} days", galaxy_age_days)
        };
        writeln!(
            output,
            "⏳ Temporal Scan: {} of history analyzed ({} observations by {} observers).",
            years_str, total_observations, observer_count
        ).ok();

        // Line 2: Volcanic activity (Supernovas + Tectonic Shifts)
        let volcanic_count = supernovas.len() + tectonic_shifts.len();
        if volcanic_count > 0 {
            if !supernovas.is_empty() {
                let nova_names: Vec<&str> = supernovas.iter()
                    .take(2)
                    .map(|s| s.path.rsplit('/').next().unwrap_or(&s.path))
                    .collect();
                let nova_summary = if supernovas.len() > 2 {
                    format!("{} and {} others", nova_names.join(", "), supernovas.len() - 2)
                } else {
                    nova_names.join(", ")
                };
                writeln!(
                    output,
                    "🌋 Volcanic Activity: {} Supernova{} detected ({}). Active refactoring zone!",
                    supernovas.len(),
                    if supernovas.len() == 1 { "" } else { "s" },
                    nova_summary
                ).ok();
            }

            if !tectonic_shifts.is_empty() {
                let shift_count = tectonic_shifts.len();
                let high_risk_count = tectonic_shifts.iter().filter(|s| s.risk_score > 0.7).count();
                if high_risk_count > 0 {
                    writeln!(
                        output,
                        "⚠️  Tectonic Stress: {} shift{} identified ({} high-risk). Consider stabilization.",
                        shift_count,
                        if shift_count == 1 { "" } else { "s" },
                        high_risk_count
                    ).ok();
                } else {
                    writeln!(
                        output,
                        "🌍 Minor Tectonic Shifts: {} region{} with elevated churn.",
                        shift_count,
                        if shift_count == 1 { "" } else { "s" }
                    ).ok();
                }
            }
        } else {
            writeln!(output, "🌍 Geological Stability: No volcanic activity detected.").ok();
        }

        // Line 3: Ancient Stars (dormant core files)
        let core_ancient: Vec<_> = ancient_stars.iter().filter(|a| a.is_core).collect();
        if !core_ancient.is_empty() {
            writeln!(
                output,
                "📜 {} Ancient Star{} identified in the deep strata (core files dormant > 2 years).",
                core_ancient.len(),
                if core_ancient.len() == 1 { "" } else { "s" }
            ).ok();

            // Show top 2 ancient stars in detailed mode
            if matches!(self.detail_level, DetailLevel::Detailed) {
                for ancient in core_ancient.iter().take(2) {
                    let file_name = ancient.path.rsplit('/').next().unwrap_or(&ancient.path);
                    writeln!(
                        output,
                        "   📜 {} (dormant {} days, {} stars)",
                        file_name, ancient.dormant_days, ancient.star_count
                    ).ok();
                }
            }
        } else if !ancient_stars.is_empty() {
            writeln!(
                output,
                "📜 {} dormant file{} in archaeological strata (non-core, low priority).",
                ancient_stars.len(),
                if ancient_stars.len() == 1 { "" } else { "s" }
            ).ok();
        }

        output
    }

    /// Format a temporal narrative (non-temporal fallback - empty output).
    #[cfg(not(feature = "temporal"))]
    pub fn format_temporal_narrative<S, T, A>(
        &self,
        _galaxy_age_days: u64,
        _total_observations: usize,
        _observer_count: usize,
        _supernovas: &[S],
        _tectonic_shifts: &[T],
        _ancient_stars: &[A],
    ) -> String {
        String::new()
    }

    /// Format temporal narrative from a TemporalCensus (convenience method).
    #[cfg(feature = "temporal")]
    pub fn format_temporal_narrative_from_census(
        &self,
        census: &crate::core::temporal::TemporalCensus,
    ) -> String {
        self.format_temporal_narrative(
            census.galaxy_age_days,
            census.total_observations,
            census.observer_count,
            &census.supernovas,
            &census.tectonic_shifts,
            &census.ancient_stars,
        )
    }

    // =========================================================================
    // Phase 3: Plugin Ecosystem Summary
    // =========================================================================

    /// Generate a plugin summary for the Mission Log.
    ///
    /// Shows loaded plugins with sandbox status.
    ///
    /// # Arguments
    /// * `plugin_count` - Number of loaded plugins
    /// * `loaded_plugins` - Names of loaded plugins
    /// * `sandbox_active` - Whether the Iron Sandbox is active
    pub fn format_plugin_summary(
        &self,
        plugin_count: usize,
        loaded_plugins: &[String],
        sandbox_active: bool,
    ) -> String {
        use std::fmt::Write;
        let mut output = String::new();

        if plugin_count == 0 {
            return String::from("🔌 No external optics detected.\n");
        }

        writeln!(output, "🔌 External Optics: {} community plugin{} loaded",
            plugin_count,
            if plugin_count == 1 { "" } else { "s" }
        ).ok();

        // Show plugin names (up to 5)
        let show_count = loaded_plugins.len().min(5);
        for (i, name) in loaded_plugins.iter().take(show_count).enumerate() {
            let prefix = if i == show_count - 1 && loaded_plugins.len() <= 5 {
                "└─"
            } else {
                "├─"
            };
            writeln!(output, "   {} {}", prefix, name).ok();
        }

        if loaded_plugins.len() > 5 {
            writeln!(output, "   └─ ... and {} more", loaded_plugins.len() - 5).ok();
        }

        // Sandbox status
        if sandbox_active {
            writeln!(output, "🛡️ Plugin sandbox: Active (10MB memory, 100ms timeout)").ok();
        } else {
            writeln!(output, "⚠️ Plugin sandbox: Inactive").ok();
        }

        output
    }

    /// Format plugin summary from a PluginEngine (convenience method).
    pub fn format_plugin_summary_from_engine(&self, engine: &crate::core::plugins::PluginEngine) -> String {
        let names: Vec<String> = engine.plugin_names().iter().map(|s| s.to_string()).collect();
        let sandbox_active = crate::core::plugins::is_plugins_available();
        self.format_plugin_summary(engine.plugin_count(), &names, sandbox_active)
    }

    /// Detect the two hemispheres (top 2 languages) from a language distribution.
    pub fn detect_hemispheres(languages: &[(String, usize)]) -> (String, Option<String>) {
        let mut sorted: Vec<_> = languages.to_vec();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        let primary = sorted.first()
            .map(|(lang, _)| format_language_name(lang))
            .unwrap_or_else(|| "Unknown".to_string());

        let secondary = sorted.get(1)
            .map(|(lang, _)| format_language_name(lang));

        (primary, secondary)
    }
}

/// Format a language name for display.
fn format_language_name(lang: &str) -> String {
    match lang.to_lowercase().as_str() {
        "rust" => "Logic: Rust".to_string(),
        "python" => "Logic: Python".to_string(),
        "typescript" => "Interface: TypeScript".to_string(),
        "javascript" => "Interface: JavaScript".to_string(),
        "html" | "css" => "Presentation: Web".to_string(),
        "shell" | "bash" => "Automation: Shell".to_string(),
        "go" => "Logic: Go".to_string(),
        "java" => "Logic: Java".to_string(),
        "c" | "cpp" => "Systems: C/C++".to_string(),
        "sql" => "Data: SQL".to_string(),
        "markdown" => "Docs: Markdown".to_string(),
        "json" | "yaml" | "toml" => "Config: Structured".to_string(),
        _ => format!("Code: {}", capitalize_first(lang)),
    }
}

/// Format a number with thousand separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Capitalize the first letter of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_presenter_new() {
        let presenter = IntelligentPresenter::new();
        assert_eq!(presenter.detail_level(), DetailLevel::Smart);
    }

    #[test]
    fn test_format_exploration_summary() {
        let presenter = IntelligentPresenter::new();
        let output = presenter.format_exploration_summary(
            "business-logic",
            42,
            3,
            2100,
            0.85,
        );

        assert!(output.contains("Business-logic Exploration"));
        assert!(output.contains("42 files"));
        assert!(output.contains("3 languages"));
        assert!(output.contains("2.1s"));
    }

    #[test]
    fn test_format_insights_limited() {
        let presenter = IntelligentPresenter::new()
            .with_detail_level(DetailLevel::Summary);

        let insights = vec![
            "Insight 1".to_string(),
            "Insight 2".to_string(),
            "Insight 3".to_string(),
            "Insight 4".to_string(),
        ];

        let output = presenter.format_insights(&insights);

        // Summary mode should show only 2 insights
        assert!(output.contains("Insight 1"));
        assert!(output.contains("Insight 2"));
        assert!(!output.contains("Insight 3"));
        assert!(output.contains("2 more insights"));
    }

    #[test]
    fn test_format_starting_point() {
        let presenter = IntelligentPresenter::new();
        let output = presenter.format_starting_point(
            "calculate_total",
            "Core business calculation",
        );

        assert!(output.contains("calculate_total"));
        assert!(output.contains("Core business calculation"));
    }

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    // =========================================================================
    // Voyager Mission Log Tests (Stage 3)
    // =========================================================================

    #[test]
    fn test_mission_log_contains_telescope_emoji() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "my_project",
            ("Logic: Rust", Some("Interface: TypeScript")),
            "architecture",
            0.85,
            50_000,
            100_000,
            15,
            Some("Core Engine"),
        );

        // Verify telescope emoji at start
        assert!(log.contains("🔭"), "Mission log should contain telescope emoji");
        assert!(log.contains("Observatory pointed at my_project"));
    }

    #[test]
    fn test_mission_log_contains_hemispheres() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Rust", Some("Interface: TypeScript")),
            "debug",
            0.7,
            25_000,
            50_000,
            10,
            None,
        );

        assert!(log.contains("✨"), "Mission log should contain notable star emoji");
        assert!(log.contains("Two hemispheres detected"));
        assert!(log.contains("Logic: Rust | Interface: TypeScript"));
    }

    #[test]
    fn test_mission_log_contains_spectral_filter() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Python", None),
            "security",
            0.9,
            10_000,
            20_000,
            5,
            None,
        );

        assert!(log.contains("🔭"), "Mission log should contain view emoji");
        assert!(log.contains("Spectral Filter 'Security' applied"));
        assert!(log.contains("High Confidence"));
    }

    #[test]
    fn test_mission_log_fuel_gauge() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Go", None),
            "minimal",
            0.5,
            50_000,
            100_000,
            3,
            None,
        );

        assert!(log.contains("🔋"), "Mission log should contain fuel emoji");
        assert!(log.contains("Fuel:"));
        assert!(log.contains("50,000"));
        assert!(log.contains("100,000"));
        assert!(log.contains("50%"));
    }

    #[test]
    fn test_mission_log_points_of_interest() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Java", None),
            "architecture",
            0.8,
            75_000,
            100_000,
            12,
            Some("Service Layer"),
        );

        assert!(log.contains("💎"), "Mission log should contain gem emoji");
        assert!(log.contains("12 Points of Interest"));
        assert!(log.contains("'Service Layer'"));
    }

    #[test]
    fn test_mission_log_transmission() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Automation: Shell", None),
            "auto",
            0.6,
            5_000,
            10_000,
            2,
            None,
        );

        assert!(log.contains("📡"), "Mission log should contain transmit emoji");
        assert!(log.contains("Teleporting context sample to LLM base"));
    }

    #[test]
    fn test_detect_hemispheres_single_language() {
        let languages = vec![("rust".to_string(), 50)];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Logic: Rust");
        assert!(secondary.is_none());
    }

    #[test]
    fn test_detect_hemispheres_multiple_languages() {
        let languages = vec![
            ("typescript".to_string(), 30),
            ("python".to_string(), 25),
            ("shell".to_string(), 10),
        ];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Interface: TypeScript");
        assert_eq!(secondary, Some("Logic: Python".to_string()));
    }

    #[test]
    fn test_detect_hemispheres_empty() {
        let languages: Vec<(String, usize)> = vec![];
        let (primary, secondary) = IntelligentPresenter::detect_hemispheres(&languages);

        assert_eq!(primary, "Unknown");
        assert!(secondary.is_none());
    }

    #[test]
    fn test_format_language_name_categories() {
        // Logic languages
        assert_eq!(format_language_name("rust"), "Logic: Rust");
        assert_eq!(format_language_name("python"), "Logic: Python");
        assert_eq!(format_language_name("go"), "Logic: Go");
        assert_eq!(format_language_name("java"), "Logic: Java");

        // Interface languages
        assert_eq!(format_language_name("typescript"), "Interface: TypeScript");
        assert_eq!(format_language_name("javascript"), "Interface: JavaScript");

        // Presentation
        assert_eq!(format_language_name("html"), "Presentation: Web");
        assert_eq!(format_language_name("css"), "Presentation: Web");

        // Automation
        assert_eq!(format_language_name("shell"), "Automation: Shell");
        assert_eq!(format_language_name("bash"), "Automation: Shell");

        // Systems
        assert_eq!(format_language_name("c"), "Systems: C/C++");
        assert_eq!(format_language_name("cpp"), "Systems: C/C++");

        // Data
        assert_eq!(format_language_name("sql"), "Data: SQL");

        // Config
        assert_eq!(format_language_name("json"), "Config: Structured");
        assert_eq!(format_language_name("yaml"), "Config: Structured");
        assert_eq!(format_language_name("toml"), "Config: Structured");

        // Docs
        assert_eq!(format_language_name("markdown"), "Docs: Markdown");

        // Unknown
        assert_eq!(format_language_name("cobol"), "Code: Cobol");
    }

    #[test]
    fn test_format_number_with_separators() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1_000), "1,000");
        assert_eq!(format_number(10_000), "10,000");
        assert_eq!(format_number(100_000), "100,000");
        assert_eq!(format_number(1_000_000), "1,000,000");
    }

    #[test]
    fn test_mission_log_no_jargon() {
        let presenter = IntelligentPresenter::new();
        let log = presenter.format_mission_log(
            "project",
            ("Logic: Rust", None),
            "architecture",
            0.9,
            50_000,
            100_000,
            10,
            Some("Core"),
        );

        // Verify no technical jargon in default output
        assert!(!log.contains("Substrate"));
        assert!(!log.contains("EMA"));
        assert!(!log.contains("vectorize"));
        assert!(!log.contains("semantic"));
    }

    // =========================================================================
    // Governance Report Tests (Phase 1C)
    // =========================================================================

    #[test]
    fn test_format_health_indicator() {
        let presenter = IntelligentPresenter::new();

        assert_eq!(presenter.format_health_indicator(&HealthRating::Healthy), "⭐");
        assert_eq!(presenter.format_health_indicator(&HealthRating::Stable), "✅");
        assert_eq!(presenter.format_health_indicator(&HealthRating::HighDarkMatter), "⚠️");
        assert_eq!(presenter.format_health_indicator(&HealthRating::Critical), "🔴");
    }

    #[test]
    fn test_governance_report_contains_header() {
        use crate::core::census::{GalaxyCensus, CelestialCensus};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        assert!(report.contains("Governance Report"));
        assert!(report.contains("━━━"));
    }

    #[test]
    fn test_governance_report_shows_stars() {
        use crate::core::census::{GalaxyCensus, CensusMetrics, StarMetrics};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());

        // Add file with known star count
        let mut metrics = CensusMetrics::default();
        metrics.stars = StarMetrics {
            count: 10,
            functions: 6,
            methods: 4,
            types: 2,
            constants: 1,
        };
        galaxy.add_file("src/test.rs", metrics);
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        assert!(report.contains("Stars (Logic)"));
        assert!(report.contains("10"));
        assert!(report.contains("6 functions"));
        assert!(report.contains("4 methods"));
    }

    #[test]
    fn test_governance_report_shows_nebulae() {
        use crate::core::census::{GalaxyCensus, CensusMetrics, NebulaeMetrics};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());

        let mut metrics = CensusMetrics::default();
        metrics.nebulae = NebulaeMetrics {
            doc_lines: 50,
            comment_lines: 20,
            documented_stars: 5,
            total_stars: 10,
        };
        metrics.total_lines = 100;
        galaxy.add_file("src/lib.rs", metrics);
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        assert!(report.contains("Nebulae (Docs)"));
        assert!(report.contains("70 lines"));  // 50 + 20
    }

    #[test]
    fn test_governance_report_shows_dark_matter() {
        use crate::core::census::{GalaxyCensus, CensusMetrics, DarkMatterMetrics};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());

        let mut metrics = CensusMetrics::default();
        metrics.dark_matter = DarkMatterMetrics {
            unknown_regions: 3,
            unknown_bytes: 150,
            volcanic_regions: 2,
            max_nesting_depth: 6,
            parameter_heavy: 1,
        };
        metrics.total_lines = 100;
        galaxy.add_file("src/complex.rs", metrics);
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        assert!(report.contains("Dark Matter"));
        assert!(report.contains("3 regions"));
        assert!(report.contains("2 volcanic"));
    }

    #[test]
    fn test_governance_report_recommendations() {
        use crate::core::census::{GalaxyCensus, CensusMetrics, DarkMatterMetrics};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());

        // Create metrics that trigger recommendations
        let mut metrics = CensusMetrics::default();
        metrics.dark_matter = DarkMatterMetrics {
            unknown_regions: 5,
            unknown_bytes: 500,
            volcanic_regions: 10,
            max_nesting_depth: 8,
            parameter_heavy: 3,
        };
        metrics.total_lines = 500;
        galaxy.add_file("src/messy.rs", metrics);
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        assert!(report.contains("Recommendations"));
        // Should have recommendations for volcanic regions and unknown regions
        assert!(report.contains("volcanic regions") || report.contains("unparsed regions"));
    }

    #[test]
    fn test_governance_report_healthy_codebase() {
        use crate::core::census::{GalaxyCensus, CensusMetrics, NebulaeMetrics, StarMetrics};

        let presenter = IntelligentPresenter::new();
        let mut galaxy = GalaxyCensus::new(".".to_string());

        // Create healthy metrics
        let mut metrics = CensusMetrics::default();
        metrics.stars = StarMetrics {
            count: 5,
            functions: 5,
            methods: 0,
            types: 0,
            constants: 0,
        };
        metrics.nebulae = NebulaeMetrics {
            doc_lines: 20,
            comment_lines: 10,
            documented_stars: 4,
            total_stars: 5,
        };
        metrics.total_lines = 200;
        galaxy.add_file("src/clean.rs", metrics);
        galaxy.finalize();

        let report = presenter.format_governance_report(&galaxy);

        // Should have positive recommendation
        assert!(report.contains("Recommendations"));
    }

    // =========================================================================
    // Temporal Narrative Tests (Phase 2)
    // =========================================================================

    #[cfg(feature = "temporal")]
    mod temporal_tests {
        use super::*;
        use crate::core::temporal::{Supernova, TectonicShift, AncientStar};

        #[test]
        fn test_temporal_narrative_contains_time_emoji() {
            let presenter = IntelligentPresenter::new();
            let narrative = presenter.format_temporal_narrative(
                730,  // 2 years
                500,
                5,
                &[],
                &[],
                &[],
            );

            assert!(narrative.contains("⏳"), "Should contain hourglass emoji");
            assert!(narrative.contains("Temporal Scan"));
            assert!(narrative.contains("2.0 years"));
            assert!(narrative.contains("500 observations"));
            assert!(narrative.contains("5 observers"));
        }

        #[test]
        fn test_temporal_narrative_short_history_shows_days() {
            let presenter = IntelligentPresenter::new();
            let narrative = presenter.format_temporal_narrative(
                45,  // Less than a year
                20,
                2,
                &[],
                &[],
                &[],
            );

            assert!(narrative.contains("45 days"));
            assert!(!narrative.contains("years"));
        }

        #[test]
        fn test_temporal_narrative_supernovas() {
            let presenter = IntelligentPresenter::new();
            let supernovas = vec![
                Supernova {
                    path: "src/core/engine.rs".to_string(),
                    observations_30d: 35,
                    observer_count: 3,
                    lines_changed: 1000,
                    warning: "High activity".to_string(),
                },
            ];

            let narrative = presenter.format_temporal_narrative(
                365,
                100,
                3,
                &supernovas,
                &[],
                &[],
            );

            assert!(narrative.contains("🌋"), "Should contain volcano emoji");
            assert!(narrative.contains("Volcanic Activity"));
            assert!(narrative.contains("1 Supernova"));
            assert!(narrative.contains("engine.rs"));
        }

        #[test]
        fn test_temporal_narrative_multiple_supernovas() {
            let presenter = IntelligentPresenter::new();
            let supernovas = vec![
                Supernova {
                    path: "src/a.rs".to_string(),
                    observations_30d: 40,
                    observer_count: 2,
                    lines_changed: 500,
                    warning: "".to_string(),
                },
                Supernova {
                    path: "src/b.rs".to_string(),
                    observations_30d: 35,
                    observer_count: 2,
                    lines_changed: 400,
                    warning: "".to_string(),
                },
                Supernova {
                    path: "src/c.rs".to_string(),
                    observations_30d: 32,
                    observer_count: 1,
                    lines_changed: 300,
                    warning: "".to_string(),
                },
            ];

            let narrative = presenter.format_temporal_narrative(
                200,
                150,
                4,
                &supernovas,
                &[],
                &[],
            );

            assert!(narrative.contains("3 Supernovas"));
            assert!(narrative.contains("and 1 others"));  // Shows overflow
        }

        #[test]
        fn test_temporal_narrative_tectonic_shifts() {
            let presenter = IntelligentPresenter::new();
            let shifts = vec![
                TectonicShift {
                    path: "src/complex.rs".to_string(),
                    churn_90d: 15,
                    dark_matter_ratio: 0.25,
                    risk_score: 0.8,
                    reason: "High risk".to_string(),
                },
            ];

            let narrative = presenter.format_temporal_narrative(
                400,
                200,
                5,
                &[],
                &shifts,
                &[],
            );

            assert!(narrative.contains("Tectonic Stress"));
            assert!(narrative.contains("1 shift"));
            assert!(narrative.contains("high-risk"));
        }

        #[test]
        fn test_temporal_narrative_ancient_stars() {
            let presenter = IntelligentPresenter::new();
            let ancient = vec![
                AncientStar {
                    path: "src/core/legacy.rs".to_string(),
                    age_days: 1000,
                    dormant_days: 800,
                    star_count: 10,
                    is_core: true,
                },
            ];

            let narrative = presenter.format_temporal_narrative(
                1000,
                500,
                8,
                &[],
                &[],
                &ancient,
            );

            assert!(narrative.contains("📜"), "Should contain scroll emoji");
            assert!(narrative.contains("1 Ancient Star"));
            assert!(narrative.contains("deep strata"));
            assert!(narrative.contains("core files dormant > 2 years"));
        }

        #[test]
        fn test_temporal_narrative_geological_stability() {
            let presenter = IntelligentPresenter::new();
            let narrative = presenter.format_temporal_narrative(
                365,
                100,
                3,
                &[],  // No supernovas
                &[],  // No tectonic shifts
                &[],  // No ancient stars
            );

            assert!(narrative.contains("🌍"), "Should contain earth emoji");
            assert!(narrative.contains("Geological Stability"));
            assert!(narrative.contains("No volcanic activity"));
        }

        #[test]
        fn test_temporal_narrative_non_core_ancient() {
            let presenter = IntelligentPresenter::new();
            let ancient = vec![
                AncientStar {
                    path: "tests/old_test.rs".to_string(),
                    age_days: 900,
                    dormant_days: 750,
                    star_count: 2,
                    is_core: false,  // Not a core file
                },
            ];

            let narrative = presenter.format_temporal_narrative(
                900,
                200,
                4,
                &[],
                &[],
                &ancient,
            );

            // Should mention dormant files but not as "Ancient Stars"
            assert!(narrative.contains("dormant file"));
            assert!(narrative.contains("non-core"));
        }
    }
}
