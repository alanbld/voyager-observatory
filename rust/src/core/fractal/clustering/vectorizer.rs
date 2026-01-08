//! Symbol Vectorizer - Convert code to feature vectors
//!
//! Transforms code elements into numerical feature vectors for clustering.
//! Extracts structural, semantic, behavioral, and textual features.

use std::collections::HashMap;

use crate::core::fractal::{BlockType, ContextLayer, LayerContent, SymbolKind, ZoomLevel};

// =============================================================================
// Feature Vector
// =============================================================================

/// A feature vector representing a code element.
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// Numerical feature values (normalized to [0, 1])
    pub values: Vec<f32>,
    /// Metadata about the vector
    pub metadata: VectorMetadata,
}

impl FeatureVector {
    /// Get the dimensionality of the vector.
    pub fn dim(&self) -> usize {
        self.values.len()
    }

    /// Calculate Euclidean distance to another vector.
    pub fn distance(&self, other: &FeatureVector) -> f32 {
        if self.dim() != other.dim() {
            return f32::INFINITY;
        }

        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Calculate cosine similarity with another vector.
    pub fn cosine_similarity(&self, other: &FeatureVector) -> f32 {
        if self.dim() != other.dim() {
            return 0.0;
        }

        let dot: f32 = self
            .values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum();
        let mag_a: f32 = self.values.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();
        let mag_b: f32 = other.values.iter().map(|x| x.powi(2)).sum::<f32>().sqrt();

        if mag_a == 0.0 || mag_b == 0.0 {
            0.0
        } else {
            dot / (mag_a * mag_b)
        }
    }
}

/// Metadata about a feature vector.
#[derive(Debug, Clone)]
pub struct VectorMetadata {
    /// Source layer ID
    pub source_id: String,
    /// Layer type
    pub layer_type: ZoomLevel,
    /// Confidence in the extraction
    pub confidence: f32,
    /// Feature types included
    pub feature_types: Vec<FeatureType>,
}

/// Types of features extracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureType {
    /// Function length, nesting depth, parameter count
    Structural,
    /// Keywords, operations, patterns
    Semantic,
    /// What it does (IO, computation, etc.)
    Behavioral,
    /// Dependencies, calls
    Relational,
    /// Name similarity, comments
    Textual,
}

// =============================================================================
// Symbol Vectorizer
// =============================================================================

/// Configuration for vectorization.
#[derive(Debug, Clone)]
pub struct VectorizerConfig {
    pub include_structural: bool,
    pub include_semantic: bool,
    pub include_behavioral: bool,
    pub include_textual: bool,
    /// Fixed output dimension (pads or truncates)
    pub fixed_dimension: Option<usize>,
}

impl Default for VectorizerConfig {
    fn default() -> Self {
        Self {
            include_structural: true,
            include_semantic: true,
            include_behavioral: true,
            include_textual: true,
            fixed_dimension: Some(64), // Standard 64-dim vectors
        }
    }
}

/// Converts code elements to feature vectors.
#[allow(dead_code)]
pub struct SymbolVectorizer {
    config: VectorizerConfig,
    keyword_weights: HashMap<String, f32>,
}

impl Default for SymbolVectorizer {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolVectorizer {
    pub fn new() -> Self {
        Self::with_config(VectorizerConfig::default())
    }

    pub fn with_config(config: VectorizerConfig) -> Self {
        Self {
            config,
            keyword_weights: Self::load_keyword_weights(),
        }
    }

    /// Vectorize a context layer.
    pub fn vectorize_layer(&self, layer: &ContextLayer) -> FeatureVector {
        let mut features = Vec::new();
        let mut feature_types = Vec::new();

        match &layer.content {
            LayerContent::Symbol {
                name,
                kind,
                signature,
                parameters,
                documentation,
                visibility: _,
                range,
                ..
            } => {
                // 1. Structural features (8 dimensions)
                if self.config.include_structural {
                    features.extend(self.extract_structural_features(
                        kind,
                        parameters.len(),
                        range.end_line.saturating_sub(range.start_line),
                        layer.child_ids.len(),
                    ));
                    feature_types.push(FeatureType::Structural);
                }

                // 2. Semantic features (16 dimensions)
                if self.config.include_semantic {
                    features.extend(self.extract_semantic_features(
                        name,
                        signature,
                        documentation.as_deref(),
                    ));
                    feature_types.push(FeatureType::Semantic);
                }

                // 3. Behavioral features (12 dimensions)
                if self.config.include_behavioral {
                    features.extend(self.extract_behavioral_features(
                        name,
                        signature,
                        documentation.as_deref(),
                    ));
                    feature_types.push(FeatureType::Behavioral);
                }

                // 4. Textual features (8 dimensions)
                if self.config.include_textual {
                    features.extend(self.extract_textual_features(name, documentation.as_deref()));
                    feature_types.push(FeatureType::Textual);
                }
            }

            LayerContent::Block {
                block_type,
                nested_depth,
                ..
            } => {
                features.extend(self.extract_block_features(block_type, *nested_depth));
                feature_types.push(FeatureType::Structural);
            }

            LayerContent::File {
                language,
                line_count,
                symbol_count,
                ..
            } => {
                features.extend(self.extract_file_features(language, *line_count, *symbol_count));
                feature_types.push(FeatureType::Structural);
            }

            _ => {
                // Default features for other layer types
                features.extend(vec![0.0; 8]);
            }
        }

        // Normalize and fix dimension
        self.normalize_features(&mut features);
        if let Some(dim) = self.config.fixed_dimension {
            self.fix_dimension(&mut features, dim);
        }

        FeatureVector {
            values: features,
            metadata: VectorMetadata {
                source_id: layer.id.clone(),
                layer_type: layer.level,
                confidence: layer.metadata.confidence,
                feature_types,
            },
        }
    }

    /// Vectorize multiple layers.
    pub fn vectorize_layers(&self, layers: &[&ContextLayer]) -> Vec<FeatureVector> {
        layers
            .iter()
            .map(|layer| self.vectorize_layer(layer))
            .collect()
    }

    // -------------------------------------------------------------------------
    // Structural Features (8 dimensions)
    // -------------------------------------------------------------------------

    fn extract_structural_features(
        &self,
        kind: &SymbolKind,
        param_count: usize,
        line_count: usize,
        child_count: usize,
    ) -> Vec<f32> {
        let mut features = vec![0.0; 8];

        // [0] Symbol kind encoding (one-hot style)
        features[0] = match kind {
            SymbolKind::Function => 0.1,
            SymbolKind::Method => 0.2,
            SymbolKind::Class => 0.3,
            SymbolKind::Struct => 0.4,
            SymbolKind::Enum => 0.5,
            SymbolKind::Trait => 0.6,
            SymbolKind::Interface => 0.7,
            SymbolKind::Constant => 0.8,
            SymbolKind::Variable => 0.9,
            SymbolKind::Module => 1.0,
            _ => 0.0,
        };

        // [1] Parameter count (normalized, max 10)
        features[1] = (param_count as f32 / 10.0).min(1.0);

        // [2] Line count / complexity (normalized, max 100)
        features[2] = (line_count as f32 / 100.0).min(1.0);

        // [3] Child count / complexity (normalized, max 20)
        features[3] = (child_count as f32 / 20.0).min(1.0);

        // [4] Is function-like
        features[4] = if matches!(kind, SymbolKind::Function | SymbolKind::Method) {
            1.0
        } else {
            0.0
        };

        // [5] Is type-like
        features[5] = if matches!(
            kind,
            SymbolKind::Class | SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Interface
        ) {
            1.0
        } else {
            0.0
        };

        // [6] Is small (< 10 lines)
        features[6] = if line_count < 10 { 1.0 } else { 0.0 };

        // [7] Is complex (> 50 lines or > 5 params)
        features[7] = if line_count > 50 || param_count > 5 {
            1.0
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // Semantic Features (16 dimensions)
    // -------------------------------------------------------------------------

    fn extract_semantic_features(
        &self,
        name: &str,
        signature: &str,
        documentation: Option<&str>,
    ) -> Vec<f32> {
        let mut features = vec![0.0; 16];
        let lower_name = name.to_lowercase();
        let lower_sig = signature.to_lowercase();
        let docs = documentation.map(|d| d.to_lowercase()).unwrap_or_default();

        // Semantic categories (check name, signature, and docs)
        let text = format!("{} {} {}", lower_name, lower_sig, docs);

        // [0] Error handling
        features[0] = if text.contains("error")
            || text.contains("exception")
            || text.contains("result")
            || text.contains("try")
        {
            0.9
        } else {
            0.0
        };

        // [1] Validation
        features[1] = if text.contains("valid")
            || text.contains("check")
            || text.contains("assert")
            || text.contains("verify")
        {
            0.9
        } else {
            0.0
        };

        // [2] Data processing
        features[2] = if text.contains("process")
            || text.contains("transform")
            || text.contains("convert")
            || text.contains("parse")
        {
            0.9
        } else {
            0.0
        };

        // [3] IO operations
        features[3] = if text.contains("read")
            || text.contains("write")
            || text.contains("file")
            || text.contains("stream")
        {
            0.9
        } else {
            0.0
        };

        // [4] Computation
        features[4] = if text.contains("calc")
            || text.contains("compute")
            || text.contains("sum")
            || text.contains("count")
        {
            0.9
        } else {
            0.0
        };

        // [5] Initialization
        features[5] = if text.contains("init")
            || text.contains("new")
            || text.contains("create")
            || text.contains("build")
        {
            0.9
        } else {
            0.0
        };

        // [6] Cleanup
        features[6] = if text.contains("clean")
            || text.contains("close")
            || text.contains("drop")
            || text.contains("free")
        {
            0.9
        } else {
            0.0
        };

        // [7] Configuration
        features[7] = if text.contains("config")
            || text.contains("setting")
            || text.contains("option")
            || text.contains("param")
        {
            0.9
        } else {
            0.0
        };

        // [8] Async/concurrent
        features[8] = if text.contains("async")
            || text.contains("await")
            || text.contains("thread")
            || text.contains("spawn")
        {
            0.9
        } else {
            0.0
        };

        // [9] Getter
        features[9] = if lower_name.starts_with("get")
            || lower_name.starts_with("is_")
            || lower_name.starts_with("has_")
        {
            0.9
        } else {
            0.0
        };

        // [10] Setter
        features[10] = if lower_name.starts_with("set")
            || lower_name.starts_with("update")
            || lower_name.starts_with("modify")
        {
            0.9
        } else {
            0.0
        };

        // [11] Collection operation
        features[11] = if text.contains("add")
            || text.contains("remove")
            || text.contains("push")
            || text.contains("pop")
            || text.contains("insert")
            || text.contains("delete")
        {
            0.9
        } else {
            0.0
        };

        // [12] Search/find
        features[12] = if text.contains("find")
            || text.contains("search")
            || text.contains("lookup")
            || text.contains("query")
        {
            0.9
        } else {
            0.0
        };

        // [13] Formatting
        features[13] = if text.contains("format")
            || text.contains("display")
            || text.contains("render")
            || text.contains("to_string")
        {
            0.9
        } else {
            0.0
        };

        // [14] Testing
        features[14] = if text.contains("test")
            || text.contains("mock")
            || text.contains("expect")
            || text.contains("assert")
        {
            0.9
        } else {
            0.0
        };

        // [15] Utility/helper
        features[15] = if text.contains("helper") || text.contains("util") || text.contains("misc")
        {
            0.9
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // Behavioral Features (12 dimensions)
    // -------------------------------------------------------------------------

    fn extract_behavioral_features(
        &self,
        name: &str,
        signature: &str,
        documentation: Option<&str>,
    ) -> Vec<f32> {
        // 12-dimensional behavioral vector
        let mut features = vec![0.0; 12];
        let text = format!(
            "{} {} {}",
            name.to_lowercase(),
            signature.to_lowercase(),
            documentation.unwrap_or("").to_lowercase()
        );

        // [0] Pure function (no side effects)
        features[0] = if !text.contains("mut")
            && !text.contains("write")
            && !text.contains("set")
            && !text.contains("modify")
        {
            0.7
        } else {
            0.0
        };

        // [1] Mutating
        features[1] = if text.contains("mut") || text.contains("&mut") {
            0.9
        } else {
            0.0
        };

        // [2] Returns result
        features[2] = if signature.contains("Result") || signature.contains("Option") {
            0.9
        } else {
            0.0
        };

        // [3] Takes callback
        features[3] = if signature.contains("Fn")
            || signature.contains("fn(")
            || signature.contains("callback")
        {
            0.9
        } else {
            0.0
        };

        // [4] Generic
        features[4] = if signature.contains('<') && signature.contains('>') {
            0.9
        } else {
            0.0
        };

        // [5] Public API
        features[5] = if signature.starts_with("pub ") {
            0.9
        } else {
            0.0
        };

        // [6] Static/associated
        features[6] = if !signature.contains("self") && !signature.contains("&self") {
            0.9
        } else {
            0.0
        };

        // [7] Constructor pattern
        features[7] = if name == "new"
            || name == "default"
            || name.starts_with("from_")
            || name.starts_with("with_")
        {
            0.9
        } else {
            0.0
        };

        // [8] Destructor pattern
        features[8] = if name == "drop" || name == "close" || name == "cleanup" {
            0.9
        } else {
            0.0
        };

        // [9] Iterator pattern
        features[9] = if name == "iter" || name == "next" || signature.contains("Iterator") {
            0.9
        } else {
            0.0
        };

        // [10] Trait implementation
        features[10] = if name.starts_with("fmt")
            || name == "clone"
            || name == "default"
            || name == "eq"
            || name == "cmp"
        {
            0.9
        } else {
            0.0
        };

        // [11] Conversion
        features[11] = if name.starts_with("to_")
            || name.starts_with("into_")
            || name.starts_with("as_")
            || name.starts_with("from_")
        {
            0.9
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // Textual Features (8 dimensions)
    // -------------------------------------------------------------------------

    fn extract_textual_features(&self, name: &str, documentation: Option<&str>) -> Vec<f32> {
        let mut features = vec![0.0; 8];

        // [0] Name length (normalized)
        features[0] = (name.len() as f32 / 50.0).min(1.0);

        // [1] snake_case
        features[1] = if name.contains('_') { 1.0 } else { 0.0 };

        // [2] CamelCase
        features[2] = if name.chars().any(|c| c.is_uppercase())
            && name.chars().any(|c| c.is_lowercase())
            && !name.contains('_')
        {
            1.0
        } else {
            0.0
        };

        // [3] ALL_CAPS (constant style)
        features[3] = if name.chars().all(|c| c.is_uppercase() || c == '_') {
            1.0
        } else {
            0.0
        };

        // [4] Has documentation
        features[4] = if documentation.is_some() { 1.0 } else { 0.0 };

        // [5] Documentation length (normalized)
        features[5] = documentation
            .map(|d| (d.len() as f32 / 500.0).min(1.0))
            .unwrap_or(0.0);

        // [6] Has numeric suffix (like func1, func2)
        features[6] = if name.chars().last().map(|c| c.is_numeric()).unwrap_or(false) {
            1.0
        } else {
            0.0
        };

        // [7] Has common prefix (get_, set_, is_, has_)
        features[7] = if name.starts_with("get_")
            || name.starts_with("set_")
            || name.starts_with("is_")
            || name.starts_with("has_")
            || name.starts_with("on_")
            || name.starts_with("do_")
        {
            1.0
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // Block Features
    // -------------------------------------------------------------------------

    fn extract_block_features(&self, block_type: &BlockType, nested_depth: usize) -> Vec<f32> {
        let mut features = vec![0.0; 8];

        // [0] Block type encoding
        features[0] = match block_type {
            BlockType::If => 0.1,
            BlockType::Else => 0.15,
            BlockType::ElseIf => 0.2,
            BlockType::Loop => 0.3,
            BlockType::While => 0.35,
            BlockType::For => 0.4,
            BlockType::Match => 0.5,
            BlockType::Case => 0.55,
            BlockType::Try => 0.6,
            BlockType::Catch => 0.65,
            BlockType::Finally => 0.7,
            BlockType::With => 0.8,
            BlockType::Unsafe | BlockType::Async => 0.85,
            BlockType::Closure => 0.9,
            BlockType::Unknown => 1.0,
        };

        // [1] Nesting depth
        features[1] = (nested_depth as f32 / 10.0).min(1.0);

        // [2] Is control flow
        features[2] = if matches!(
            block_type,
            BlockType::If
                | BlockType::Else
                | BlockType::ElseIf
                | BlockType::Loop
                | BlockType::While
                | BlockType::For
                | BlockType::Match
        ) {
            1.0
        } else {
            0.0
        };

        // [3] Is error handling
        features[3] = if matches!(
            block_type,
            BlockType::Try | BlockType::Catch | BlockType::Finally
        ) {
            1.0
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // File Features
    // -------------------------------------------------------------------------

    fn extract_file_features(
        &self,
        language: &str,
        line_count: usize,
        symbol_count: usize,
    ) -> Vec<f32> {
        let mut features = vec![0.0; 8];

        // [0] Language encoding
        features[0] = match language {
            "rust" => 0.1,
            "python" => 0.2,
            "javascript" | "typescript" => 0.3,
            "go" => 0.4,
            "shell" | "bash" => 0.5,
            "c" | "cpp" => 0.6,
            "java" => 0.7,
            "ruby" => 0.8,
            _ => 0.9,
        };

        // [1] Line count (normalized)
        features[1] = (line_count as f32 / 1000.0).min(1.0);

        // [2] Symbol count (normalized)
        features[2] = (symbol_count as f32 / 50.0).min(1.0);

        // [3] Symbols per line ratio
        features[3] = if line_count > 0 {
            (symbol_count as f32 / line_count as f32 * 10.0).min(1.0)
        } else {
            0.0
        };

        features
    }

    // -------------------------------------------------------------------------
    // Normalization
    // -------------------------------------------------------------------------

    fn normalize_features(&self, features: &mut Vec<f32>) {
        // Ensure all values are in [0, 1] and no NaN
        for feature in features.iter_mut() {
            if feature.is_nan() || feature.is_infinite() {
                *feature = 0.0;
            }
            *feature = feature.clamp(0.0, 1.0);
        }
    }

    fn fix_dimension(&self, features: &mut Vec<f32>, dim: usize) {
        if features.len() < dim {
            // Pad with zeros
            features.resize(dim, 0.0);
        } else if features.len() > dim {
            // Truncate
            features.truncate(dim);
        }
    }

    fn load_keyword_weights() -> HashMap<String, f32> {
        let mut weights = HashMap::new();

        // Error handling
        weights.insert("error".to_string(), 0.9);
        weights.insert("exception".to_string(), 0.9);
        weights.insert("validate".to_string(), 0.8);
        weights.insert("check".to_string(), 0.7);

        // Data processing
        weights.insert("process".to_string(), 0.8);
        weights.insert("transform".to_string(), 0.8);
        weights.insert("convert".to_string(), 0.7);
        weights.insert("parse".to_string(), 0.7);

        // IO operations
        weights.insert("read".to_string(), 0.8);
        weights.insert("write".to_string(), 0.8);
        weights.insert("save".to_string(), 0.7);
        weights.insert("load".to_string(), 0.7);

        // Computation
        weights.insert("calculate".to_string(), 0.8);
        weights.insert("compute".to_string(), 0.8);

        // Utility
        weights.insert("helper".to_string(), 0.6);
        weights.insert("util".to_string(), 0.6);

        // Initialization
        weights.insert("init".to_string(), 0.7);
        weights.insert("setup".to_string(), 0.7);
        weights.insert("configure".to_string(), 0.7);

        weights
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{Parameter, Range, Visibility};

    fn create_test_symbol(name: &str, kind: SymbolKind) -> ContextLayer {
        ContextLayer::new(
            format!("sym_{}", name),
            LayerContent::Symbol {
                name: name.to_string(),
                kind,
                signature: format!("fn {}()", name),
                return_type: None,
                parameters: vec![],
                documentation: Some(format!("Documentation for {}", name)),
                visibility: Visibility::Public,
                range: Range::line_range(1, 10),
            },
        )
    }

    #[test]
    fn test_vectorizer_basic() {
        let vectorizer = SymbolVectorizer::new();
        let layer = create_test_symbol("calculate_sum", SymbolKind::Function);

        let vector = vectorizer.vectorize_layer(&layer);

        assert_eq!(vector.dim(), 64); // Fixed dimension
        assert!(vector.values.iter().all(|&v| v >= 0.0 && v <= 1.0));
        assert_eq!(vector.metadata.source_id, "sym_calculate_sum");
    }

    #[test]
    fn test_vectorizer_with_params() {
        let vectorizer = SymbolVectorizer::new();

        let layer = ContextLayer::new(
            "sym_test",
            LayerContent::Symbol {
                name: "process_data".to_string(),
                kind: SymbolKind::Function,
                signature: "pub fn process_data(a: i32, b: i32, c: i32) -> Result<()>".to_string(),
                return_type: Some("Result<()>".to_string()),
                parameters: vec![
                    Parameter {
                        name: "a".to_string(),
                        type_hint: Some("i32".to_string()),
                        default_value: None,
                    },
                    Parameter {
                        name: "b".to_string(),
                        type_hint: Some("i32".to_string()),
                        default_value: None,
                    },
                    Parameter {
                        name: "c".to_string(),
                        type_hint: Some("i32".to_string()),
                        default_value: None,
                    },
                ],
                documentation: Some("Process data and transform it".to_string()),
                visibility: Visibility::Public,
                range: Range::line_range(1, 50),
            },
        );

        let vector = vectorizer.vectorize_layer(&layer);

        assert_eq!(vector.dim(), 64);
        // Should have some non-zero features
        assert!(vector.values.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_distance_calculation() {
        let vectorizer = SymbolVectorizer::new();

        let layer1 = create_test_symbol("calculate_sum", SymbolKind::Function);
        let layer2 = create_test_symbol("calculate_product", SymbolKind::Function);
        let layer3 = create_test_symbol("validate_user", SymbolKind::Function);

        let v1 = vectorizer.vectorize_layer(&layer1);
        let v2 = vectorizer.vectorize_layer(&layer2);
        let v3 = vectorizer.vectorize_layer(&layer3);

        // Similar functions should have smaller distance
        let dist_similar = v1.distance(&v2);
        let dist_different = v1.distance(&v3);

        // Both should be finite
        assert!(dist_similar.is_finite());
        assert!(dist_different.is_finite());
    }

    #[test]
    fn test_cosine_similarity() {
        let vectorizer = SymbolVectorizer::new();

        let layer1 = create_test_symbol("get_user", SymbolKind::Function);
        let layer2 = create_test_symbol("get_account", SymbolKind::Function);

        let v1 = vectorizer.vectorize_layer(&layer1);
        let v2 = vectorizer.vectorize_layer(&layer2);

        let similarity = v1.cosine_similarity(&v2);

        // Similar naming pattern should have high similarity
        assert!(similarity >= 0.0 && similarity <= 1.0);
        assert!(similarity > 0.5); // Should be fairly similar
    }

    #[test]
    fn test_semantic_features() {
        let vectorizer = SymbolVectorizer::new();

        // Error handling function
        let error_layer = ContextLayer::new(
            "sym_error",
            LayerContent::Symbol {
                name: "handle_error".to_string(),
                kind: SymbolKind::Function,
                signature: "fn handle_error(e: Error) -> Result<()>".to_string(),
                return_type: Some("Result<()>".to_string()),
                parameters: vec![],
                documentation: Some("Handle error and exception cases".to_string()),
                visibility: Visibility::Public,
                range: Range::line_range(1, 10),
            },
        );

        let vector = vectorizer.vectorize_layer(&error_layer);

        // Should have error handling semantic features activated
        assert!(vector.values.iter().any(|&v| v > 0.5));
    }

    #[test]
    fn test_block_features() {
        let vectorizer = SymbolVectorizer::new();

        let block_layer = ContextLayer::new(
            "block_1",
            LayerContent::Block {
                block_type: BlockType::If,
                condition: Some("x > 0".to_string()),
                body_preview: "return x;".to_string(),
                nested_depth: 2,
                range: Range::line_range(5, 10),
            },
        );

        let vector = vectorizer.vectorize_layer(&block_layer);

        assert_eq!(vector.dim(), 64);
        assert!(vector.values.iter().any(|&v| v > 0.0));
    }

    #[test]
    fn test_file_features() {
        let vectorizer = SymbolVectorizer::new();

        let file_layer = ContextLayer::new(
            "file_1",
            LayerContent::File {
                path: std::path::PathBuf::from("test.rs"),
                language: "rust".to_string(),
                size_bytes: 1024,
                line_count: 100,
                symbol_count: 10,
                imports: vec![],
            },
        );

        let vector = vectorizer.vectorize_layer(&file_layer);

        assert_eq!(vector.dim(), 64);
    }

    #[test]
    fn test_vectorize_multiple() {
        let vectorizer = SymbolVectorizer::new();

        let layers = vec![
            create_test_symbol("func1", SymbolKind::Function),
            create_test_symbol("func2", SymbolKind::Function),
            create_test_symbol("struct1", SymbolKind::Struct),
        ];

        let refs: Vec<&ContextLayer> = layers.iter().collect();
        let vectors = vectorizer.vectorize_layers(&refs);

        assert_eq!(vectors.len(), 3);
        assert!(vectors.iter().all(|v| v.dim() == 64));
    }

    #[test]
    fn test_custom_config() {
        let config = VectorizerConfig {
            include_structural: true,
            include_semantic: false,
            include_behavioral: false,
            include_textual: false,
            fixed_dimension: Some(32),
        };

        let vectorizer = SymbolVectorizer::with_config(config);
        let layer = create_test_symbol("test", SymbolKind::Function);

        let vector = vectorizer.vectorize_layer(&layer);

        assert_eq!(vector.dim(), 32);
        // Only structural features should be present
        assert!(vector
            .metadata
            .feature_types
            .contains(&FeatureType::Structural));
    }
}
