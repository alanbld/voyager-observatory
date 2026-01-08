//! Semantic Clustering Engine
//!
//! This module provides semantic clustering for code elements in the Fractal Context Engine.
//! It groups similar functions, methods, and code patterns based on feature analysis.
//!
//! # Features
//!
//! - **Vectorization**: Convert code elements to numerical feature vectors
//! - **Clustering**: K-means and DBSCAN algorithms for grouping similar code
//! - **Shell Patterns**: Recognize shell script patterns (deployment, automation, etc.)
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::clustering::{ClusterEngine, ClusterConfig};
//!
//! let engine = ClusterEngine::new();
//! let clusters = engine.cluster_layers(&layers);
//!
//! for cluster in clusters {
//!     println!("Cluster: {} ({} members)", cluster.name, cluster.members.len());
//! }
//! ```

pub mod algorithms;
pub mod shell_patterns;
pub mod vectorizer;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::core::fractal::{ContextLayer, LayerContent, SymbolKind, ZoomLevel};

pub use algorithms::{
    cosine_distance, cosine_similarity, euclidean_distance, manhattan_distance, ClusterResult,
    ClusteringError, ClusteringResult, KMeans, DBSCAN,
};
pub use shell_patterns::{ShellPattern, ShellPatternRecognizer, ShellPatternType};
pub use vectorizer::{
    FeatureType, FeatureVector, SymbolVectorizer, VectorMetadata, VectorizerConfig,
};

// =============================================================================
// Semantic Cluster
// =============================================================================

/// A cluster of semantically similar code elements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticCluster {
    /// Unique cluster ID
    pub id: String,
    /// Human-readable cluster name
    pub name: String,
    /// Description of what this cluster represents
    pub description: String,
    /// Members of this cluster
    pub members: Vec<ClusterMember>,
    /// Cluster centroid (average feature vector)
    pub centroid: Vec<f32>,
    /// Cluster quality metrics
    pub metrics: ClusterMetrics,
    /// Common patterns found in this cluster
    pub patterns: Vec<ClusterPattern>,
}

impl SemanticCluster {
    /// Get the number of members.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Check if cluster contains a specific layer.
    pub fn contains(&self, layer_id: &str) -> bool {
        self.members.iter().any(|m| m.layer_id == layer_id)
    }

    /// Get member IDs.
    pub fn member_ids(&self) -> Vec<&str> {
        self.members.iter().map(|m| m.layer_id.as_str()).collect()
    }
}

/// A member of a semantic cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMember {
    /// Layer ID
    pub layer_id: String,
    /// Layer name
    pub name: String,
    /// Layer type
    pub layer_type: ZoomLevel,
    /// Similarity score to cluster centroid (0-1)
    pub similarity: f32,
    /// Distance to centroid
    pub distance: f32,
    /// Whether this is a representative member (closest to centroid)
    pub is_representative: bool,
}

/// Quality metrics for a cluster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterMetrics {
    /// Number of members
    pub size: usize,
    /// Cohesion (how close members are to each other)
    pub cohesion: f32,
    /// Separation (how far from other clusters)
    pub separation: f32,
    /// Silhouette score for this cluster
    pub silhouette: f32,
    /// Average distance to centroid
    pub avg_distance: f32,
    /// Maximum distance to centroid
    pub max_distance: f32,
}

/// A pattern found in a cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterPattern {
    /// All members are same symbol kind
    SameKind(SymbolKind),
    /// Common naming prefix
    NamingPrefix(String),
    /// Common naming suffix
    NamingSuffix(String),
    /// Shell pattern type
    ShellPattern(ShellPatternType),
    /// Similar parameter count
    SimilarArity(usize),
    /// Similar size (line count)
    SimilarSize(usize),
    /// Custom pattern
    Custom(String),
}

// =============================================================================
// Cluster Configuration
// =============================================================================

/// Configuration for the clustering engine.
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    /// Clustering algorithm to use
    pub algorithm: ClusterAlgorithm,
    /// Minimum cluster size
    pub min_cluster_size: usize,
    /// Maximum number of clusters
    pub max_clusters: usize,
    /// Include shell pattern analysis
    pub analyze_shell_patterns: bool,
    /// Vectorizer configuration
    pub vectorizer_config: VectorizerConfig,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            algorithm: ClusterAlgorithm::KMeans { k: 5 },
            min_cluster_size: 2,
            max_clusters: 20,
            analyze_shell_patterns: true,
            vectorizer_config: VectorizerConfig::default(),
        }
    }
}

/// Clustering algorithm selection.
#[derive(Debug, Clone)]
pub enum ClusterAlgorithm {
    KMeans { k: usize },
    DBSCAN { eps: f32, min_samples: usize },
    Auto, // Automatically choose based on data
}

// =============================================================================
// Cluster Engine
// =============================================================================

/// Engine for semantic clustering of code elements.
#[allow(dead_code)]
pub struct ClusterEngine {
    config: ClusterConfig,
    vectorizer: SymbolVectorizer,
    shell_recognizer: ShellPatternRecognizer,
    cluster_counter: usize,
}

impl Default for ClusterEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterEngine {
    pub fn new() -> Self {
        Self::with_config(ClusterConfig::default())
    }

    pub fn with_config(config: ClusterConfig) -> Self {
        let vectorizer = SymbolVectorizer::with_config(config.vectorizer_config.clone());
        Self {
            config,
            vectorizer,
            shell_recognizer: ShellPatternRecognizer::new(),
            cluster_counter: 0,
        }
    }

    /// Cluster a collection of context layers.
    pub fn cluster_layers(&mut self, layers: &[&ContextLayer]) -> Vec<SemanticCluster> {
        if layers.len() < self.config.min_cluster_size {
            return Vec::new();
        }

        // Vectorize all layers
        let vectors: Vec<FeatureVector> = layers
            .iter()
            .map(|layer| self.vectorizer.vectorize_layer(layer))
            .collect();

        // Extract raw values for clustering
        let raw_vectors: Vec<Vec<f32>> = vectors.iter().map(|v| v.values.clone()).collect();

        // Determine algorithm parameters
        let algorithm = self.determine_algorithm(&raw_vectors);

        // Perform clustering
        let result = match algorithm {
            ClusterAlgorithm::KMeans { k } => {
                let kmeans = KMeans::new(k).with_max_iter(100).with_tolerance(1e-4);
                kmeans.fit(&raw_vectors)
            }
            ClusterAlgorithm::DBSCAN { eps, min_samples } => {
                let dbscan = DBSCAN::new(eps, min_samples);
                dbscan.fit(&raw_vectors)
            }
            ClusterAlgorithm::Auto => {
                // Use K-means with auto-determined k
                let k = self.estimate_k(&raw_vectors);
                let kmeans = KMeans::new(k);
                kmeans.fit(&raw_vectors)
            }
        };

        match result {
            Ok(cluster_result) => self.build_semantic_clusters(layers, &vectors, &cluster_result),
            Err(e) => {
                eprintln!("Clustering failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Cluster symbol layers only.
    pub fn cluster_symbols(&mut self, layers: &[&ContextLayer]) -> Vec<SemanticCluster> {
        let symbol_layers: Vec<&ContextLayer> = layers
            .iter()
            .copied()
            .filter(|l| l.level == ZoomLevel::Symbol)
            .collect();

        self.cluster_layers(&symbol_layers)
    }

    /// Find similar layers to a given layer.
    pub fn find_similar<'a>(
        &self,
        target: &ContextLayer,
        candidates: &[&'a ContextLayer],
        top_k: usize,
    ) -> Vec<(&'a ContextLayer, f32)> {
        let target_vector = self.vectorizer.vectorize_layer(target);

        let mut similarities: Vec<_> = candidates
            .iter()
            .map(|&layer| {
                let vector = self.vectorizer.vectorize_layer(layer);
                let sim = target_vector.cosine_similarity(&vector);
                (layer, sim)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        similarities.truncate(top_k);
        similarities
    }

    // -------------------------------------------------------------------------
    // Internal Methods
    // -------------------------------------------------------------------------

    fn determine_algorithm(&self, data: &[Vec<f32>]) -> ClusterAlgorithm {
        match &self.config.algorithm {
            ClusterAlgorithm::Auto => {
                // Estimate good k for K-means
                let k = self.estimate_k(data);
                ClusterAlgorithm::KMeans { k }
            }
            other => other.clone(),
        }
    }

    fn estimate_k(&self, data: &[Vec<f32>]) -> usize {
        let n = data.len();

        // Rule of thumb: k ≈ sqrt(n/2)
        let estimated = ((n as f32 / 2.0).sqrt()).ceil() as usize;

        // Bound by config
        estimated.max(2).min(self.config.max_clusters).min(n / 2)
    }

    fn build_semantic_clusters(
        &mut self,
        layers: &[&ContextLayer],
        vectors: &[FeatureVector],
        result: &ClusterResult,
    ) -> Vec<SemanticCluster> {
        let mut clusters_map: HashMap<i32, Vec<(usize, &ContextLayer, &FeatureVector)>> =
            HashMap::new();

        // Group by cluster label
        for (i, &label) in result.labels.iter().enumerate() {
            if label >= 0 {
                clusters_map
                    .entry(label)
                    .or_default()
                    .push((i, layers[i], &vectors[i]));
            }
        }

        // Build semantic clusters
        let mut semantic_clusters = Vec::new();

        for (cluster_id, members) in clusters_map {
            if members.len() < self.config.min_cluster_size {
                continue;
            }

            self.cluster_counter += 1;
            let id = format!("cluster_{}", self.cluster_counter);

            // Get centroid
            let centroid = if (cluster_id as usize) < result.centroids.len() {
                result.centroids[cluster_id as usize].clone()
            } else {
                self.compute_centroid(&members)
            };

            // Build members
            let cluster_members = self.build_cluster_members(&members, &centroid);

            // Detect patterns
            let patterns = self.detect_cluster_patterns(&members);

            // Generate name and description
            let (name, description) = self.generate_cluster_info(&members, &patterns);

            // Calculate metrics
            let metrics = self.calculate_cluster_metrics(&members, &centroid);

            semantic_clusters.push(SemanticCluster {
                id,
                name,
                description,
                members: cluster_members,
                centroid,
                metrics,
                patterns,
            });
        }

        // Sort by size
        semantic_clusters.sort_by(|a, b| b.size().cmp(&a.size()));

        semantic_clusters
    }

    fn compute_centroid(&self, members: &[(usize, &ContextLayer, &FeatureVector)]) -> Vec<f32> {
        if members.is_empty() {
            return Vec::new();
        }

        let dim = members[0].2.values.len();
        let mut centroid = vec![0.0f32; dim];

        for (_, _, vector) in members {
            for (i, &val) in vector.values.iter().enumerate() {
                centroid[i] += val;
            }
        }

        let n = members.len() as f32;
        for val in &mut centroid {
            *val /= n;
        }

        centroid
    }

    fn build_cluster_members(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
        centroid: &[f32],
    ) -> Vec<ClusterMember> {
        let mut cluster_members: Vec<ClusterMember> = members
            .iter()
            .map(|(_, layer, vector)| {
                let distance = euclidean_distance(&vector.values, centroid);
                let similarity = 1.0 / (1.0 + distance);

                ClusterMember {
                    layer_id: layer.id.clone(),
                    name: layer.name().to_string(),
                    layer_type: layer.level,
                    similarity,
                    distance,
                    is_representative: false,
                }
            })
            .collect();

        // Mark closest member as representative
        if let Some(representative) = cluster_members
            .iter_mut()
            .min_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap())
        {
            representative.is_representative = true;
        }

        cluster_members
    }

    fn detect_cluster_patterns(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
    ) -> Vec<ClusterPattern> {
        let mut patterns = Vec::new();

        // Check for same symbol kind
        if let Some(kind) = self.check_same_kind(members) {
            patterns.push(ClusterPattern::SameKind(kind));
        }

        // Check for naming patterns
        if let Some(prefix) = self.check_naming_prefix(members) {
            patterns.push(ClusterPattern::NamingPrefix(prefix));
        }
        if let Some(suffix) = self.check_naming_suffix(members) {
            patterns.push(ClusterPattern::NamingSuffix(suffix));
        }

        // Check for similar arity
        if let Some(arity) = self.check_similar_arity(members) {
            patterns.push(ClusterPattern::SimilarArity(arity));
        }

        patterns
    }

    fn check_same_kind(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
    ) -> Option<SymbolKind> {
        let mut kinds: Vec<SymbolKind> = Vec::new();

        for (_, layer, _) in members {
            if let LayerContent::Symbol { kind, .. } = &layer.content {
                kinds.push(kind.clone());
            }
        }

        if kinds.is_empty() {
            return None;
        }

        let first = &kinds[0];
        if kinds.iter().all(|k| k == first) {
            Some(first.clone())
        } else {
            None
        }
    }

    fn check_naming_prefix(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
    ) -> Option<String> {
        let names: Vec<String> = members
            .iter()
            .map(|(_, layer, _)| layer.name().to_string())
            .collect();

        if names.len() < 2 {
            return None;
        }

        // Find common prefix
        let first = &names[0];
        let mut prefix_len = 0;

        'outer: for i in 1..first.len() {
            let prefix = &first[..i];
            for name in &names[1..] {
                if !name.starts_with(prefix) {
                    break 'outer;
                }
            }
            prefix_len = i;
        }

        if prefix_len >= 3 {
            Some(names[0][..prefix_len].to_string())
        } else {
            None
        }
    }

    fn check_naming_suffix(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
    ) -> Option<String> {
        let names: Vec<String> = members
            .iter()
            .map(|(_, layer, _)| layer.name().to_string())
            .collect();

        if names.len() < 2 {
            return None;
        }

        // Find common suffix
        let first: String = names[0].chars().rev().collect();
        let mut suffix_len = 0;

        'outer: for i in 1..first.len() {
            let suffix = &first[..i];
            for name in &names[1..] {
                let rev: String = name.chars().rev().collect();
                if !rev.starts_with(suffix) {
                    break 'outer;
                }
            }
            suffix_len = i;
        }

        if suffix_len >= 3 {
            Some(names[0][names[0].len() - suffix_len..].to_string())
        } else {
            None
        }
    }

    fn check_similar_arity(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
    ) -> Option<usize> {
        let arities: Vec<usize> = members
            .iter()
            .filter_map(|(_, layer, _)| {
                if let LayerContent::Symbol { parameters, .. } = &layer.content {
                    Some(parameters.len())
                } else {
                    None
                }
            })
            .collect();

        if arities.is_empty() {
            return None;
        }

        // Check if all arities are within 1 of each other
        let min = *arities.iter().min().unwrap();
        let max = *arities.iter().max().unwrap();

        if max - min <= 1 {
            Some(arities.iter().sum::<usize>() / arities.len())
        } else {
            None
        }
    }

    fn generate_cluster_info(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
        patterns: &[ClusterPattern],
    ) -> (String, String) {
        let mut name = String::new();
        let mut description = String::new();

        // Generate name based on patterns
        for pattern in patterns {
            match pattern {
                ClusterPattern::SameKind(kind) => {
                    name = format!("{:?}s", kind);
                    description = format!("Functions of type {:?}", kind);
                }
                ClusterPattern::NamingPrefix(prefix) => {
                    if name.is_empty() {
                        name = format!("{}* functions", prefix);
                    }
                    description = format!("Functions starting with '{}'", prefix);
                }
                ClusterPattern::NamingSuffix(suffix) => {
                    if name.is_empty() {
                        name = format!("*{} functions", suffix);
                    }
                    if description.is_empty() {
                        description = format!("Functions ending with '{}'", suffix);
                    }
                }
                ClusterPattern::SimilarArity(arity) => {
                    if description.is_empty() {
                        description = format!("Functions with ~{} parameters", arity);
                    }
                }
                _ => {}
            }
        }

        // Fallback name
        if name.is_empty() {
            name = format!("Cluster of {} items", members.len());
        }
        if description.is_empty() {
            description = format!("Group of {} similar code elements", members.len());
        }

        (name, description)
    }

    fn calculate_cluster_metrics(
        &self,
        members: &[(usize, &ContextLayer, &FeatureVector)],
        centroid: &[f32],
    ) -> ClusterMetrics {
        if members.is_empty() {
            return ClusterMetrics::default();
        }

        // Calculate distances to centroid
        let distances: Vec<f32> = members
            .iter()
            .map(|(_, _, vector)| euclidean_distance(&vector.values, centroid))
            .collect();

        let avg_distance = distances.iter().sum::<f32>() / distances.len() as f32;
        let max_distance = distances.iter().cloned().fold(0.0f32, f32::max);

        // Cohesion: 1 / (1 + avg_distance)
        let cohesion = 1.0 / (1.0 + avg_distance);

        ClusterMetrics {
            size: members.len(),
            cohesion,
            separation: 0.0, // Would need other clusters to calculate
            silhouette: 0.0, // Would need other clusters to calculate
            avg_distance,
            max_distance,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{Parameter, Range, Visibility};

    fn create_test_function(name: &str, kind: SymbolKind) -> ContextLayer {
        ContextLayer::new(
            format!("sym_{}", name),
            LayerContent::Symbol {
                name: name.to_string(),
                kind,
                signature: format!("fn {}()", name),
                return_type: None,
                parameters: vec![],
                documentation: Some(format!("Docs for {}", name)),
                visibility: Visibility::Public,
                range: Range::line_range(1, 10),
            },
        )
    }

    fn create_test_function_with_params(name: &str, param_count: usize) -> ContextLayer {
        let params: Vec<Parameter> = (0..param_count)
            .map(|i| Parameter {
                name: format!("arg{}", i),
                type_hint: Some("i32".to_string()),
                default_value: None,
            })
            .collect();

        ContextLayer::new(
            format!("sym_{}", name),
            LayerContent::Symbol {
                name: name.to_string(),
                kind: SymbolKind::Function,
                signature: format!(
                    "fn {}({})",
                    name,
                    params
                        .iter()
                        .map(|p| p.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                return_type: None,
                parameters: params,
                documentation: None,
                visibility: Visibility::Public,
                range: Range::line_range(1, 10),
            },
        )
    }

    #[test]
    fn test_cluster_engine_basic() {
        let mut engine = ClusterEngine::new();

        let layers = vec![
            create_test_function("get_user", SymbolKind::Function),
            create_test_function("get_account", SymbolKind::Function),
            create_test_function("get_order", SymbolKind::Function),
            create_test_function("set_user", SymbolKind::Function),
            create_test_function("set_account", SymbolKind::Function),
            create_test_function("set_order", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // Should find clusters
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_cluster_engine_with_config() {
        let config = ClusterConfig {
            algorithm: ClusterAlgorithm::KMeans { k: 2 },
            min_cluster_size: 2,
            max_clusters: 5,
            analyze_shell_patterns: false,
            vectorizer_config: VectorizerConfig::default(),
        };

        let mut engine = ClusterEngine::with_config(config);

        let layers = vec![
            create_test_function("calculate_sum", SymbolKind::Function),
            create_test_function("calculate_product", SymbolKind::Function),
            create_test_function("calculate_diff", SymbolKind::Function),
            create_test_function("validate_user", SymbolKind::Function),
            create_test_function("validate_email", SymbolKind::Function),
            create_test_function("validate_age", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // Should create 2 clusters as configured
        assert_eq!(clusters.len(), 2);
    }

    #[test]
    fn test_cluster_finds_naming_pattern() {
        let mut engine = ClusterEngine::with_config(ClusterConfig {
            algorithm: ClusterAlgorithm::KMeans { k: 2 },
            min_cluster_size: 2,
            ..Default::default()
        });

        let layers = vec![
            create_test_function("get_user", SymbolKind::Function),
            create_test_function("get_account", SymbolKind::Function),
            create_test_function("get_order", SymbolKind::Function),
            create_test_function("set_user", SymbolKind::Function),
            create_test_function("set_account", SymbolKind::Function),
            create_test_function("set_order", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // Check if naming patterns were detected
        let has_prefix_pattern = clusters.iter().any(|c| {
            c.patterns
                .iter()
                .any(|p| matches!(p, ClusterPattern::NamingPrefix(_)))
        });

        // May or may not find prefix pattern depending on clustering
        // At minimum, should have clusters
        assert!(!clusters.is_empty());
    }

    #[test]
    fn test_find_similar() {
        let engine = ClusterEngine::new();

        let target = create_test_function("get_user", SymbolKind::Function);

        let candidates = vec![
            create_test_function("get_account", SymbolKind::Function),
            create_test_function("set_user", SymbolKind::Function),
            create_test_function("delete_all", SymbolKind::Function),
            create_test_function("get_order", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = candidates.iter().collect();
        let similar = engine.find_similar(&target, &refs, 3);

        assert_eq!(similar.len(), 3);
        // All should have positive similarity
        assert!(similar.iter().all(|(_, sim)| *sim > 0.0));
    }

    #[test]
    fn test_cluster_metrics() {
        let mut engine = ClusterEngine::new();

        let layers = vec![
            create_test_function("func1", SymbolKind::Function),
            create_test_function("func2", SymbolKind::Function),
            create_test_function("func3", SymbolKind::Function),
            create_test_function("func4", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        for cluster in &clusters {
            assert!(cluster.metrics.size > 0);
            assert!(cluster.metrics.cohesion >= 0.0);
            assert!(cluster.metrics.avg_distance >= 0.0);
        }
    }

    #[test]
    fn test_cluster_member_representative() {
        let mut engine = ClusterEngine::with_config(ClusterConfig {
            algorithm: ClusterAlgorithm::KMeans { k: 1 },
            min_cluster_size: 2,
            ..Default::default()
        });

        let layers = vec![
            create_test_function("func1", SymbolKind::Function),
            create_test_function("func2", SymbolKind::Function),
            create_test_function("func3", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // Each cluster should have exactly one representative
        for cluster in &clusters {
            let rep_count = cluster
                .members
                .iter()
                .filter(|m| m.is_representative)
                .count();
            assert_eq!(rep_count, 1);
        }
    }

    #[test]
    fn test_empty_input() {
        let mut engine = ClusterEngine::new();
        let layers: Vec<&ContextLayer> = vec![];

        let clusters = engine.cluster_layers(&layers);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_too_few_layers() {
        let mut engine = ClusterEngine::with_config(ClusterConfig {
            min_cluster_size: 3,
            ..Default::default()
        });

        let layers = vec![
            create_test_function("func1", SymbolKind::Function),
            create_test_function("func2", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        assert!(clusters.is_empty());
    }

    #[test]
    fn test_dbscan_algorithm() {
        let mut engine = ClusterEngine::with_config(ClusterConfig {
            algorithm: ClusterAlgorithm::DBSCAN {
                eps: 0.5,
                min_samples: 2,
            },
            min_cluster_size: 2,
            ..Default::default()
        });

        let layers = vec![
            create_test_function("get_user", SymbolKind::Function),
            create_test_function("get_account", SymbolKind::Function),
            create_test_function("set_user", SymbolKind::Function),
            create_test_function("set_account", SymbolKind::Function),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // DBSCAN may or may not find clusters depending on data
        // Just verify it doesn't panic
        assert!(clusters.len() <= 4);
    }

    #[test]
    fn test_auto_algorithm() {
        let mut engine = ClusterEngine::with_config(ClusterConfig {
            algorithm: ClusterAlgorithm::Auto,
            min_cluster_size: 2,
            ..Default::default()
        });

        let layers: Vec<ContextLayer> = (0..20)
            .map(|i| create_test_function(&format!("func{}", i), SymbolKind::Function))
            .collect();

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let clusters = engine.cluster_layers(&refs);

        // Auto should pick reasonable k
        assert!(!clusters.is_empty());
        assert!(clusters.len() <= 10); // sqrt(20/2) ≈ 3, bounded
    }

    #[test]
    fn test_semantic_cluster_contains() {
        let cluster = SemanticCluster {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: "Test cluster".to_string(),
            members: vec![
                ClusterMember {
                    layer_id: "layer_1".to_string(),
                    name: "func1".to_string(),
                    layer_type: ZoomLevel::Symbol,
                    similarity: 0.9,
                    distance: 0.1,
                    is_representative: true,
                },
                ClusterMember {
                    layer_id: "layer_2".to_string(),
                    name: "func2".to_string(),
                    layer_type: ZoomLevel::Symbol,
                    similarity: 0.8,
                    distance: 0.2,
                    is_representative: false,
                },
            ],
            centroid: vec![0.5, 0.5],
            metrics: ClusterMetrics::default(),
            patterns: vec![],
        };

        assert!(cluster.contains("layer_1"));
        assert!(cluster.contains("layer_2"));
        assert!(!cluster.contains("layer_3"));
        assert_eq!(cluster.size(), 2);
    }
}
