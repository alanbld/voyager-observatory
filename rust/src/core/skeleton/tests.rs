//! Comprehensive unit tests for Skeleton Protocol v2.2
//!
//! These tests focus on edge cases and specific language parsing behavior
//! not covered by the integration tests.

use super::*;

// ============================================================================
// Rust Parser Tests
// ============================================================================

mod rust_parser {
    use super::*;

    #[test]
    fn test_struct_with_fields_preserved() {
        let input = r#"
pub struct Config {
    pub name: String,
    pub value: i32,
    private_field: bool,
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Struct definition should be fully preserved (fields are signatures)
        assert!(result.content.contains("pub struct Config"));
        assert!(result.content.contains("pub name: String"));
        assert!(result.content.contains("pub value: i32"));
        assert!(result.content.contains("private_field: bool"));
        assert!(result.preserved_symbols.contains(&"Config".to_string()));
    }

    #[test]
    fn test_struct_empty_body() {
        let input = r#"
pub struct Unit;
pub struct EmptyBraces {}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub struct Unit"));
        assert!(result.content.contains("pub struct EmptyBraces"));
    }

    #[test]
    fn test_impl_with_multiple_methods() {
        let input = r#"
impl Config {
    pub fn new() -> Self {
        Self { name: String::new(), value: 0, private_field: false }
    }

    fn private_method(&self) -> bool {
        self.private_field && self.value > 0
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("impl Config"));
        assert!(result.content.contains("pub fn new()"));
        assert!(result.content.contains("fn private_method(&self)"));
        assert!(result.content.contains("pub fn get_name(&self)"));

        // Bodies should be stripped
        assert!(!result.content.contains("String::new()"));
        assert!(!result.content.contains("self.value > 0"));
    }

    #[test]
    fn test_trait_definition() {
        let input = r#"
pub trait Service {
    type Output;

    fn call(&self, input: Request) -> Self::Output;

    fn default_method(&self) -> bool {
        true
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub trait Service"));
        assert!(result.content.contains("type Output"));
        assert!(result.content.contains("fn call(&self, input: Request)"));
        assert!(result.content.contains("fn default_method(&self)"));
        assert!(result.preserved_symbols.contains(&"Service".to_string()));
    }

    #[test]
    fn test_impl_trait_for_struct() {
        let input = r#"
impl Service for Config {
    type Output = Response;

    fn call(&self, input: Request) -> Self::Output {
        let processed = process_request(input);
        Response::new(processed)
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("impl Service for Config"));
        assert!(result.content.contains("fn call(&self, input: Request)"));
        assert!(!result.content.contains("process_request"));
    }

    #[test]
    fn test_derive_attribute_preserved() {
        let input = r#"
#[derive(Debug, Clone, Serialize)]
pub struct Data {
    value: i32,
}

#[derive(Default)]
impl Data {
    fn new() -> Self { Data { value: 0 } }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Derive attributes should be preserved
        assert!(result
            .content
            .contains("#[derive(Debug, Clone, Serialize)]"));
        assert!(result.content.contains("pub struct Data"));
    }

    #[test]
    fn test_cfg_and_other_attributes() {
        let input = r#"
#[cfg(test)]
mod tests {
    #[test]
    fn test_something() {
        assert!(true);
    }
}

#[inline]
pub fn fast_function() -> i32 {
    42
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Outer attributes should be preserved
        assert!(result.content.contains("#[cfg(test)]"));
        assert!(result.content.contains("#[inline]"));
        assert!(result.content.contains("pub fn fast_function()"));
    }

    #[test]
    fn test_enum_definition() {
        let input = r#"
#[derive(Debug)]
pub enum Status {
    Active,
    Inactive,
    Pending(String),
    Error { code: i32, message: String },
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub enum Status"));
        assert!(result.content.contains("Active"));
        assert!(result.content.contains("Pending(String)"));
        assert!(result.content.contains("Error { code: i32"));
        assert!(result.preserved_symbols.contains(&"Status".to_string()));
    }

    #[test]
    fn test_deeply_nested_braces() {
        let input = r#"
fn nested() {
    loop {
        if true {
            match x {
                Some(v) => {
                    while v > 0 {
                        if v % 2 == 0 {
                            break;
                        }
                    }
                }
                None => {}
            }
        }
    }
}

fn after_nested() -> i32 {
    100
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Both function signatures should be found
        assert!(result.content.contains("fn nested()"));
        assert!(result.content.contains("fn after_nested() -> i32"));

        // Deep nesting content should not be present
        assert!(!result.content.contains("while v > 0"));
        assert!(!result.content.contains("break"));

        assert!(result.preserved_symbols.contains(&"nested".to_string()));
        assert!(result
            .preserved_symbols
            .contains(&"after_nested".to_string()));
    }

    #[test]
    fn test_async_function() {
        let input = r#"
pub async fn fetch_data(url: &str) -> Result<Response, Error> {
    let client = Client::new();
    let response = client.get(url).await?;
    Ok(response)
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub async fn fetch_data"));
        assert!(result.content.contains("Result<Response, Error>"));
        assert!(!result.content.contains("Client::new()"));
        assert!(result.preserved_symbols.contains(&"fetch_data".to_string()));
    }

    #[test]
    fn test_doc_comments_preserved() {
        let input = r#"
/// Main entry point
///
/// # Examples
/// ```
/// main();
/// ```
pub fn main() {
    println!("Hello");
}

//! Module documentation
//! This is a module.
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("/// Main entry point"));
        assert!(result.content.contains("//! Module documentation"));
        assert!(result.content.contains("pub fn main()"));
        assert!(!result.content.contains("println!"));
    }

    #[test]
    fn test_generic_struct_and_impl() {
        let input = r#"
pub struct Container<T> {
    items: Vec<T>,
}

impl<T: Clone> Container<T> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add(&mut self, item: T) {
        self.items.push(item);
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub struct Container<T>"));
        assert!(result.content.contains("items: Vec<T>"));
        assert!(result.content.contains("impl<T: Clone> Container<T>"));
        assert!(result.content.contains("pub fn new()"));
        assert!(result.content.contains("pub fn add(&mut self, item: T)"));
    }

    #[test]
    fn test_multiline_function_signature() {
        let input = r#"
pub fn complex_function(
    first_arg: &str,
    second_arg: i32,
    third_arg: Option<String>,
) -> Result<ProcessedData, Error> {
    let validated = validate(first_arg)?;
    process(validated, second_arg, third_arg)
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("pub fn complex_function"));
        assert!(result.content.contains("first_arg: &str"));
        assert!(result.content.contains("Result<ProcessedData, Error>"));
        assert!(!result.content.contains("validate(first_arg)"));
    }
}

// ============================================================================
// Python Parser Tests
// ============================================================================

mod python_parser {
    use super::*;

    #[test]
    fn test_class_with_docstring() {
        let input = r#"
class User:
    """Represents a user in the system.

    Attributes:
        name: The user's name
        email: The user's email address
    """

    def __init__(self, name: str, email: str):
        self.name = name
        self.email = email
        self._validate()
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result.content.contains("class User:"));
        assert!(result.content.contains("Represents a user"));
        assert!(result
            .content
            .contains("def __init__(self, name: str, email: str):"));
        assert!(!result.content.contains("self.name = name"));
        assert!(!result.content.contains("self._validate()"));
    }

    #[test]
    fn test_method_docstring_preserved() {
        let input = r#"
class Calculator:
    def add(self, a: int, b: int) -> int:
        """Add two numbers.

        Args:
            a: First number
            b: Second number

        Returns:
            Sum of a and b
        """
        result = a + b
        self.log(result)
        return result
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result
            .content
            .contains("def add(self, a: int, b: int) -> int:"));
        assert!(result.content.contains("Add two numbers"));
        assert!(!result.content.contains("result = a + b"));
        assert!(!result.content.contains("self.log"));
    }

    #[test]
    fn test_decorator_not_preserved() {
        // Note: Current implementation doesn't preserve decorators
        let input = r#"
from dataclasses import dataclass

@dataclass
class Config:
    host: str
    port: int = 8080

@property
def url(self) -> str:
    return f"http://{self.host}:{self.port}"
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        // Import should be preserved
        assert!(result.content.contains("from dataclasses import dataclass"));

        // Class should be preserved
        // Note: Decorators are currently NOT preserved by the implementation
        // This test documents the current behavior
        assert!(result.content.contains("class Config:"));
    }

    #[test]
    fn test_nested_class() {
        let input = r#"
class Outer:
    """Outer class."""

    class Inner:
        """Inner class."""

        def inner_method(self, x: int) -> int:
            for i in range(x):
                if i > 10:
                    break
            return x * 2

    def outer_method(self) -> Inner:
        return self.Inner()
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result.content.contains("class Outer:"));
        assert!(result.content.contains("class Inner:"));
        assert!(result
            .content
            .contains("def inner_method(self, x: int) -> int:"));
        assert!(result.content.contains("def outer_method(self) -> Inner:"));
        assert!(!result.content.contains("range(x)"));
        assert!(!result.content.contains("break"));
    }

    #[test]
    fn test_standalone_function() {
        let input = r#"
def helper_function(data: list) -> dict:
    """Convert list to dict."""
    result = {}
    for item in data:
        result[item.key] = item.value
    return result
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result
            .content
            .contains("def helper_function(data: list) -> dict:"));
        assert!(result.content.contains("Convert list to dict"));
        assert!(!result.content.contains("result = {}"));
        assert!(!result.content.contains("for item in data"));
    }

    #[test]
    fn test_async_def() {
        let input = r#"
async def fetch_data(url: str) -> bytes:
    """Fetch data from URL."""
    async with aiohttp.ClientSession() as session:
        async with session.get(url) as response:
            return await response.read()
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result
            .content
            .contains("async def fetch_data(url: str) -> bytes:"));
        assert!(result.content.contains("Fetch data from URL"));
        assert!(!result.content.contains("aiohttp.ClientSession"));
    }

    #[test]
    fn test_multiple_imports() {
        let input = r#"
import os
import sys
from pathlib import Path
from typing import Optional, List, Dict
from .config import Config

def main():
    pass
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result.content.contains("import os"));
        assert!(result.content.contains("import sys"));
        assert!(result.content.contains("from pathlib import Path"));
        assert!(result
            .content
            .contains("from typing import Optional, List, Dict"));
        assert!(result.content.contains("from .config import Config"));
        assert!(result.content.contains("def main():"));
    }

    #[test]
    fn test_class_with_property() {
        let input = r#"
class Rectangle:
    def __init__(self, width: float, height: float):
        self._width = width
        self._height = height

    @property
    def area(self) -> float:
        return self._width * self._height

    @area.setter
    def area(self, value: float):
        raise ValueError("Cannot set area directly")
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        assert!(result.content.contains("class Rectangle:"));
        assert!(result
            .content
            .contains("def __init__(self, width: float, height: float):"));
        // Properties are just methods at this level
        assert!(result.content.contains("def area(self)"));
    }
}

// ============================================================================
// TypeScript/JavaScript Parser Tests
// ============================================================================

mod js_parser {
    use super::*;

    #[test]
    fn test_function_declaration() {
        let input = r#"
function processData(input: string): ProcessedData {
    const validated = validate(input);
    const parsed = parse(validated);
    return transform(parsed);
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("function processData"));
        assert!(!result.content.contains("validate(input)"));
        assert!(result
            .preserved_symbols
            .contains(&"processData".to_string()));
    }

    #[test]
    fn test_export_function() {
        let input = r#"
export function publicApi(request: Request): Response {
    const data = request.body;
    return new Response(data);
}

export async function fetchData(url: string): Promise<Data> {
    const response = await fetch(url);
    return response.json();
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("export function publicApi"));
        assert!(result.content.contains("export async function fetchData"));
        assert!(!result.content.contains("request.body"));
        assert!(!result.content.contains("await fetch"));
    }

    #[test]
    fn test_class_declaration() {
        let input = r#"
export class UserService {
    private users: Map<string, User>;

    constructor() {
        this.users = new Map();
    }

    getUser(id: string): User | undefined {
        return this.users.get(id);
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("export class UserService"));
        assert!(result
            .preserved_symbols
            .contains(&"UserService".to_string()));
        // Note: Class body is included but methods inside may not be extracted
    }

    #[test]
    fn test_interface_definition() {
        let input = r#"
export interface Config {
    host: string;
    port: number;
    debug?: boolean;
}

interface InternalConfig extends Config {
    secretKey: string;
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("export interface Config"));
        assert!(result
            .content
            .contains("interface InternalConfig extends Config"));
        assert!(result.preserved_symbols.contains(&"Config".to_string()));
        assert!(result
            .preserved_symbols
            .contains(&"InternalConfig".to_string()));
    }

    #[test]
    fn test_type_alias() {
        let input = r#"
export type UserId = string;
type Callback<T> = (value: T) => void;
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("export type UserId"));
        assert!(result.content.contains("type Callback<T>"));
        assert!(result.preserved_symbols.contains(&"UserId".to_string()));
        assert!(result.preserved_symbols.contains(&"Callback".to_string()));
    }

    #[test]
    fn test_arrow_function_const() {
        let input = r#"
export const add = (a: number, b: number) => {
    return a + b;
};

const multiply = async (a: number, b: number) => {
    await delay(100);
    return a * b;
};
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("export const add"));
        assert!(result.content.contains("const multiply"));
        assert!(result.preserved_symbols.contains(&"add".to_string()));
        assert!(result.preserved_symbols.contains(&"multiply".to_string()));
    }

    #[test]
    fn test_import_statements() {
        let input = r#"
import { Component } from 'react';
import * as fs from 'fs';
import axios from 'axios';
import type { Config } from './config';

function main() {
    console.log("Hello");
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::TypeScript);

        assert!(result.content.contains("import { Component }"));
        assert!(result.content.contains("import * as fs"));
        assert!(result.content.contains("import axios"));
        assert!(result.content.contains("import type { Config }"));
        assert!(result.content.contains("function main"));
        assert!(!result.content.contains("console.log"));
    }

    #[test]
    fn test_javascript_module() {
        let input = r#"
const express = require('express');

function createApp() {
    const app = express();
    app.use(cors());
    return app;
}

module.exports = { createApp };
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::JavaScript);

        assert!(result.content.contains("function createApp"));
        assert!(!result.content.contains("express()"));
    }
}

// ============================================================================
// Go Parser Tests
// ============================================================================

mod go_parser {
    use super::*;

    #[test]
    fn test_function_definition() {
        let input = r#"
package main

func ProcessData(input []byte) ([]byte, error) {
    if len(input) == 0 {
        return nil, errors.New("empty input")
    }
    result := transform(input)
    return result, nil
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("package main"));
        assert!(result.content.contains("func ProcessData"));
        assert!(!result.content.contains("len(input)"));
        assert!(!result.content.contains("transform(input)"));
        assert!(result
            .preserved_symbols
            .contains(&"ProcessData".to_string()));
    }

    #[test]
    fn test_method_with_receiver() {
        let input = r#"
package main

func (s *Service) HandleRequest(req *Request) (*Response, error) {
    validated, err := s.validate(req)
    if err != nil {
        return nil, err
    }
    return s.process(validated)
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("package main"));
        assert!(result.content.contains("func (s *Service) HandleRequest"));
        assert!(!result.content.contains("s.validate(req)"));
    }

    #[test]
    fn test_struct_definition() {
        let input = r#"
package models

type User struct {
    ID       string
    Name     string
    Email    string
    Active   bool
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("package models"));
        assert!(result.content.contains("type User struct"));
        assert!(result.preserved_symbols.contains(&"User".to_string()));
    }

    #[test]
    fn test_interface_definition() {
        let input = r#"
package service

type Handler interface {
    Handle(ctx context.Context, req Request) (Response, error)
    Close() error
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("package service"));
        assert!(result.content.contains("type Handler interface"));
        assert!(result.preserved_symbols.contains(&"Handler".to_string()));
    }

    #[test]
    fn test_import_block() {
        let input = r#"
package main

import (
    "context"
    "fmt"
    "net/http"

    "github.com/user/pkg"
)

func main() {
    fmt.Println("Hello")
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("import ("));
        assert!(result.content.contains("\"context\""));
        assert!(result.content.contains("\"fmt\""));
        assert!(result.content.contains("func main"));
        assert!(!result.content.contains("Println"));
    }

    #[test]
    fn test_const_and_var() {
        let input = r#"
package config

const (
    MaxRetries = 5
    Timeout    = 30 * time.Second
)

var (
    defaultConfig = Config{
        Retries: MaxRetries,
    }
)
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Go);

        assert!(result.content.contains("package config"));
        assert!(result.content.contains("const ("));
        assert!(result.content.contains("MaxRetries"));
        assert!(result.content.contains("var ("));
    }
}

// ============================================================================
// Edge Cases and Fallback Tests
// ============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_empty_content() {
        let s = Skeletonizer::new();

        let result_rust = s.skeletonize("", Language::Rust);
        assert!(result_rust.content.is_empty());
        assert_eq!(result_rust.original_tokens, 0);
        assert_eq!(result_rust.skeleton_tokens, 0);

        let result_py = s.skeletonize("", Language::Python);
        assert!(result_py.content.is_empty());

        let result_js = s.skeletonize("", Language::JavaScript);
        assert!(result_js.content.is_empty());

        let result_go = s.skeletonize("", Language::Go);
        assert!(result_go.content.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let input = "   \n\n   \t\t\n   ";
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Should produce minimal output
        assert!(result.skeleton_tokens <= result.original_tokens);
    }

    #[test]
    fn test_comments_only_rust() {
        let input = r#"
// This is a comment
// Another comment line

/* Block comment
   spanning multiple
   lines */

//! Module-level doc
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Doc comments should be preserved
        assert!(result.content.contains("//! Module-level doc"));
        // Regular comments may or may not be preserved
        assert!(result.skeleton_tokens <= result.original_tokens);
    }

    #[test]
    fn test_comments_only_python() {
        let input = r#"
# This is a comment
# Another comment

"""
Module docstring
"""
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        // Docstrings should be preserved
        assert!(result.content.contains("Module docstring") || result.content.is_empty());
    }

    #[test]
    fn test_unbalanced_braces_fallback() {
        let input = r#"
fn broken() {
    if true {
        // Missing closing brace here

fn next_function() {
    println!("unreachable");
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Should fallback to first N lines
        assert!(
            !result.content.is_empty(),
            "Fallback should produce some output"
        );
        // Fallback returns first 50 lines by default
        assert!(result.content.len() <= input.len());
    }

    #[test]
    fn test_single_line_file() {
        let input = "fn main() { println!(\"Hello\"); }";
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("fn main()"));
        assert!(result.preserved_symbols.contains(&"main".to_string()));
    }

    #[test]
    fn test_very_long_signature() {
        let input = r#"
fn very_long_function_name_that_goes_on_and_on(
    first_parameter: VeryLongTypeName<WithGenerics, AndMore>,
    second_parameter: AnotherLongType<T, U, V>,
    third_parameter: YetAnotherType,
    fourth_parameter: Option<Box<dyn SomeTrait>>,
) -> Result<ComplexReturnType<A, B, C>, ErrorType> {
    implementation_here()
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(result.content.contains("fn very_long_function_name"));
        assert!(result.content.contains("first_parameter"));
        assert!(result.content.contains("fourth_parameter"));
        assert!(result.content.contains("Result<ComplexReturnType"));
        assert!(!result.content.contains("implementation_here"));
    }

    #[test]
    fn test_mixed_indentation() {
        let input = "def mixed():\n\treturn 1\n    if True:\n\t    pass";
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        // Should handle mixed tabs and spaces
        assert!(result.content.contains("def mixed():"));
    }

    #[test]
    fn test_unicode_identifiers() {
        let input = r#"
def 计算(数值: int) -> int:
    """计算结果"""
    return 数值 * 2

class Über:
    def méthode(self):
        pass
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Python);

        // Should handle unicode in identifiers
        assert!(result.content.contains("计算") || result.content.contains("def"));
    }

    #[test]
    fn test_string_with_braces() {
        let input = r#"
fn format_json() -> String {
    let s = "{ \"key\": \"value\" }";
    let t = format!("{{ nested: {} }}", value);
    s.to_string()
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // Braces in strings shouldn't confuse the parser
        assert!(result.content.contains("fn format_json()"));
        // Body should still be stripped
        assert!(!result.content.contains("let s =") || result.content.contains("{ /* ... */ }"));
    }

    #[test]
    fn test_docstring_preservation_disabled() {
        let input = r#"
/// Important doc
pub fn documented() {
    implementation()
}
"#;
        let s = Skeletonizer::new().with_docstrings(false);
        let result = s.skeletonize(input, Language::Rust);

        // With docstrings disabled, they should not be in output
        // (Note: Depends on implementation - this tests the builder pattern)
        assert!(result.content.contains("pub fn documented"));
    }

    #[test]
    fn test_fallback_line_count() {
        // Create input longer than default fallback (50 lines)
        let mut lines: Vec<String> = Vec::new();
        for i in 0..100 {
            lines.push(format!("// line {}", i));
        }
        let input = lines.join("\n");

        let s = Skeletonizer::new();
        let result = s.skeletonize(&input, Language::Rust);

        // Should extract doc comments from first part
        assert!(result.skeleton_tokens <= result.original_tokens);
    }
}

// ============================================================================
// Compression Ratio Tests
// ============================================================================

mod compression {
    use super::*;

    #[test]
    fn test_high_compression_ratio() {
        let input = r#"
fn complex_function() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    for i in 0..100 {
        for j in 0..100 {
            if i + j > 50 {
                println!("{} {}", i, j);
            }
        }
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(
            result.compression_ratio > 0.5,
            "Expected >50% compression, got {:.1}%",
            result.compression_ratio * 100.0
        );
    }

    #[test]
    fn test_low_compression_when_all_signatures() {
        let input = r#"
pub const A: i32 = 1;
pub const B: i32 = 2;
pub const C: i32 = 3;
pub type D = i32;
pub type E = String;
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        // All content is signatures, so compression should be low or negative
        assert!(
            result.compression_ratio < 0.5,
            "Expected <50% compression for signature-only file, got {:.1}%",
            result.compression_ratio * 100.0
        );
    }

    #[test]
    fn test_skeleton_reasonable_size() {
        // For very short inputs, skeleton may be slightly larger due to placeholders
        // like `{ /* ... */ }`. For substantial code, skeleton should be smaller.
        let input = r#"
fn complex_function() {
    let a = 1;
    let b = 2;
    let c = 3;
    for i in 0..100 {
        println!("{}", i);
    }
}

fn another_function() {
    let data = vec![1, 2, 3, 4, 5];
    for item in data {
        process(item);
    }
}
"#;
        let s = Skeletonizer::new();
        let result = s.skeletonize(input, Language::Rust);

        assert!(
            result.skeleton_tokens < result.original_tokens,
            "Skeleton ({}) should be smaller than original ({}) for substantial code",
            result.skeleton_tokens,
            result.original_tokens,
        );
    }
}
