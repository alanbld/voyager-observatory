//! Fractal Context Navigation
//!
//! This module provides interactive navigation through hierarchical fractal contexts.
//! It enables zoom in/out, sibling traversal, and history-based navigation.
//!
//! # Example
//!
//! ```rust,ignore
//! use pm_encoder::core::fractal::{FractalContextBuilder, FractalNavigator};
//!
//! let context = FractalContextBuilder::for_file("src/main.rs").build()?;
//! let mut navigator = FractalNavigator::new(context);
//!
//! // Get current layer
//! let current = navigator.current_layer()?;
//! println!("At: {} ({})", current.name(), current.level);
//!
//! // Zoom into first child
//! if navigator.zoom_in_first()? {
//!     println!("Zoomed into: {}", navigator.current_layer()?.name());
//! }
//!
//! // Go back
//! navigator.back()?;
//! ```

use std::collections::VecDeque;

use thiserror::Error;

use super::context::FractalContext;
use super::layers::{ContextLayer, ZoomLevel};

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during navigation.
#[derive(Debug, Error)]
pub enum NavigationError {
    #[error("Layer not found: {0}")]
    LayerNotFound(String),

    #[error("No parent layer available - already at root")]
    NoParentLayer,

    #[error("No children available - already at deepest level")]
    NoChildren,

    #[error("No siblings available")]
    NoSiblings,

    #[error("No history available")]
    NoHistory,

    #[error("No forward history available")]
    NoForwardHistory,

    #[error("Invalid navigation: {0}")]
    InvalidNavigation(String),

    #[error("Context not loaded")]
    NoContext,
}

pub type NavigationResult<T> = Result<T, NavigationError>;

// =============================================================================
// Navigation Direction
// =============================================================================

/// Direction for sibling navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiblingDirection {
    Next,
    Previous,
}

/// Pan direction for horizontal navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanDirection {
    Left,
    Right,
}

// =============================================================================
// Navigation State
// =============================================================================

/// Snapshot of navigation state for history.
#[derive(Debug, Clone)]
pub struct NavigationSnapshot {
    pub layer_id: String,
    pub level: ZoomLevel,
    pub timestamp: std::time::Instant,
}

/// Statistics about navigation session.
#[derive(Debug, Clone, Default)]
pub struct NavigationStats {
    pub total_navigations: usize,
    pub zoom_ins: usize,
    pub zoom_outs: usize,
    pub sibling_moves: usize,
    pub back_navigations: usize,
    pub forward_navigations: usize,
}

// =============================================================================
// FractalNavigator
// =============================================================================

/// Interactive navigator for fractal contexts.
///
/// The navigator maintains a current position in the fractal hierarchy
/// and provides methods to move through it. It also maintains history
/// for back/forward navigation.
#[derive(Debug)]
pub struct FractalNavigator {
    /// The fractal context being navigated
    context: FractalContext,
    /// Current layer ID
    current_layer_id: String,
    /// Navigation history (for back)
    history: VecDeque<NavigationSnapshot>,
    /// Forward history (for redo after back)
    forward: VecDeque<NavigationSnapshot>,
    /// Maximum history size
    max_history: usize,
    /// Navigation statistics
    stats: NavigationStats,
}

impl FractalNavigator {
    /// Create a new navigator for a fractal context.
    ///
    /// Starts at the root layer of the context.
    pub fn new(context: FractalContext) -> Self {
        let current_layer_id = context.root_id.clone();

        Self {
            context,
            current_layer_id,
            history: VecDeque::new(),
            forward: VecDeque::new(),
            max_history: 100,
            stats: NavigationStats::default(),
        }
    }

    /// Create a navigator starting at a specific layer.
    pub fn new_at(context: FractalContext, layer_id: &str) -> NavigationResult<Self> {
        if !context.layers.contains_key(layer_id) {
            return Err(NavigationError::LayerNotFound(layer_id.to_string()));
        }

        Ok(Self {
            current_layer_id: layer_id.to_string(),
            context,
            history: VecDeque::new(),
            forward: VecDeque::new(),
            max_history: 100,
            stats: NavigationStats::default(),
        })
    }

    /// Get the current layer.
    pub fn current_layer(&self) -> NavigationResult<&ContextLayer> {
        self.context
            .layers
            .get(&self.current_layer_id)
            .ok_or_else(|| NavigationError::LayerNotFound(self.current_layer_id.clone()))
    }

    /// Get the current zoom level.
    pub fn current_level(&self) -> NavigationResult<ZoomLevel> {
        Ok(self.current_layer()?.level)
    }

    /// Get the current layer ID.
    pub fn current_id(&self) -> &str {
        &self.current_layer_id
    }

    /// Get reference to the underlying context.
    pub fn context(&self) -> &FractalContext {
        &self.context
    }

    /// Get mutable reference to the underlying context.
    pub fn context_mut(&mut self) -> &mut FractalContext {
        &mut self.context
    }

    /// Get navigation statistics.
    pub fn stats(&self) -> &NavigationStats {
        &self.stats
    }

    // =========================================================================
    // Zoom Navigation
    // =========================================================================

    /// Zoom into a specific child layer.
    pub fn zoom_in(&mut self, target_id: &str) -> NavigationResult<&ContextLayer> {
        let current = self.current_layer()?;

        // Verify target is a child
        if !current.child_ids.contains(&target_id.to_string()) {
            // Check if it exists at all
            if !self.context.layers.contains_key(target_id) {
                return Err(NavigationError::LayerNotFound(target_id.to_string()));
            }
            return Err(NavigationError::InvalidNavigation(format!(
                "'{}' is not a child of current layer",
                target_id
            )));
        }

        self.push_history();
        self.current_layer_id = target_id.to_string();
        self.stats.zoom_ins += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Zoom into the first child layer.
    pub fn zoom_in_first(&mut self) -> NavigationResult<&ContextLayer> {
        let first_child = {
            let current = self.current_layer()?;
            current
                .child_ids
                .first()
                .cloned()
                .ok_or(NavigationError::NoChildren)?
        };

        self.push_history();
        self.current_layer_id = first_child;
        self.stats.zoom_ins += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Zoom into a child by index.
    pub fn zoom_in_nth(&mut self, index: usize) -> NavigationResult<&ContextLayer> {
        let child_id = {
            let current = self.current_layer()?;
            current
                .child_ids
                .get(index)
                .cloned()
                .ok_or(NavigationError::NoChildren)?
        };

        self.push_history();
        self.current_layer_id = child_id;
        self.stats.zoom_ins += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Zoom out to parent layer.
    pub fn zoom_out(&mut self) -> NavigationResult<&ContextLayer> {
        let parent_id = {
            let current = self.current_layer()?;
            current
                .parent_id
                .clone()
                .ok_or(NavigationError::NoParentLayer)?
        };

        self.push_history();
        self.current_layer_id = parent_id;
        self.stats.zoom_outs += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Zoom out to a specific level.
    pub fn zoom_out_to(&mut self, target_level: ZoomLevel) -> NavigationResult<&ContextLayer> {
        let current_level = self.current_level()?;

        if target_level.depth() >= current_level.depth() {
            return Err(NavigationError::InvalidNavigation(format!(
                "Cannot zoom out to {} from {}",
                target_level, current_level
            )));
        }

        // Keep zooming out until we reach target level
        while self.current_level()?.depth() > target_level.depth() {
            self.zoom_out()?;
        }

        self.current_layer()
    }

    /// Zoom out to root.
    pub fn zoom_to_root(&mut self) -> NavigationResult<&ContextLayer> {
        self.push_history();
        self.current_layer_id = self.context.root_id.clone();
        self.stats.zoom_outs += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    // =========================================================================
    // Sibling Navigation (Pan)
    // =========================================================================

    /// Move to a sibling layer by ID.
    pub fn pan_to(&mut self, sibling_id: &str) -> NavigationResult<&ContextLayer> {
        let is_sibling = {
            let current = self.current_layer()?;
            current.sibling_ids.contains(&sibling_id.to_string())
        };

        if !is_sibling {
            if !self.context.layers.contains_key(sibling_id) {
                return Err(NavigationError::LayerNotFound(sibling_id.to_string()));
            }
            return Err(NavigationError::InvalidNavigation(format!(
                "'{}' is not a sibling of current layer",
                sibling_id
            )));
        }

        self.push_history();
        self.current_layer_id = sibling_id.to_string();
        self.stats.sibling_moves += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Move to next sibling.
    pub fn pan_next(&mut self) -> NavigationResult<&ContextLayer> {
        let next_sibling = {
            let current = self.current_layer()?;
            let siblings = &current.sibling_ids;

            if siblings.is_empty() {
                return Err(NavigationError::NoSiblings);
            }

            // Find current position among siblings and get next
            // If not in siblings list, take first
            siblings.first().cloned()
        };

        if let Some(sibling_id) = next_sibling {
            self.push_history();
            self.current_layer_id = sibling_id;
            self.stats.sibling_moves += 1;
            self.stats.total_navigations += 1;
            self.current_layer()
        } else {
            Err(NavigationError::NoSiblings)
        }
    }

    /// Move to previous sibling.
    pub fn pan_prev(&mut self) -> NavigationResult<&ContextLayer> {
        let prev_sibling = {
            let current = self.current_layer()?;
            let siblings = &current.sibling_ids;

            if siblings.is_empty() {
                return Err(NavigationError::NoSiblings);
            }

            siblings.last().cloned()
        };

        if let Some(sibling_id) = prev_sibling {
            self.push_history();
            self.current_layer_id = sibling_id;
            self.stats.sibling_moves += 1;
            self.stats.total_navigations += 1;
            self.current_layer()
        } else {
            Err(NavigationError::NoSiblings)
        }
    }

    // =========================================================================
    // History Navigation
    // =========================================================================

    /// Navigate back to previous position.
    pub fn back(&mut self) -> NavigationResult<&ContextLayer> {
        let snapshot = self.history.pop_back().ok_or(NavigationError::NoHistory)?;

        // Push current to forward stack
        self.push_forward();

        self.current_layer_id = snapshot.layer_id;
        self.stats.back_navigations += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Navigate forward (redo after back).
    pub fn forward(&mut self) -> NavigationResult<&ContextLayer> {
        let snapshot = self
            .forward
            .pop_front()
            .ok_or(NavigationError::NoForwardHistory)?;

        // Push current to history
        self.push_history();

        self.current_layer_id = snapshot.layer_id;
        self.stats.forward_navigations += 1;
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Check if back navigation is available.
    pub fn can_go_back(&self) -> bool {
        !self.history.is_empty()
    }

    /// Check if forward navigation is available.
    pub fn can_go_forward(&self) -> bool {
        !self.forward.is_empty()
    }

    /// Get history length.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Clear navigation history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.forward.clear();
    }

    // =========================================================================
    // Query Methods
    // =========================================================================

    /// Get all children of current layer.
    pub fn children(&self) -> Vec<&ContextLayer> {
        self.current_layer()
            .map(|current| {
                current
                    .child_ids
                    .iter()
                    .filter_map(|id| self.context.layers.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all siblings of current layer.
    pub fn siblings(&self) -> Vec<&ContextLayer> {
        self.current_layer()
            .map(|current| {
                current
                    .sibling_ids
                    .iter()
                    .filter_map(|id| self.context.layers.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get parent of current layer.
    pub fn parent(&self) -> Option<&ContextLayer> {
        self.current_layer()
            .ok()
            .and_then(|current| current.parent_id.as_ref())
            .and_then(|parent_id| self.context.layers.get(parent_id))
    }

    /// Get the navigation path from root to current.
    pub fn path(&self) -> Vec<&ContextLayer> {
        let mut path = Vec::new();
        let mut current_id = Some(self.current_layer_id.clone());

        while let Some(id) = current_id {
            if let Some(layer) = self.context.layers.get(&id) {
                path.push(layer);
                current_id = layer.parent_id.clone();
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    /// Get path as zoom levels.
    pub fn path_levels(&self) -> Vec<ZoomLevel> {
        self.path().iter().map(|layer| layer.level).collect()
    }

    /// Get breadcrumb string for current position.
    pub fn breadcrumb(&self) -> String {
        self.path()
            .iter()
            .map(|layer| layer.name())
            .collect::<Vec<_>>()
            .join(" > ")
    }

    /// Check if at root level.
    pub fn is_at_root(&self) -> bool {
        self.current_layer_id == self.context.root_id
    }

    /// Check if current layer has children.
    pub fn has_children(&self) -> bool {
        self.current_layer()
            .map(|layer| !layer.child_ids.is_empty())
            .unwrap_or(false)
    }

    /// Check if current layer has siblings.
    pub fn has_siblings(&self) -> bool {
        self.current_layer()
            .map(|layer| !layer.sibling_ids.is_empty())
            .unwrap_or(false)
    }

    /// Count children at current level.
    pub fn child_count(&self) -> usize {
        self.current_layer()
            .map(|layer| layer.child_ids.len())
            .unwrap_or(0)
    }

    /// Count siblings at current level.
    pub fn sibling_count(&self) -> usize {
        self.current_layer()
            .map(|layer| layer.sibling_ids.len())
            .unwrap_or(0)
    }

    // =========================================================================
    // Search & Jump
    // =========================================================================

    /// Navigate directly to a layer by ID.
    pub fn jump_to(&mut self, layer_id: &str) -> NavigationResult<&ContextLayer> {
        if !self.context.layers.contains_key(layer_id) {
            return Err(NavigationError::LayerNotFound(layer_id.to_string()));
        }

        self.push_history();
        self.current_layer_id = layer_id.to_string();
        self.stats.total_navigations += 1;

        self.current_layer()
    }

    /// Find layers by name (partial match).
    pub fn find_by_name(&self, name: &str) -> Vec<&ContextLayer> {
        self.context
            .layers
            .values()
            .filter(|layer| layer.name().to_lowercase().contains(&name.to_lowercase()))
            .collect()
    }

    /// Find layers at a specific level.
    pub fn find_at_level(&self, level: ZoomLevel) -> Vec<&ContextLayer> {
        self.context
            .layers
            .values()
            .filter(|layer| layer.level == level)
            .collect()
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    fn push_history(&mut self) {
        if let Ok(current) = self.current_layer() {
            let snapshot = NavigationSnapshot {
                layer_id: self.current_layer_id.clone(),
                level: current.level,
                timestamp: std::time::Instant::now(),
            };

            self.history.push_back(snapshot);

            // Limit history size
            while self.history.len() > self.max_history {
                self.history.pop_front();
            }

            // Clear forward history on new navigation
            self.forward.clear();
        }
    }

    fn push_forward(&mut self) {
        if let Ok(current) = self.current_layer() {
            let snapshot = NavigationSnapshot {
                layer_id: self.current_layer_id.clone(),
                level: current.level,
                timestamp: std::time::Instant::now(),
            };

            self.forward.push_front(snapshot);
        }
    }
}

// =============================================================================
// Display Implementation
// =============================================================================

impl std::fmt::Display for FractalNavigator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(layer) = self.current_layer() {
            write!(
                f,
                "Navigator @ {} (level: {}, children: {}, siblings: {})",
                layer.name(),
                layer.level,
                layer.child_ids.len(),
                layer.sibling_ids.len()
            )
        } else {
            write!(f, "Navigator @ <unknown>")
        }
    }
}

// =============================================================================
// Tests (TDD)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::fractal::{
        ContextLayer, FractalContext, LayerContent, Range, SymbolKind, Visibility,
    };
    use std::path::PathBuf;

    // =========================================================================
    // Test Helpers
    // =========================================================================

    fn create_test_context() -> FractalContext {
        // Create file layer
        let file_layer = ContextLayer::new(
            "file_001",
            LayerContent::File {
                path: PathBuf::from("src/main.rs"),
                language: "rust".to_string(),
                size_bytes: 1024,
                line_count: 50,
                symbol_count: 3,
                imports: vec![],
            },
        );

        let mut ctx = FractalContext::new("ctx_001", file_layer);

        // Add symbol layers
        let sym1 = ContextLayer::new(
            "sym_001",
            LayerContent::Symbol {
                name: "main".to_string(),
                kind: SymbolKind::Function,
                signature: "fn main()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Public,
                range: Range::line_range(1, 10),
            },
        )
        .with_parent("file_001");
        ctx.add_layer(sym1);

        let sym2 = ContextLayer::new(
            "sym_002",
            LayerContent::Symbol {
                name: "helper".to_string(),
                kind: SymbolKind::Function,
                signature: "fn helper()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Private,
                range: Range::line_range(12, 20),
            },
        )
        .with_parent("file_001");
        ctx.add_layer(sym2);

        let sym3 = ContextLayer::new(
            "sym_003",
            LayerContent::Symbol {
                name: "process".to_string(),
                kind: SymbolKind::Function,
                signature: "fn process()".to_string(),
                return_type: None,
                parameters: vec![],
                documentation: None,
                visibility: Visibility::Public,
                range: Range::line_range(22, 30),
            },
        )
        .with_parent("file_001");
        ctx.add_layer(sym3);

        // Link children
        if let Some(file) = ctx.get_layer_mut("file_001") {
            file.add_child("sym_001");
            file.add_child("sym_002");
            file.add_child("sym_003");
        }

        // Link siblings
        if let Some(sym) = ctx.get_layer_mut("sym_001") {
            sym.add_sibling("sym_002");
            sym.add_sibling("sym_003");
        }
        if let Some(sym) = ctx.get_layer_mut("sym_002") {
            sym.add_sibling("sym_001");
            sym.add_sibling("sym_003");
        }
        if let Some(sym) = ctx.get_layer_mut("sym_003") {
            sym.add_sibling("sym_001");
            sym.add_sibling("sym_002");
        }

        ctx
    }

    // =========================================================================
    // Basic Navigation Tests
    // =========================================================================

    #[test]
    fn test_navigator_new() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        assert_eq!(nav.current_id(), "file_001");
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::File);
    }

    #[test]
    fn test_navigator_new_at() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new_at(ctx, "sym_001").unwrap();

        assert_eq!(nav.current_id(), "sym_001");
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::Symbol);
    }

    #[test]
    fn test_navigator_new_at_invalid() {
        let ctx = create_test_context();
        let result = FractalNavigator::new_at(ctx, "nonexistent");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NavigationError::LayerNotFound(_)));
    }

    #[test]
    fn test_current_layer() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        let layer = nav.current_layer().unwrap();
        assert_eq!(layer.name(), "main.rs");
        assert_eq!(layer.level, ZoomLevel::File);
    }

    // =========================================================================
    // Zoom Navigation Tests
    // =========================================================================

    #[test]
    fn test_zoom_in() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let layer = nav.zoom_in("sym_001").unwrap();
        assert_eq!(layer.name(), "main");
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::Symbol);
    }

    #[test]
    fn test_zoom_in_invalid_child() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let result = nav.zoom_in("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_zoom_in_first() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let layer = nav.zoom_in_first().unwrap();
        assert_eq!(layer.name(), "main");
    }

    #[test]
    fn test_zoom_in_nth() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let layer = nav.zoom_in_nth(1).unwrap();
        assert_eq!(layer.name(), "helper");
    }

    #[test]
    fn test_zoom_out() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::Symbol);

        let layer = nav.zoom_out().unwrap();
        assert_eq!(layer.level, ZoomLevel::File);
    }

    #[test]
    fn test_zoom_out_at_root() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let result = nav.zoom_out();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NavigationError::NoParentLayer));
    }

    #[test]
    fn test_zoom_to_root() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        assert!(!nav.is_at_root());

        nav.zoom_to_root().unwrap();
        assert!(nav.is_at_root());
    }

    // =========================================================================
    // Sibling Navigation Tests
    // =========================================================================

    #[test]
    fn test_pan_to() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in("sym_001").unwrap();
        nav.pan_to("sym_002").unwrap();

        assert_eq!(nav.current_layer().unwrap().name(), "helper");
    }

    #[test]
    fn test_pan_next() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in("sym_001").unwrap();
        nav.pan_next().unwrap();

        // Should move to first sibling
        assert!(nav.current_layer().unwrap().name() != "main");
    }

    #[test]
    fn test_pan_at_file_level_no_siblings() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        // File has no siblings
        let result = nav.pan_next();
        assert!(result.is_err());
    }

    // =========================================================================
    // History Navigation Tests
    // =========================================================================

    #[test]
    fn test_back_navigation() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        assert!(!nav.can_go_back());

        nav.zoom_in_first().unwrap();
        assert!(nav.can_go_back());

        nav.back().unwrap();
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::File);
    }

    #[test]
    fn test_forward_navigation() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        nav.back().unwrap();

        assert!(nav.can_go_forward());

        nav.forward().unwrap();
        assert_eq!(nav.current_level().unwrap(), ZoomLevel::Symbol);
    }

    #[test]
    fn test_back_no_history() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let result = nav.back();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NavigationError::NoHistory));
    }

    #[test]
    fn test_forward_no_history() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let result = nav.forward();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NavigationError::NoForwardHistory));
    }

    #[test]
    fn test_history_length() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        assert_eq!(nav.history_len(), 0);

        nav.zoom_in_first().unwrap();
        assert_eq!(nav.history_len(), 1);

        nav.zoom_out().unwrap();
        assert_eq!(nav.history_len(), 2);
    }

    #[test]
    fn test_clear_history() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        nav.zoom_out().unwrap();
        assert!(nav.history_len() > 0);

        nav.clear_history();
        assert_eq!(nav.history_len(), 0);
        assert!(!nav.can_go_back());
    }

    // =========================================================================
    // Query Method Tests
    // =========================================================================

    #[test]
    fn test_children() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        let children = nav.children();
        assert_eq!(children.len(), 3);
    }

    #[test]
    fn test_siblings() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();

        let siblings = nav.siblings();
        assert_eq!(siblings.len(), 2); // Two other functions
    }

    #[test]
    fn test_parent() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        assert!(nav.parent().is_none()); // At root

        nav.zoom_in_first().unwrap();
        let parent = nav.parent().unwrap();
        assert_eq!(parent.level, ZoomLevel::File);
    }

    #[test]
    fn test_path() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();

        let path = nav.path();
        assert_eq!(path.len(), 2); // file -> symbol
        assert_eq!(path[0].level, ZoomLevel::File);
        assert_eq!(path[1].level, ZoomLevel::Symbol);
    }

    #[test]
    fn test_path_levels() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();

        let levels = nav.path_levels();
        assert_eq!(levels, vec![ZoomLevel::File, ZoomLevel::Symbol]);
    }

    #[test]
    fn test_breadcrumb() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();

        let breadcrumb = nav.breadcrumb();
        assert!(breadcrumb.contains("main.rs"));
        assert!(breadcrumb.contains("main"));
        assert!(breadcrumb.contains(" > "));
    }

    #[test]
    fn test_is_at_root() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        assert!(nav.is_at_root());

        nav.zoom_in_first().unwrap();
        assert!(!nav.is_at_root());
    }

    #[test]
    fn test_has_children() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        assert!(nav.has_children()); // File has children

        nav.zoom_in_first().unwrap();
        assert!(!nav.has_children()); // Symbol has no children
    }

    #[test]
    fn test_child_count() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        assert_eq!(nav.child_count(), 3);
    }

    #[test]
    fn test_sibling_count() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        assert_eq!(nav.sibling_count(), 2);
    }

    // =========================================================================
    // Search & Jump Tests
    // =========================================================================

    #[test]
    fn test_jump_to() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.jump_to("sym_002").unwrap();
        assert_eq!(nav.current_layer().unwrap().name(), "helper");
    }

    #[test]
    fn test_jump_to_invalid() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        let result = nav.jump_to("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_by_name() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        let results = nav.find_by_name("main");
        assert!(!results.is_empty());
        assert!(results.iter().any(|l| l.name().contains("main")));
    }

    #[test]
    fn test_find_at_level() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        let symbols = nav.find_at_level(ZoomLevel::Symbol);
        assert_eq!(symbols.len(), 3);

        let files = nav.find_at_level(ZoomLevel::File);
        assert_eq!(files.len(), 1);
    }

    // =========================================================================
    // Statistics Tests
    // =========================================================================

    #[test]
    fn test_navigation_stats() {
        let ctx = create_test_context();
        let mut nav = FractalNavigator::new(ctx);

        nav.zoom_in_first().unwrap();
        nav.zoom_out().unwrap();
        nav.back().unwrap();

        let stats = nav.stats();
        assert_eq!(stats.zoom_ins, 1);
        assert_eq!(stats.zoom_outs, 1);
        assert_eq!(stats.back_navigations, 1);
        assert!(stats.total_navigations >= 3);
    }

    // =========================================================================
    // Display Tests
    // =========================================================================

    #[test]
    fn test_navigator_display() {
        let ctx = create_test_context();
        let nav = FractalNavigator::new(ctx);

        let display = format!("{}", nav);
        assert!(display.contains("Navigator"));
        assert!(display.contains("main.rs"));
    }
}
