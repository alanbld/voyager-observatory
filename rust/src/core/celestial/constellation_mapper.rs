//! Constellation Mapper Module
//!
//! Groups files into Nebulae based on semantic similarity.
//! Creates a "Celestial Map" for navigation.

use std::collections::HashMap;
use std::path::Path;

use super::nebula_namer::{NebulaName, NebulaNamer};
use crate::core::fractal::semantic::UniversalConceptType;

// =============================================================================
// Star (Individual File)
// =============================================================================

/// A star represents an individual file in the celestial map.
#[derive(Debug, Clone)]
pub struct Star {
    /// File path
    pub path: String,
    /// Detected language
    pub language: String,
    /// Brightness (utility score, 0.0 - 1.0)
    pub brightness: f64,
    /// Primary concept type
    pub concept_type: Option<UniversalConceptType>,
    /// Token count
    pub tokens: usize,
    /// Whether this is the brightest star in its nebula
    pub is_brightest: bool,
}

impl Star {
    /// Create a new star.
    pub fn new(path: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            language: language.into(),
            brightness: 0.5,
            concept_type: None,
            tokens: 0,
            is_brightest: false,
        }
    }

    /// Set brightness (utility).
    pub fn with_brightness(mut self, brightness: f64) -> Self {
        self.brightness = brightness;
        self
    }

    /// Set concept type.
    pub fn with_concept_type(mut self, concept_type: UniversalConceptType) -> Self {
        self.concept_type = Some(concept_type);
        self
    }

    /// Set token count.
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens = tokens;
        self
    }

    /// Get brightness indicator.
    pub fn brightness_indicator(&self) -> &'static str {
        if self.brightness >= 0.9 {
            "🌟"
        } else if self.brightness >= 0.8 {
            "⭐"
        } else if self.brightness >= 0.5 {
            "✨"
        } else {
            "·"
        }
    }

    /// Get file extension.
    pub fn extension(&self) -> Option<&str> {
        Path::new(&self.path).extension().and_then(|e| e.to_str())
    }

    /// Get file name without path.
    pub fn file_name(&self) -> &str {
        Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&self.path)
    }
}

// =============================================================================
// Nebula (Semantic Cluster)
// =============================================================================

/// A nebula is a cluster of semantically similar files.
#[derive(Debug, Clone)]
pub struct Nebula {
    /// Unique identifier
    pub id: String,
    /// Human-readable name
    pub name: NebulaName,
    /// Stars (files) in this nebula
    pub stars: Vec<Star>,
    /// Average cohesion score (0.0 - 1.0)
    pub cohesion: f32,
    /// Dominant concept type
    pub dominant_concept: Option<UniversalConceptType>,
    /// Languages present in this nebula
    pub languages: Vec<String>,
    /// Whether this nebula is "faded" (low overall utility)
    pub is_faded: bool,
}

impl Nebula {
    /// Create a new nebula.
    pub fn new(id: impl Into<String>, name: NebulaName) -> Self {
        Self {
            id: id.into(),
            name,
            stars: Vec::new(),
            cohesion: 0.0,
            dominant_concept: None,
            languages: Vec::new(),
            is_faded: false,
        }
    }

    /// Add a star to the nebula.
    pub fn add_star(&mut self, star: Star) {
        // Track language
        if !self.languages.contains(&star.language) {
            self.languages.push(star.language.clone());
        }
        self.stars.push(star);
    }

    /// Get the number of stars.
    pub fn star_count(&self) -> usize {
        self.stars.len()
    }

    /// Get the brightest star in the nebula.
    pub fn brightest_star(&self) -> Option<&Star> {
        self.stars.iter().max_by(|a, b| {
            a.brightness
                .partial_cmp(&b.brightness)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    /// Mark the brightest star.
    pub fn mark_brightest(&mut self) {
        if let Some(max_brightness) = self.stars.iter().map(|s| s.brightness).reduce(f64::max) {
            for star in &mut self.stars {
                star.is_brightest = (star.brightness - max_brightness).abs() < 0.001;
            }
        }
    }

    /// Calculate average brightness.
    pub fn average_brightness(&self) -> f64 {
        if self.stars.is_empty() {
            return 0.0;
        }
        self.stars.iter().map(|s| s.brightness).sum::<f64>() / self.stars.len() as f64
    }

    /// Check if nebula is "faded" (low utility).
    pub fn update_faded_status(&mut self, threshold: f64) {
        self.is_faded = self.average_brightness() < threshold;
    }

    /// Format the nebula for display.
    pub fn format_display(&self) -> String {
        let mut output = String::new();

        // Header with nebula name and star count
        let indicator = if self.is_faded { "🌫️" } else { "✨" };
        output.push_str(&format!(
            "{} {} ({} stars)\n",
            indicator,
            self.name.display(),
            self.star_count()
        ));

        // Sort stars by brightness (descending)
        let mut sorted_stars: Vec<_> = self.stars.iter().collect();
        sorted_stars.sort_by(|a, b| {
            b.brightness
                .partial_cmp(&a.brightness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Display stars (limit to top 5 for readability)
        for star in sorted_stars.iter().take(5) {
            let brightest_marker = if star.is_brightest {
                " (Brightest Star)"
            } else {
                ""
            };
            output.push_str(&format!(
                "    {} {}{}\n",
                star.brightness_indicator(),
                star.file_name(),
                brightest_marker
            ));
        }

        if self.stars.len() > 5 {
            output.push_str(&format!("    ... and {} more\n", self.stars.len() - 5));
        }

        output
    }
}

// =============================================================================
// Celestial Map
// =============================================================================

/// The complete celestial map of a project.
#[derive(Debug, Clone)]
pub struct CelestialMap {
    /// All nebulae in the map
    pub nebulae: Vec<Nebula>,
    /// Ungrouped stars (didn't fit any nebula)
    pub ungrouped_stars: Vec<Star>,
    /// Total file count
    pub total_stars: usize,
    /// Analysis time in milliseconds
    pub analysis_time_ms: u64,
}

impl CelestialMap {
    /// Create a new celestial map.
    pub fn new() -> Self {
        Self {
            nebulae: Vec::new(),
            ungrouped_stars: Vec::new(),
            total_stars: 0,
            analysis_time_ms: 0,
        }
    }

    /// Add a nebula to the map.
    pub fn add_nebula(&mut self, nebula: Nebula) {
        self.total_stars += nebula.star_count();
        self.nebulae.push(nebula);
    }

    /// Add an ungrouped star.
    pub fn add_ungrouped(&mut self, star: Star) {
        self.total_stars += 1;
        self.ungrouped_stars.push(star);
    }

    /// Get all nebulae sorted by star count (descending).
    pub fn sorted_nebulae(&self) -> Vec<&Nebula> {
        let mut nebulae: Vec<_> = self.nebulae.iter().collect();
        nebulae.sort_by(|a, b| b.star_count().cmp(&a.star_count()));
        nebulae
    }

    /// Format the complete celestial map for display.
    pub fn format_display(&self) -> String {
        let mut output = String::new();

        output.push_str("🌌 CELESTIAL MAP\n");
        output.push_str("═══════════════════════════════════════════════════════════\n\n");

        // Stats
        output.push_str(&format!(
            "🔭 {} stars across {} nebulae\n\n",
            self.total_stars,
            self.nebulae.len()
        ));

        // Display each nebula
        for nebula in self.sorted_nebulae() {
            output.push_str(&nebula.format_display());
            output.push('\n');
        }

        // Ungrouped stars
        if !self.ungrouped_stars.is_empty() {
            output.push_str(&format!(
                "🌑 Ungrouped Stars ({} files)\n",
                self.ungrouped_stars.len()
            ));
            for star in self.ungrouped_stars.iter().take(3) {
                output.push_str(&format!("    · {}\n", star.file_name()));
            }
            if self.ungrouped_stars.len() > 3 {
                output.push_str(&format!(
                    "    ... and {} more\n",
                    self.ungrouped_stars.len() - 3
                ));
            }
        }

        output
    }
}

impl Default for CelestialMap {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Constellation Mapper
// =============================================================================

/// The constellation mapper groups files into nebulae.
pub struct ConstellationMapper {
    /// Nebula namer
    namer: NebulaNamer,
    /// Similarity threshold for grouping (0.0 - 1.0)
    similarity_threshold: f32,
    /// Minimum nebula size
    min_nebula_size: usize,
    /// Faded threshold (average brightness below this = faded)
    faded_threshold: f64,
}

impl Default for ConstellationMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstellationMapper {
    /// Create a new constellation mapper.
    pub fn new() -> Self {
        Self {
            namer: NebulaNamer::new(),
            similarity_threshold: 0.7,
            min_nebula_size: 2,
            faded_threshold: 0.4,
        }
    }

    /// Set similarity threshold.
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    /// Set minimum nebula size.
    pub fn with_min_nebula_size(mut self, size: usize) -> Self {
        self.min_nebula_size = size;
        self
    }

    /// Map files into a celestial map.
    ///
    /// This is the main entry point for constellation mapping.
    /// Groups files by directory structure and semantic similarity.
    pub fn map(&self, files: &[FileInfo]) -> CelestialMap {
        let start = std::time::Instant::now();
        let mut map = CelestialMap::new();

        if files.is_empty() {
            return map;
        }

        // Group files by top-level directory first
        let mut dir_groups: HashMap<String, Vec<&FileInfo>> = HashMap::new();
        for file in files {
            let top_dir = self.get_top_directory(&file.path);
            dir_groups.entry(top_dir).or_default().push(file);
        }

        // Create nebulae from directory groups
        let mut nebula_id = 0;
        for (_dir, group_files) in dir_groups {
            if group_files.len() < self.min_nebula_size {
                // Add to ungrouped
                for file in group_files {
                    map.add_ungrouped(self.file_to_star(file));
                }
                continue;
            }

            // Calculate concept distribution for naming
            let concept_counts = self.calculate_concept_counts(&group_files);
            let file_paths: Vec<String> = group_files.iter().map(|f| f.path.clone()).collect();

            // Name the nebula
            let name = self.namer.name_nebula(&file_paths, &concept_counts);

            // Create nebula
            let mut nebula = Nebula::new(format!("nebula-{}", nebula_id), name);
            nebula_id += 1;

            // Add stars
            for file in group_files {
                nebula.add_star(self.file_to_star(file));
            }

            // Set dominant concept
            nebula.dominant_concept = concept_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(t, _)| *t);

            // Update metrics
            nebula.mark_brightest();
            nebula.update_faded_status(self.faded_threshold);

            map.add_nebula(nebula);
        }

        map.analysis_time_ms = start.elapsed().as_millis() as u64;
        map
    }

    /// Map files with semantic similarity consideration.
    ///
    /// Uses pairwise similarity to refine groupings.
    pub fn map_with_similarity(
        &self,
        files: &[FileInfo],
        similarities: &[(usize, usize, f32)],
    ) -> CelestialMap {
        let start = std::time::Instant::now();
        let mut map = CelestialMap::new();

        if files.is_empty() {
            return map;
        }

        // Build similarity graph
        let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, j, sim) in similarities {
            if *sim >= self.similarity_threshold {
                adjacency.entry(*i).or_default().push(*j);
                adjacency.entry(*j).or_default().push(*i);
            }
        }

        // Find connected components (clusters)
        let mut visited = vec![false; files.len()];
        let mut clusters: Vec<Vec<usize>> = Vec::new();

        for start_idx in 0..files.len() {
            if visited[start_idx] {
                continue;
            }

            // BFS to find connected component
            let mut cluster = Vec::new();
            let mut queue = vec![start_idx];
            visited[start_idx] = true;

            while let Some(idx) = queue.pop() {
                cluster.push(idx);
                if let Some(neighbors) = adjacency.get(&idx) {
                    for &neighbor in neighbors {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            queue.push(neighbor);
                        }
                    }
                }
            }

            clusters.push(cluster);
        }

        // Convert clusters to nebulae
        let mut nebula_id = 0;
        for cluster in clusters {
            let group_files: Vec<&FileInfo> = cluster.iter().map(|&i| &files[i]).collect();

            if group_files.len() < self.min_nebula_size {
                for file in group_files {
                    map.add_ungrouped(self.file_to_star(file));
                }
                continue;
            }

            let concept_counts = self.calculate_concept_counts(&group_files);
            let file_paths: Vec<String> = group_files.iter().map(|f| f.path.clone()).collect();
            let name = self.namer.name_nebula(&file_paths, &concept_counts);

            let mut nebula = Nebula::new(format!("nebula-{}", nebula_id), name);
            nebula_id += 1;

            for file in group_files {
                nebula.add_star(self.file_to_star(file));
            }

            nebula.dominant_concept = concept_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(t, _)| *t);

            nebula.mark_brightest();
            nebula.update_faded_status(self.faded_threshold);

            map.add_nebula(nebula);
        }

        map.analysis_time_ms = start.elapsed().as_millis() as u64;
        map
    }

    /// Get top-level directory from path.
    fn get_top_directory(&self, path: &str) -> String {
        let path = Path::new(path);
        let components: Vec<_> = path.components().collect();

        // Find first meaningful directory
        for component in &components {
            if let std::path::Component::Normal(os_str) = component {
                if let Some(name) = os_str.to_str() {
                    // Skip if it looks like a file (has extension)
                    if !name.contains('.') {
                        return name.to_string();
                    }
                }
            }
        }

        // Fallback to "root" for files in root directory
        "root".to_string()
    }

    /// Convert FileInfo to Star.
    fn file_to_star(&self, file: &FileInfo) -> Star {
        let mut star = Star::new(&file.path, &file.language)
            .with_brightness(file.utility)
            .with_tokens(file.tokens);

        if let Some(concept) = file.concept_type {
            star = star.with_concept_type(concept);
        }

        star
    }

    /// Calculate concept type counts for a group of files.
    fn calculate_concept_counts(
        &self,
        files: &[&FileInfo],
    ) -> HashMap<UniversalConceptType, usize> {
        let mut counts = HashMap::new();
        for file in files {
            if let Some(concept) = file.concept_type {
                *counts.entry(concept).or_insert(0) += 1;
            }
        }
        counts
    }
}

// =============================================================================
// File Info (Input Type)
// =============================================================================

/// Input information about a file for mapping.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File path
    pub path: String,
    /// Detected language
    pub language: String,
    /// Utility score (brightness)
    pub utility: f64,
    /// Token count
    pub tokens: usize,
    /// Primary concept type
    pub concept_type: Option<UniversalConceptType>,
}

impl FileInfo {
    /// Create new file info.
    pub fn new(path: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            language: language.into(),
            utility: 0.5,
            tokens: 0,
            concept_type: None,
        }
    }

    /// Set utility.
    pub fn with_utility(mut self, utility: f64) -> Self {
        self.utility = utility;
        self
    }

    /// Set tokens.
    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens = tokens;
        self
    }

    /// Set concept type.
    pub fn with_concept_type(mut self, concept_type: UniversalConceptType) -> Self {
        self.concept_type = Some(concept_type);
        self
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star_brightness_indicator() {
        let star = Star::new("test.rs", "rust").with_brightness(0.95);
        assert_eq!(star.brightness_indicator(), "🌟");

        let star = Star::new("test.rs", "rust").with_brightness(0.85);
        assert_eq!(star.brightness_indicator(), "⭐");

        let star = Star::new("test.rs", "rust").with_brightness(0.6);
        assert_eq!(star.brightness_indicator(), "✨");

        let star = Star::new("test.rs", "rust").with_brightness(0.3);
        assert_eq!(star.brightness_indicator(), "·");
    }

    #[test]
    fn test_nebula_brightest_star() {
        let mut nebula = Nebula::new(
            "test",
            NebulaName::new("Test", super::super::nebula_namer::NamingStrategy::Fallback),
        );

        nebula.add_star(Star::new("a.rs", "rust").with_brightness(0.5));
        nebula.add_star(Star::new("b.rs", "rust").with_brightness(0.9));
        nebula.add_star(Star::new("c.rs", "rust").with_brightness(0.7));

        let brightest = nebula.brightest_star().unwrap();
        assert_eq!(brightest.path, "b.rs");
    }

    #[test]
    fn test_constellation_mapper_basic() {
        let mapper = ConstellationMapper::new();

        let files = vec![
            FileInfo::new("src/lib.rs", "rust").with_utility(0.9),
            FileInfo::new("src/main.rs", "rust").with_utility(0.8),
            FileInfo::new("tests/test_lib.rs", "rust").with_utility(0.7),
            FileInfo::new("tests/test_main.rs", "rust").with_utility(0.6),
        ];

        let map = mapper.map(&files);

        // Should have at least 2 nebulae (src and tests)
        assert!(map.nebulae.len() >= 2);
        assert_eq!(map.total_stars, 4);
    }

    #[test]
    fn test_constellation_mapper_mixed_languages() {
        let mapper = ConstellationMapper::new();

        let files = vec![
            FileInfo::new("src/lib.rs", "rust").with_utility(0.9),
            FileInfo::new("src/main.py", "python").with_utility(0.8),
            FileInfo::new("src/utils.js", "javascript").with_utility(0.7),
        ];

        let map = mapper.map(&files);

        // All in src, should be one nebula with multiple languages
        assert_eq!(map.nebulae.len(), 1);
        assert!(map.nebulae[0].languages.len() >= 2);
    }

    #[test]
    fn test_celestial_map_display() {
        let mut map = CelestialMap::new();

        let mut nebula = Nebula::new(
            "test",
            NebulaName::new(
                "Service Layer",
                super::super::nebula_namer::NamingStrategy::ConceptBased,
            ),
        );
        nebula.add_star(Star::new("service.rs", "rust").with_brightness(0.9));
        nebula.add_star(Star::new("handler.rs", "rust").with_brightness(0.7));
        nebula.mark_brightest();

        map.add_nebula(nebula);

        let display = map.format_display();
        assert!(display.contains("CELESTIAL MAP"));
        assert!(display.contains("Service Layer"));
        assert!(display.contains("service.rs"));
    }

    // =========================================================================
    // Phase 1: Coverage Blitz - Ungrouped Stars Display Tests
    // =========================================================================

    #[test]
    fn test_celestial_map_with_ungrouped_stars_few() {
        let mut map = CelestialMap::new();

        // Add ungrouped stars (fewer than 3)
        map.ungrouped_stars
            .push(Star::new("orphan1.rs", "rust").with_brightness(0.5));
        map.ungrouped_stars
            .push(Star::new("orphan2.py", "python").with_brightness(0.4));

        let display = map.format_display();
        assert!(
            display.contains("Ungrouped Stars (2 files)"),
            "Should show ungrouped count"
        );
        assert!(display.contains("orphan1.rs"), "Should show first orphan");
        assert!(display.contains("orphan2.py"), "Should show second orphan");
        assert!(
            !display.contains("... and"),
            "Should not show 'and more' for <3 files"
        );
    }

    #[test]
    fn test_celestial_map_with_ungrouped_stars_many() {
        let mut map = CelestialMap::new();

        // Add more than 3 ungrouped stars
        map.ungrouped_stars
            .push(Star::new("orphan1.rs", "rust").with_brightness(0.5));
        map.ungrouped_stars
            .push(Star::new("orphan2.py", "python").with_brightness(0.4));
        map.ungrouped_stars
            .push(Star::new("orphan3.js", "javascript").with_brightness(0.3));
        map.ungrouped_stars
            .push(Star::new("orphan4.ts", "typescript").with_brightness(0.2));
        map.ungrouped_stars
            .push(Star::new("orphan5.go", "go").with_brightness(0.1));

        let display = map.format_display();
        assert!(
            display.contains("Ungrouped Stars (5 files)"),
            "Should show total count"
        );
        assert!(display.contains("orphan1.rs"), "Should show first 3");
        assert!(display.contains("orphan2.py"), "Should show first 3");
        assert!(display.contains("orphan3.js"), "Should show first 3");
        assert!(
            !display.contains("orphan4.ts"),
            "Should not show beyond first 3"
        );
        assert!(
            display.contains("... and 2 more"),
            "Should show remaining count"
        );
    }

    #[test]
    fn test_celestial_map_ungrouped_only_no_nebulae() {
        let mut map = CelestialMap::new();

        // Only ungrouped stars, no nebulae
        map.ungrouped_stars
            .push(Star::new("lonely.rs", "rust").with_brightness(0.8));

        let display = map.format_display();
        assert!(
            display.contains("CELESTIAL MAP"),
            "Should still have header"
        );
        assert!(
            display.contains("Ungrouped Stars"),
            "Should show ungrouped section"
        );
        assert!(display.contains("lonely.rs"), "Should show the orphan");
    }

    #[test]
    fn test_celestial_map_empty() {
        let map = CelestialMap::new();

        let display = map.format_display();
        assert!(
            display.contains("CELESTIAL MAP"),
            "Should have header even when empty"
        );
        assert!(
            !display.contains("Ungrouped Stars"),
            "Should not show ungrouped section when empty"
        );
    }

    #[test]
    fn test_celestial_map_default() {
        let map = CelestialMap::default();
        assert!(map.nebulae.is_empty());
        assert!(map.ungrouped_stars.is_empty());
        assert_eq!(map.total_stars, 0);
    }

    #[test]
    fn test_constellation_mapper_default() {
        let mapper = ConstellationMapper::default();
        let files = vec![FileInfo::new("test.rs", "rust").with_utility(0.5)];
        let map = mapper.map(&files);
        assert_eq!(map.total_stars, 1);
    }

    #[test]
    fn test_celestial_map_mixed_nebulae_and_ungrouped() {
        let mut map = CelestialMap::new();

        // Add a nebula
        let mut nebula = Nebula::new(
            "src",
            NebulaName::new(
                "Core Logic",
                super::super::nebula_namer::NamingStrategy::ConceptBased,
            ),
        );
        nebula.add_star(Star::new("src/lib.rs", "rust").with_brightness(0.9));
        nebula.mark_brightest();
        map.add_nebula(nebula);

        // Add ungrouped stars
        map.ungrouped_stars
            .push(Star::new("README.md", "markdown").with_brightness(0.3));
        map.ungrouped_stars
            .push(Star::new("Cargo.toml", "toml").with_brightness(0.4));

        let display = map.format_display();
        assert!(display.contains("Core Logic"), "Should show nebula");
        assert!(
            display.contains("Ungrouped Stars (2 files)"),
            "Should show ungrouped"
        );
        assert!(display.contains("README.md"), "Should list ungrouped files");
    }
}
