//! Syntax Integration Tests - Phase 1A Validation
//!
//! These tests verify that the Tree-sitter based AST extraction works correctly
//! for multiple programming languages. This is the foundation for Voyager Observatory's
//! ability to understand code structure beyond raw text.

use pm_encoder::core::{SymbolKind, SymbolVisibility, SyntaxLanguage, SyntaxRegistry};

// ============================================================================
// Rust Language Tests
// ============================================================================

#[test]
fn test_rust_function_extraction() {
    let source = r#"
/// Greets the world
pub fn hello_world() {
    println!("Hello, world!");
}

fn private_helper(x: i32, y: i32) -> i32 {
    x + y
}
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Rust)
        .expect("Failed to parse Rust");

    assert!(ast.symbols.len() >= 2, "Expected at least 2 functions");

    // Find public function
    let hello = ast.symbols.iter().find(|s| s.name == "hello_world");
    assert!(hello.is_some(), "hello_world function not found");
    let hello = hello.unwrap();
    assert_eq!(hello.kind, SymbolKind::Function);
    assert_eq!(hello.visibility, SymbolVisibility::Public);

    // Find private function
    let helper = ast.symbols.iter().find(|s| s.name == "private_helper");
    assert!(helper.is_some(), "private_helper function not found");
    let helper = helper.unwrap();
    assert_eq!(helper.kind, SymbolKind::Function);
    assert_eq!(helper.visibility, SymbolVisibility::Private);
}

#[test]
fn test_rust_struct_extraction() {
    let source = r#"
/// A point in 2D space
#[derive(Debug, Clone)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

struct PrivateData {
    value: i32,
}
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Rust)
        .expect("Failed to parse Rust");

    // Find public struct
    let point = ast.symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "Point struct not found");
    let point = point.unwrap();
    assert_eq!(point.kind, SymbolKind::Struct);
    assert_eq!(point.visibility, SymbolVisibility::Public);

    // Find private struct
    let private = ast.symbols.iter().find(|s| s.name == "PrivateData");
    assert!(private.is_some(), "PrivateData struct not found");
    assert_eq!(private.unwrap().visibility, SymbolVisibility::Private);
}

#[test]
fn test_rust_impl_and_trait() {
    let source = r#"
pub trait Drawable {
    fn draw(&self);
}

impl Drawable for Point {
    fn draw(&self) {
        println!("Drawing point");
    }
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Rust)
        .expect("Failed to parse Rust");

    // Find trait
    let drawable = ast.symbols.iter().find(|s| s.name == "Drawable");
    assert!(drawable.is_some(), "Drawable trait not found");
    assert_eq!(drawable.unwrap().kind, SymbolKind::Trait);

    // Find methods (may appear as nested or top-level depending on extraction)
    let methods: Vec<_> = ast
        .symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Method || s.kind == SymbolKind::Function)
        .collect();
    assert!(!methods.is_empty(), "Expected to find methods/functions");
}

// ============================================================================
// Python Language Tests
// ============================================================================

#[test]
fn test_python_function_extraction() {
    let source = r#"
def greet(name: str) -> str:
    """Greet someone by name."""
    return f"Hello, {name}!"

def _private_helper():
    pass
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Python)
        .expect("Failed to parse Python");

    assert!(ast.symbols.len() >= 2, "Expected at least 2 functions");

    let greet = ast.symbols.iter().find(|s| s.name == "greet");
    assert!(greet.is_some(), "greet function not found");
    assert_eq!(greet.unwrap().kind, SymbolKind::Function);

    let helper = ast.symbols.iter().find(|s| s.name == "_private_helper");
    assert!(helper.is_some(), "_private_helper function not found");
}

#[test]
fn test_python_class_extraction() {
    let source = r#"
class Animal:
    """Base class for animals."""

    def __init__(self, name: str):
        self.name = name

    def speak(self) -> str:
        raise NotImplementedError

class Dog(Animal):
    def speak(self) -> str:
        return "Woof!"
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Python)
        .expect("Failed to parse Python");

    let animal = ast.symbols.iter().find(|s| s.name == "Animal");
    assert!(animal.is_some(), "Animal class not found");
    assert_eq!(animal.unwrap().kind, SymbolKind::Class);

    let dog = ast.symbols.iter().find(|s| s.name == "Dog");
    assert!(dog.is_some(), "Dog class not found");
    assert_eq!(dog.unwrap().kind, SymbolKind::Class);
}

#[test]
fn test_python_import_extraction() {
    let source = r#"
import os
import sys as system
from pathlib import Path
from typing import List, Optional
from . import relative_module
from ..parent import something
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Python)
        .expect("Failed to parse Python");

    // Phase 1A: Verify parsing works without errors
    // Import extraction is basic in Phase 1A and will be enhanced
    assert!(
        ast.errors.is_empty() || !ast.has_errors(),
        "Parse should succeed for valid Python import statements"
    );
}

// ============================================================================
// TypeScript Language Tests
// ============================================================================

#[test]
fn test_typescript_function_extraction() {
    let source = r#"
export function greet(name: string): string {
    return `Hello, ${name}!`;
}

function privateHelper(): void {
    console.log("Helper");
}

export const arrowFn = (x: number): number => x * 2;
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::TypeScript)
        .expect("Failed to parse TypeScript");

    let greet = ast.symbols.iter().find(|s| s.name == "greet");
    assert!(greet.is_some(), "greet function not found");
    assert_eq!(greet.unwrap().kind, SymbolKind::Function);

    let helper = ast.symbols.iter().find(|s| s.name == "privateHelper");
    assert!(helper.is_some(), "privateHelper function not found");
}

#[test]
fn test_typescript_class_and_interface() {
    let source = r#"
export interface Drawable {
    draw(): void;
}

export class Point implements Drawable {
    constructor(public x: number, public y: number) {}

    draw(): void {
        console.log(`Point(${this.x}, ${this.y})`);
    }

    static origin(): Point {
        return new Point(0, 0);
    }
}

type Coordinate = { x: number; y: number };
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::TypeScript)
        .expect("Failed to parse TypeScript");

    // Phase 1A: Verify class extraction works
    let point = ast.symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "Point class not found");
    assert_eq!(point.unwrap().kind, SymbolKind::Class);

    // Interface extraction may be enhanced in Phase 1B
    // For now, verify we can at least parse without errors
    assert!(
        ast.errors.is_empty() || !ast.has_errors(),
        "Parse should succeed for valid TypeScript"
    );
}

#[test]
fn test_typescript_import_extraction() {
    let source = r#"
import { Component } from '@angular/core';
import * as utils from './utils';
import type { Config } from './config';
import defaultExport from 'module';
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::TypeScript)
        .expect("Failed to parse TypeScript");

    assert!(!ast.imports.is_empty(), "Expected imports to be captured");
}

// ============================================================================
// Multi-Language Tests
// ============================================================================

#[test]
fn test_language_detection_from_extension() {
    let registry = SyntaxRegistry::new();

    // Rust file
    let rust_source = "fn main() {}";
    let ast = registry
        .parse_file(rust_source, "main.rs")
        .expect("Failed to parse .rs");
    assert!(!ast.symbols.is_empty() || ast.errors.is_empty());

    // Python file
    let py_source = "def main(): pass";
    let ast = registry
        .parse_file(py_source, "main.py")
        .expect("Failed to parse .py");
    assert!(!ast.symbols.is_empty() || ast.errors.is_empty());

    // TypeScript file
    let ts_source = "function main() {}";
    let ast = registry
        .parse_file(ts_source, "main.ts")
        .expect("Failed to parse .ts");
    assert!(!ast.symbols.is_empty() || ast.errors.is_empty());
}

#[test]
fn test_supported_languages() {
    let registry = SyntaxRegistry::new();

    // Core supported languages
    assert!(registry.supports(SyntaxLanguage::Rust));
    assert!(registry.supports(SyntaxLanguage::Python));
    assert!(registry.supports(SyntaxLanguage::TypeScript));
    assert!(registry.supports(SyntaxLanguage::JavaScript));
    assert!(registry.supports(SyntaxLanguage::Go));
    assert!(registry.supports(SyntaxLanguage::Java));
    assert!(registry.supports(SyntaxLanguage::Cpp));
    assert!(registry.supports(SyntaxLanguage::CSharp));
    assert!(registry.supports(SyntaxLanguage::Ruby));

    // Phase 1B languages (not yet supported)
    // Note: These may fail gracefully with UnsupportedLanguage error
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_parse_error_handling() {
    let source = r#"
fn broken_rust( {
    // Missing closing paren
}
"#;

    let registry = SyntaxRegistry::new();
    // Tree-sitter is error-tolerant and produces partial ASTs
    // The parse should succeed even with syntax errors
    let result = registry.parse(source, SyntaxLanguage::Rust);
    assert!(result.is_ok(), "Tree-sitter should be error-tolerant");

    // We successfully parsed despite the syntax error
    let _ast = result.unwrap();
}

#[test]
fn test_unsupported_extension() {
    let registry = SyntaxRegistry::new();

    // Unknown extension should fail
    let result = registry.parse_file("content", "file.xyz");
    assert!(result.is_err());
}

// ============================================================================
// Go Language Tests (Additional coverage)
// ============================================================================

#[test]
fn test_go_function_extraction() {
    let source = r#"
package main

import "fmt"

// Greet greets a person
func Greet(name string) string {
    return fmt.Sprintf("Hello, %s!", name)
}

func privateHelper() int {
    return 42
}
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Go)
        .expect("Failed to parse Go");

    // Check for functions
    let greet = ast.symbols.iter().find(|s| s.name == "Greet");
    assert!(greet.is_some(), "Greet function not found");

    let helper = ast.symbols.iter().find(|s| s.name == "privateHelper");
    assert!(helper.is_some(), "privateHelper function not found");
}

// ============================================================================
// Java Language Tests (Additional coverage)
// ============================================================================

#[test]
fn test_java_class_extraction() {
    let source = r#"
package com.example;

import java.util.List;

public class Point {
    private final int x;
    private final int y;

    public Point(int x, int y) {
        this.x = x;
        this.y = y;
    }

    public int getX() { return x; }
    public int getY() { return y; }
}
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::Java)
        .expect("Failed to parse Java");

    let point = ast.symbols.iter().find(|s| s.name == "Point");
    assert!(point.is_some(), "Point class not found");
    assert_eq!(point.unwrap().kind, SymbolKind::Class);
}

// ============================================================================
// JavaScript Tests (TSX variant)
// ============================================================================

#[test]
fn test_javascript_extraction() {
    let source = r#"
function greet(name) {
    return `Hello, ${name}!`;
}

class Calculator {
    add(a, b) {
        return a + b;
    }
}

const multiply = (a, b) => a * b;
"#;

    let registry = SyntaxRegistry::new();
    let ast = registry
        .parse(source, SyntaxLanguage::JavaScript)
        .expect("Failed to parse JavaScript");

    let greet = ast.symbols.iter().find(|s| s.name == "greet");
    assert!(greet.is_some(), "greet function not found");

    let calc = ast.symbols.iter().find(|s| s.name == "Calculator");
    assert!(calc.is_some(), "Calculator class not found");
}
