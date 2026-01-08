//! Semantic Substrate Determinism Tests
//!
//! Integration tests verifying that UnifiedSemanticSubstrate produces
//! identical iteration order across multiple runs. This validates the BTreeMap
//! migration for deterministic iteration order.

use pm_encoder::core::fractal::clustering::vectorizer::{FeatureType, VectorMetadata};
use pm_encoder::core::fractal::{
    ContextLayer, FeatureVector, Language, LayerContent, Range, SymbolKind,
    UnifiedSemanticSubstrate, Visibility, ZoomLevel,
};

/// Create a symbol layer with the given properties
fn create_symbol_layer(
    name: &str,
    kind: SymbolKind,
    signature: &str,
    doc: Option<&str>,
) -> ContextLayer {
    ContextLayer::new(
        format!("layer_{}", name),
        LayerContent::Symbol {
            name: name.to_string(),
            kind,
            signature: signature.to_string(),
            return_type: None,
            parameters: vec![],
            documentation: doc.map(|s| s.to_string()),
            visibility: Visibility::Public,
            range: Range {
                start_line: 1,
                start_col: 0,
                end_line: 10,
                end_col: 0,
            },
        },
    )
}

/// Create a feature vector for testing
fn create_feature_vector(source_id: &str, confidence: f32) -> FeatureVector {
    FeatureVector {
        values: vec![0.5; 64],
        metadata: VectorMetadata {
            source_id: source_id.to_string(),
            layer_type: ZoomLevel::Symbol,
            confidence,
            feature_types: vec![FeatureType::Semantic],
        },
    }
}

/// Create a fixed set of mock layers for determinism testing (Python)
fn create_python_layers() -> (Vec<ContextLayer>, Vec<FeatureVector>) {
    let layers = vec![
        create_symbol_layer(
            "calculate_total",
            SymbolKind::Function,
            "def calculate_total(items: list) -> float",
            Some("Calculates the total price"),
        ),
        create_symbol_layer(
            "validate_input",
            SymbolKind::Function,
            "def validate_input(data: dict) -> bool",
            None,
        ),
        create_symbol_layer(
            "process_order",
            SymbolKind::Function,
            "def process_order(order_id: str) -> Order",
            Some("Process an order"),
        ),
    ];
    let vectors = vec![
        create_feature_vector("python:calculate_total", 0.9),
        create_feature_vector("python:validate_input", 0.85),
        create_feature_vector("python:process_order", 0.88),
    ];
    (layers, vectors)
}

/// Create a fixed set of mock layers for determinism testing (Rust)
fn create_rust_layers() -> (Vec<ContextLayer>, Vec<FeatureVector>) {
    let layers = vec![
        create_symbol_layer(
            "calculate_total",
            SymbolKind::Function,
            "pub fn calculate_total(items: &[Item]) -> f64",
            Some("Calculates the total amount"),
        ),
        create_symbol_layer(
            "UserConfig",
            SymbolKind::Struct,
            "pub struct UserConfig",
            None,
        ),
        create_symbol_layer(
            "handle_request",
            SymbolKind::Function,
            "pub async fn handle_request(req: Request) -> Response",
            Some("Handle HTTP request"),
        ),
    ];
    let vectors = vec![
        create_feature_vector("rust:calculate_total", 0.95),
        create_feature_vector("rust:UserConfig", 0.7),
        create_feature_vector("rust:handle_request", 0.92),
    ];
    (layers, vectors)
}

/// Create a fixed set of mock layers for determinism testing (TypeScript)
fn create_typescript_layers() -> (Vec<ContextLayer>, Vec<FeatureVector>) {
    let layers = vec![
        create_symbol_layer(
            "calculateTotal",
            SymbolKind::Function,
            "function calculateTotal(items: Item[]): number",
            Some("Calculates the total price"),
        ),
        create_symbol_layer(
            "IUserConfig",
            SymbolKind::Interface,
            "interface IUserConfig",
            None,
        ),
    ];
    let vectors = vec![
        create_feature_vector("ts:calculateTotal", 0.85),
        create_feature_vector("ts:IUserConfig", 0.75),
    ];
    (layers, vectors)
}

/// Test: Concepts are iterated in deterministic order
#[test]
fn test_concept_iteration_order_determinism() {
    let (layers, vectors) = create_python_layers();

    // Create substrate multiple times and check iteration order
    let mut concept_orders: Vec<Vec<String>> = Vec::new();

    for _ in 0..10 {
        let substrate = UnifiedSemanticSubstrate::from_layers(
            &layers,
            &vectors,
            Language::Python,
            "service.py",
        );
        let order: Vec<String> = substrate.concepts().map(|c| c.name.clone()).collect();
        concept_orders.push(order);
    }

    // All orders should be identical
    let first = &concept_orders[0];
    for (i, order) in concept_orders.iter().enumerate().skip(1) {
        assert_eq!(
            first, order,
            "Run {} produced different concept order than run 0.\nExpected: {:?}\n\nGot: {:?}",
            i, first, order
        );
    }
}

/// Test: Language contributions are iterated in deterministic order
#[test]
fn test_language_contribution_order_determinism() {
    // Create multi-language substrate
    let (py_layers, py_vectors) = create_python_layers();
    let (rs_layers, rs_vectors) = create_rust_layers();
    let (ts_layers, ts_vectors) = create_typescript_layers();

    let mut language_orders: Vec<Vec<Language>> = Vec::new();

    for _ in 0..10 {
        let mut substrate = UnifiedSemanticSubstrate::from_layers(
            &py_layers,
            &py_vectors,
            Language::Python,
            "service.py",
        );

        let rust_substrate = UnifiedSemanticSubstrate::from_layers(
            &rs_layers,
            &rs_vectors,
            Language::Rust,
            "service.rs",
        );

        let ts_substrate = UnifiedSemanticSubstrate::from_layers(
            &ts_layers,
            &ts_vectors,
            Language::TypeScript,
            "service.ts",
        );

        substrate.merge(rust_substrate);
        substrate.merge(ts_substrate);

        let order: Vec<Language> = substrate.languages();
        language_orders.push(order);
    }

    // All orders should be identical
    let first = &language_orders[0];
    for (i, order) in language_orders.iter().enumerate().skip(1) {
        assert_eq!(
            first, order,
            "Run {} produced different language order than run 0.\nExpected: {:?}\n\nGot: {:?}",
            i, first, order
        );
    }
}

/// Test: Merged substrates maintain deterministic order
#[test]
fn test_merge_determinism() {
    let (py_layers, py_vectors) = create_python_layers();
    let (rs_layers, rs_vectors) = create_rust_layers();

    let mut merged_concept_orders: Vec<Vec<String>> = Vec::new();

    for _ in 0..10 {
        let mut substrate1 = UnifiedSemanticSubstrate::from_layers(
            &py_layers,
            &py_vectors,
            Language::Python,
            "service.py",
        );
        let substrate2 = UnifiedSemanticSubstrate::from_layers(
            &rs_layers,
            &rs_vectors,
            Language::Rust,
            "service.rs",
        );
        substrate1.merge(substrate2);

        let order: Vec<String> = substrate1.concepts().map(|c| c.name.clone()).collect();
        merged_concept_orders.push(order);
    }

    // All outputs should be identical
    let first = &merged_concept_orders[0];
    for (i, order) in merged_concept_orders.iter().enumerate().skip(1) {
        assert_eq!(
            first, order,
            "Merge run {} produced different concept order than run 0.\nExpected: {:?}\n\nGot: {:?}",
            i, first, order
        );
    }
}

/// Test: Concept embedding similarity is deterministic
#[test]
fn test_concept_similarity_determinism() {
    let (py_layers, py_vectors) = create_python_layers();

    // Create two substrates from same data
    let substrate1 = UnifiedSemanticSubstrate::from_layers(
        &py_layers,
        &py_vectors,
        Language::Python,
        "service.py",
    );
    let substrate2 = UnifiedSemanticSubstrate::from_layers(
        &py_layers,
        &py_vectors,
        Language::Python,
        "service.py",
    );

    // Compare first concept from each - should have same embedding
    let concepts1: Vec<_> = substrate1.concepts().collect();
    let concepts2: Vec<_> = substrate2.concepts().collect();

    assert_eq!(concepts1.len(), concepts2.len());

    // Each concept should have the same embedding as its counterpart
    for (c1, c2) in concepts1.iter().zip(concepts2.iter()) {
        assert_eq!(c1.name, c2.name, "Concept names should match");
        let sim = c1.embedding_similarity(c2);
        assert!(
            (sim - 1.0).abs() < 0.001,
            "Identical concepts should have similarity ~1.0, got {}",
            sim
        );
    }
}

/// Test: languages() returns sorted Language enum in deterministic order
#[test]
fn test_languages_determinism() {
    let (py_layers, py_vectors) = create_python_layers();
    let (rs_layers, rs_vectors) = create_rust_layers();
    let (ts_layers, ts_vectors) = create_typescript_layers();

    let mut language_vec_orders: Vec<Vec<Language>> = Vec::new();

    for _ in 0..10 {
        let mut substrate = UnifiedSemanticSubstrate::from_layers(
            &ts_layers,
            &ts_vectors,
            Language::TypeScript,
            "service.ts",
        );

        let rust_substrate = UnifiedSemanticSubstrate::from_layers(
            &rs_layers,
            &rs_vectors,
            Language::Rust,
            "service.rs",
        );

        let python_substrate = UnifiedSemanticSubstrate::from_layers(
            &py_layers,
            &py_vectors,
            Language::Python,
            "service.py",
        );

        // Merge in different order to test determinism
        substrate.merge(rust_substrate);
        substrate.merge(python_substrate);

        let languages = substrate.languages();
        language_vec_orders.push(languages);
    }

    // All orders should be identical
    let first = &language_vec_orders[0];
    for (i, order) in language_vec_orders.iter().enumerate().skip(1) {
        assert_eq!(
            first, order,
            "Run {} produced different languages() order than run 0.\nExpected: {:?}\n\nGot: {:?}",
            i, first, order
        );
    }

    // Verify we have all three languages
    assert_eq!(language_vec_orders[0].len(), 3);
    assert!(language_vec_orders[0].contains(&Language::Python));
    assert!(language_vec_orders[0].contains(&Language::Rust));
    assert!(language_vec_orders[0].contains(&Language::TypeScript));
}

/// Test: Concept count is consistent across runs
#[test]
fn test_concept_count_consistency() {
    let (py_layers, py_vectors) = create_python_layers();
    let (rs_layers, rs_vectors) = create_rust_layers();

    let mut counts: Vec<usize> = Vec::new();

    for _ in 0..10 {
        let mut substrate = UnifiedSemanticSubstrate::from_layers(
            &py_layers,
            &py_vectors,
            Language::Python,
            "service.py",
        );
        let rust_substrate = UnifiedSemanticSubstrate::from_layers(
            &rs_layers,
            &rs_vectors,
            Language::Rust,
            "service.rs",
        );
        substrate.merge(rust_substrate);
        counts.push(substrate.concept_count());
    }

    // All counts should be identical
    let first = counts[0];
    for (i, count) in counts.iter().enumerate().skip(1) {
        assert_eq!(
            first, *count,
            "Run {} produced different concept count: {} vs {}",
            i, first, count
        );
    }

    // Expected: 3 Python + 3 Rust = 6 concepts
    assert_eq!(first, 6);
}
