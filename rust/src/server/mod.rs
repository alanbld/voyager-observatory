//! Minimal MCP (Model Context Protocol) Server
//!
//! A lightweight, synchronous JSON-RPC 2.0 implementation for MCP.
//! No async runtime required - uses blocking stdin/stdout.
//!
//! # Protocol
//! - JSON-RPC 2.0 over stdio (line-delimited JSON)
//! - MCP initialize handshake
//! - Tools: get_context, zoom, session_list, report_utility
//!
//! # Usage
//! ```bash
//! pm_encoder --server
//! ```

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::core::{
    ContextEngine, EncoderConfig, ZoomConfig, ZoomTarget, ZoomDepth,
    SymbolResolver, CallGraphAnalyzer, ZoomSuggestion,
    ZoomSessionStore, ContextStore, DEFAULT_ALPHA, OutputFormat,
    SkeletonMode,
    // Phase 2: Rich Context
    UsageFinder, RelatedContext,
};
use crate::{LensManager, parse_token_budget};

// ============================================================================
// JSON-RPC 2.0 Types
// ============================================================================

/// JSON-RPC 2.0 Request
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

// JSON-RPC error codes
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;
const INTERNAL_ERROR: i32 = -32603;

// ============================================================================
// MCP Tool Response Helpers
// ============================================================================

/// Create a successful MCP tool response with isError: false
fn tool_success(id: Value, text: String) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({
        "content": [{
            "type": "text",
            "text": text
        }],
        "isError": false
    }))
}

/// Create an error MCP tool response with isError: true
fn tool_error(id: Value, message: String) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({
        "content": [{
            "type": "text",
            "text": message
        }],
        "isError": true
    }))
}

// ============================================================================
// MCP Server
// ============================================================================

/// MCP Server state
pub struct McpServer {
    initialized: bool,
    project_root: PathBuf,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            initialized: false,
            project_root,
        }
    }

    /// Run the server loop (blocking)
    pub fn run(&mut self) -> io::Result<()> {
        // Note: No startup logs - wait for initialize before logging
        let stdin = io::stdin();
        let mut stdout = io::stdout();

        for line in stdin.lock().lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            // Parse and handle request - may return None for notifications
            if let Some(response) = self.handle_request(&line) {
                // Write response only for requests (not notifications)
                let response_str = serde_json::to_string(&response)
                    .unwrap_or_else(|e| {
                        serde_json::to_string(&JsonRpcResponse::error(
                            Value::Null,
                            INTERNAL_ERROR,
                            format!("Serialization error: {}", e),
                        )).unwrap()
                    });

                writeln!(stdout, "{}", response_str)?;
                stdout.flush()?;
            }
        }

        eprintln!("[MCP] Server shutting down");
        Ok(())
    }

    /// Handle a JSON-RPC request. Returns None for notifications (no id).
    fn handle_request(&mut self, line: &str) -> Option<JsonRpcResponse> {
        // Parse JSON
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                return Some(JsonRpcResponse::error(
                    Value::Null,
                    PARSE_ERROR,
                    format!("Parse error: {}", e),
                ));
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return Some(JsonRpcResponse::error(
                request.id.unwrap_or(Value::Null),
                INVALID_REQUEST,
                "Invalid JSON-RPC version".to_string(),
            ));
        }

        // Check if this is a notification (no id = no response expected)
        let is_notification = request.id.is_none();
        let id = request.id.clone().unwrap_or(Value::Null);

        // Route method
        let response = match request.method.as_str() {
            // MCP lifecycle
            "initialize" => self.handle_initialize(id, request.params),
            "initialized" => {
                // This is a notification - no response required
                return None;
            }
            "shutdown" => {
                self.initialized = false;
                JsonRpcResponse::success(id, json!({}))
            }

            // MCP discovery
            "tools/list" => self.handle_tools_list(id),

            // MCP tool calls
            "tools/call" => self.handle_tools_call(id, request.params),

            _ => JsonRpcResponse::error(
                id,
                METHOD_NOT_FOUND,
                format!("Unknown method: {}", request.method),
            ),
        };

        // Don't respond to notifications
        if is_notification {
            None
        } else {
            Some(response)
        }
    }

    fn handle_initialize(&mut self, id: Value, _params: Option<Value>) -> JsonRpcResponse {
        self.initialized = true;
        eprintln!("[MCP] Initialized");

        JsonRpcResponse::success(id, json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "pm_encoder",
                "version": crate::version()
            }
        }))
    }

    fn handle_tools_list(&self, id: Value) -> JsonRpcResponse {
        let tools = json!({
            "tools": [
                {
                    "name": "get_context",
                    "description": "Serialize a directory into LLM-optimized context format",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "Path to serialize (default: project root)"
                            },
                            "lens": {
                                "type": "string",
                                "description": "Context lens: architecture, debug, security, minimal, onboarding"
                            },
                            "token_budget": {
                                "type": "string",
                                "description": "Token budget (e.g., '100k', '2M')"
                            },
                            "format": {
                                "type": "string",
                                "description": "Output format: plusminus, xml, markdown, claude-xml"
                            },
                            "skeleton": {
                                "type": "string",
                                "description": "Skeleton mode: 'auto' (enable if budget set), 'true', 'false'. Extracts signatures, strips bodies."
                            }
                        }
                    }
                },
                {
                    "name": "zoom",
                    "description": "Zoom into a specific function, class, or file for detailed context",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "target": {
                                "type": "string",
                                "description": "Zoom target (e.g., 'function=main', 'class=Config', 'file=src/lib.rs:10-50')"
                            },
                            "path": {
                                "type": "string",
                                "description": "Optional: Override project root path (default: server root)"
                            },
                            "session_id": {
                                "type": "string",
                                "description": "Optional session ID to track zoom history"
                            }
                        },
                        "required": ["target"]
                    }
                },
                {
                    "name": "session_list",
                    "description": "List all saved zoom sessions",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "session_create",
                    "description": "Create a new zoom session",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Session name"
                            },
                            "description": {
                                "type": "string",
                                "description": "Optional session description"
                            }
                        },
                        "required": ["name"]
                    }
                },
                {
                    "name": "report_utility",
                    "description": "Report the utility of a file for learning",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "path": {
                                "type": "string",
                                "description": "File path"
                            },
                            "utility": {
                                "type": "number",
                                "description": "Utility score (0.0 to 1.0)"
                            },
                            "reason": {
                                "type": "string",
                                "description": "Optional reason for the rating"
                            }
                        },
                        "required": ["path", "utility"]
                    }
                }
            ]
        });

        JsonRpcResponse::success(id, tools)
    }

    fn handle_tools_call(&self, id: Value, params: Option<Value>) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    "Missing params".to_string(),
                );
            }
        };

        let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        match tool_name {
            "get_context" => self.tool_get_context(id, arguments),
            "zoom" => self.tool_zoom(id, arguments),
            "session_list" => self.tool_session_list(id),
            "session_create" => self.tool_session_create(id, arguments),
            "report_utility" => self.tool_report_utility(id, arguments),
            _ => JsonRpcResponse::error(
                id,
                METHOD_NOT_FOUND,
                format!("Unknown tool: {}", tool_name),
            ),
        }
    }

    // ========================================================================
    // Tool Implementations
    // ========================================================================

    fn tool_get_context(&self, id: Value, args: Value) -> JsonRpcResponse {
        let path = args.get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.project_root.clone());

        let lens = args.get("lens").and_then(|v| v.as_str());
        let token_budget = args.get("token_budget").and_then(|v| v.as_str());
        let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("plusminus");
        let skeleton = args.get("skeleton").and_then(|v| v.as_str()).unwrap_or("auto");

        // TODO: Load project .pm_encoder_config.json when core::EncoderConfig supports Deserialize
        // For now, use defaults - the lens will override patterns anyway
        let mut config = EncoderConfig::default();
        config.output_format = match format {
            "xml" => OutputFormat::Xml,
            "markdown" => OutputFormat::Markdown,
            "claude-xml" => OutputFormat::ClaudeXml,
            _ => OutputFormat::PlusMinus,
        };

        // Apply skeleton mode (v2.2.0)
        config.skeleton_mode = SkeletonMode::from_str(skeleton).unwrap_or(SkeletonMode::Auto);

        // Apply lens and merge patterns into config
        let mut lens_manager = LensManager::new();
        if let Some(lens_name) = lens {
            match lens_manager.apply_lens(lens_name) {
                Ok(applied) => {
                    // Merge lens patterns into config
                    config.ignore_patterns.extend(applied.ignore_patterns);
                    if !applied.include_patterns.is_empty() {
                        config.include_patterns = applied.include_patterns;
                    }
                    config.active_lens = Some(lens_name.to_string());
                }
                Err(e) => {
                    return tool_error(id, format!("Invalid lens '{}': {}", lens_name, e));
                }
            }
        }

        // Parse token budget
        if let Some(budget_str) = token_budget {
            match parse_token_budget(budget_str) {
                Ok(budget) => config.token_budget = Some(budget),
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        INVALID_PARAMS,
                        format!("Invalid token budget: {}", e),
                    );
                }
            }
        }

        // Generate context
        let engine = ContextEngine::with_config(config);
        match engine.serialize(path.to_str().unwrap_or(".")) {
            Ok(context) => tool_success(id, context),
            Err(e) => tool_error(id, format!("Serialization failed: {}", e)),
        }
    }

    fn tool_zoom(&self, id: Value, args: Value) -> JsonRpcResponse {
        let target_str = match args.get("target").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    "Missing 'target' parameter".to_string(),
                );
            }
        };

        // Parse optional path override (default: server's project_root)
        let project_root = args.get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| self.project_root.clone());

        // Parse target (e.g., "function=main", "file=src/lib.rs:10-50")
        let parts: Vec<&str> = target_str.splitn(2, '=').collect();
        if parts.len() != 2 {
            return JsonRpcResponse::error(
                id,
                INVALID_PARAMS,
                format!("Invalid target format '{}'. Expected <type>=<value>", target_str),
            );
        }

        let (target_type, target_value) = (parts[0], parts[1]);

        // Build ZoomTarget
        let mut target = match target_type {
            "function" | "fn" => ZoomTarget::Function(target_value.to_string()),
            "class" | "struct" => ZoomTarget::Class(target_value.to_string()),
            "module" | "mod" => ZoomTarget::Module(target_value.to_string()),
            "file" => {
                // Parse optional line range
                if let Some(colon_pos) = target_value.rfind(':') {
                    let path = target_value[..colon_pos].to_string();
                    let range = &target_value[colon_pos + 1..];
                    if let Some(dash_pos) = range.find('-') {
                        let start = range[..dash_pos].parse().ok();
                        let end = range[dash_pos + 1..].parse().ok();
                        ZoomTarget::File { path, start_line: start, end_line: end }
                    } else {
                        ZoomTarget::File { path, start_line: range.parse().ok(), end_line: None }
                    }
                } else {
                    ZoomTarget::File { path: target_value.to_string(), start_line: None, end_line: None }
                }
            }
            _ => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    format!("Unknown target type '{}'. Use: function, class, module, file", target_type),
                );
            }
        };

        // Symbol resolution for function/class
        let resolved_name = match &target {
            ZoomTarget::Function(name) | ZoomTarget::Class(name) => Some(name.clone()),
            _ => None,
        };

        if let ZoomTarget::Function(name) = &target {
            let resolver = SymbolResolver::new();
            match resolver.find_function(name, &project_root) {
                Ok(loc) => {
                    target = ZoomTarget::File {
                        path: loc.path,
                        start_line: Some(loc.start_line),
                        end_line: Some(loc.end_line),
                    };
                }
                Err(e) => {
                    return JsonRpcResponse::error(id, INVALID_PARAMS, e);
                }
            }
        } else if let ZoomTarget::Class(name) = &target {
            let resolver = SymbolResolver::new();
            match resolver.find_class(name, &project_root) {
                Ok(loc) => {
                    target = ZoomTarget::File {
                        path: loc.path,
                        start_line: Some(loc.start_line),
                        end_line: Some(loc.end_line),
                    };
                }
                Err(e) => {
                    return JsonRpcResponse::error(id, INVALID_PARAMS, e);
                }
            }
        }

        // Build zoom config
        let zoom_config = ZoomConfig {
            target,
            budget: None,
            depth: ZoomDepth::Full,
            include_tests: false,
            context_lines: 5,
        };

        // Execute zoom
        let engine = ContextEngine::new();
        match engine.zoom(project_root.to_str().unwrap_or("."), &zoom_config) {
            Ok(mut output) => {
                // Add zoom menu with call graph analysis (callees)
                let call_analyzer = CallGraphAnalyzer::new().with_max_results(10);
                let resolver = SymbolResolver::new();
                let valid_calls = call_analyzer.get_valid_calls(&output, &resolver, &project_root);

                let mut callees: Vec<ZoomSuggestion> = Vec::new();
                if !valid_calls.is_empty() {
                    let mut seen = std::collections::HashSet::new();
                    callees = valid_calls.iter()
                        .filter(|(call, _)| {
                            if let Some(ref orig) = resolved_name {
                                if &call.name == orig {
                                    return false;
                                }
                            }
                            seen.insert(call.name.clone())
                        })
                        .map(|(call, loc)| ZoomSuggestion::from_call(call, loc))
                        .collect();

                    if !callees.is_empty() {
                        let menu_items: Vec<String> = callees.iter()
                            .map(|s| format!("  {}", s.to_xml()))
                            .collect();
                        output.push_str(&format!("\n<zoom_menu>\n{}\n</zoom_menu>", menu_items.join("\n")));
                    }
                }

                // Phase 2: Add related_context with callers (reverse call graph)
                if let Some(ref name) = resolved_name {
                    let usage_finder = UsageFinder::new().with_max_results(10);
                    let callers = usage_finder.find_usages(
                        name,
                        &project_root,
                        None,  // definition_path - let it search everywhere
                        None,  // definition_line
                    );

                    if !callers.is_empty() || !callees.is_empty() {
                        let related = RelatedContext {
                            callers,
                            callees: callees.clone(),
                        };
                        output.push_str("\n");
                        output.push_str(&related.to_xml());
                    }
                }

                tool_success(id, output)
            }
            Err(e) => tool_error(id, format!("Zoom failed: {}", e)),
        }
    }

    fn tool_session_list(&self, id: Value) -> JsonRpcResponse {
        let session_path = ZoomSessionStore::default_path(&self.project_root);

        match ZoomSessionStore::load(&session_path) {
            Ok(store) => {
                let sessions: Vec<Value> = store.list_sessions_with_meta()
                    .iter()
                    .map(|(name, is_active, last_accessed)| {
                        json!({
                            "name": name,
                            "active": is_active,
                            "last_accessed": last_accessed
                        })
                    })
                    .collect();

                tool_success(id, serde_json::to_string_pretty(&sessions).unwrap_or_default())
            }
            Err(e) => tool_error(id, format!("Failed to load sessions: {}", e)),
        }
    }

    fn tool_session_create(&self, id: Value, args: Value) -> JsonRpcResponse {
        let name = match args.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    "Missing 'name' parameter".to_string(),
                );
            }
        };

        let description = args.get("description").and_then(|v| v.as_str());
        let session_path = ZoomSessionStore::default_path(&self.project_root);

        match ZoomSessionStore::with_persistence(&session_path, |store| {
            if let Some(desc) = description {
                store.create_session_with_desc(name, desc);
            } else {
                store.create_session(name);
            }
            store.session_count()
        }) {
            Ok(count) => tool_success(id, format!("Created session '{}'. Total sessions: {}", name, count)),
            Err(e) => tool_error(id, format!("Failed to create session: {}", e)),
        }
    }

    fn tool_report_utility(&self, id: Value, args: Value) -> JsonRpcResponse {
        let path = match args.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    "Missing 'path' parameter".to_string(),
                );
            }
        };

        let utility = match args.get("utility").and_then(|v| v.as_f64()) {
            Some(u) if (0.0..=1.0).contains(&u) => u,
            Some(u) => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    format!("Utility must be between 0.0 and 1.0, got: {}", u),
                );
            }
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    "Missing 'utility' parameter".to_string(),
                );
            }
        };

        let reason = args.get("reason").and_then(|v| v.as_str()).unwrap_or("MCP feedback");

        let store_path = ContextStore::default_path(&self.project_root);
        let mut store = ContextStore::load_from_file(&store_path);

        store.report_utility(path, utility, DEFAULT_ALPHA);

        if let Err(e) = store.save_to_file(&store_path) {
            return tool_error(id, format!("Failed to save: {}", e));
        }

        let current = store.get_utility_score(path);
        tool_success(id, format!("Utility reported for '{}': {:.2} → {:.2} ({})", path, utility, current, reason))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(json!(1), json!({"result": "ok"}));
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(json!(1), -32600, "Invalid request".to_string());
        assert!(resp.error.is_some());
        assert!(resp.result.is_none());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn test_parse_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.id, Some(json!(1)));
    }

    #[test]
    fn test_mcp_server_new() {
        let server = McpServer::new(PathBuf::from("/tmp"));
        assert!(!server.initialized);
        assert_eq!(server.project_root, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_handle_initialize() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#).unwrap();

        assert!(resp.error.is_none());
        assert!(server.initialized);

        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "pm_encoder");
    }

    #[test]
    fn test_handle_tools_list() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();

        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();

        // Should have 5 tools
        assert_eq!(tools.len(), 5);

        // Check tool names
        let tool_names: Vec<&str> = tools.iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(tool_names.contains(&"get_context"));
        assert!(tool_names.contains(&"zoom"));
        assert!(tool_names.contains(&"session_list"));
        assert!(tool_names.contains(&"session_create"));
        assert!(tool_names.contains(&"report_utility"));
    }

    #[test]
    fn test_handle_invalid_json() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request("not json").unwrap();

        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, PARSE_ERROR);
    }

    #[test]
    fn test_handle_invalid_version() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(r#"{"jsonrpc":"1.0","id":1,"method":"test"}"#).unwrap();

        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, INVALID_REQUEST);
    }

    #[test]
    fn test_handle_unknown_method() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(r#"{"jsonrpc":"2.0","id":1,"method":"unknown/method"}"#).unwrap();

        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_handle_shutdown() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));

        // Initialize first
        server.handle_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);
        assert!(server.initialized);

        // Shutdown
        let resp = server.handle_request(r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#).unwrap();
        assert!(resp.error.is_none());
        assert!(!server.initialized);
    }

    #[test]
    fn test_handle_initialized_notification() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        // First initialize
        server.handle_request(r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#);

        // "initialized" is a notification (no id) - should return None
        let resp = server.handle_request(r#"{"jsonrpc":"2.0","method":"initialized"}"#);
        assert!(resp.is_none(), "Notifications should not get a response");
    }

    #[test]
    fn test_tool_zoom_missing_target() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"zoom","arguments":{}}}"#
        ).unwrap();

        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("target"));
    }

    #[test]
    fn test_tool_zoom_invalid_target_format() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"zoom","arguments":{"target":"invalid"}}}"#
        ).unwrap();

        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, INVALID_PARAMS);
        assert!(err.message.contains("Invalid target format"));
    }

    #[test]
    fn test_tool_get_context() {
        // Create temp directory with test file
        let temp_dir = std::env::temp_dir().join("pm_mcp_test_context");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(temp_dir.join("test.txt"), "Hello world").unwrap();

        let mut server = McpServer::new(temp_dir.clone());
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_context","arguments":{}}}"#
        ).unwrap();

        assert!(resp.error.is_none(), "Expected success, got error: {:?}", resp.error);
        let result = resp.result.unwrap();
        let content = result["content"][0]["text"].as_str().unwrap();
        assert!(content.contains("test.txt"), "Should contain test file");

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_tool_report_utility_missing_params() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));

        // Missing path
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"report_utility","arguments":{"utility":0.5}}}"#
        ).unwrap();
        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().message.contains("path"));

        // Missing utility
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"report_utility","arguments":{"path":"test.rs"}}}"#
        ).unwrap();
        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().message.contains("utility"));
    }

    #[test]
    fn test_tool_report_utility_invalid_range() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"report_utility","arguments":{"path":"test.rs","utility":1.5}}}"#
        ).unwrap();

        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().message.contains("between 0.0 and 1.0"));
    }

    #[test]
    fn test_tool_session_create_missing_name() {
        let mut server = McpServer::new(PathBuf::from("/tmp"));
        let resp = server.handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"session_create","arguments":{}}}"#
        ).unwrap();

        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().message.contains("name"));
    }
}
