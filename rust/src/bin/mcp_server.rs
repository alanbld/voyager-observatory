//! pm_encoder MCP Server
//!
//! Model Context Protocol server for pm_encoder, allowing AI assistants
//! to serialize codebases directly.
//!
//! Build: cargo build --features mcp --bin pm_encoder_mcp
//! Run:   ./target/debug/pm_encoder_mcp

use pm_encoder::{
    ContextEngine, EncoderConfig, LensManager,
    parse_token_budget, apply_token_budget,
};
use rmcp::{
    schemars,
    schemars::JsonSchema,
    ServerHandler, ServiceExt,
    handler::server::tool::ToolRouter,
    model::{
        CallToolRequestParam, CallToolResult, Content, Implementation, ListToolsResult,
        ServerCapabilities, ServerInfo, Tool, ToolsCapability,
    },
    service::{RequestContext, RoleServer},
};
use serde::Deserialize;
use tokio::io::{stdin, stdout};

/// MCP Server for pm_encoder
#[derive(Clone)]
struct PmEncoderServer {
    tool_router: ToolRouter<Self>,
}

/// Input for get_context tool
#[derive(Debug, Deserialize, JsonSchema)]
struct GetContextParams {
    /// List of files with path and content
    files: Vec<FileInput>,
    /// Optional lens name (architecture, debug, security, minimal, onboarding)
    #[serde(default)]
    lens: Option<String>,
    /// Truncate files to this many lines (0 = no truncation)
    #[serde(default)]
    truncate_lines: Option<usize>,
    /// Maximum token budget (e.g., "100000", "100k", "2M")
    #[serde(default)]
    token_budget: Option<String>,
    /// Budget strategy: "drop", "truncate", or "hybrid"
    #[serde(default)]
    budget_strategy: Option<String>,
}

/// A file with path and content
#[derive(Debug, Deserialize, JsonSchema)]
struct FileInput {
    /// File path (e.g., "src/main.py")
    path: String,
    /// File content
    content: String,
}

/// Input for list_lenses tool (no params needed)
#[derive(Debug, Deserialize, JsonSchema)]
struct ListLensesParams {}

impl PmEncoderServer {
    fn new() -> Self {
        // Build the tool router with our tools
        let tool_router = ToolRouter::new()
            .with_route(Self::get_context_route())
            .with_route(Self::list_lenses_route());

        Self { tool_router }
    }

    fn get_context_route() -> rmcp::handler::server::tool::ToolRoute<Self> {
        let tool = Tool::new(
            "get_context",
            "Serialize files into LLM-optimized context using Plus/Minus format. Supports context lenses, token budgeting, and file truncation.",
            rmcp::handler::server::tool::schema_for_type::<GetContextParams>(),
        );

        rmcp::handler::server::tool::ToolRoute::new_dyn(tool, |ctx| {
            Box::pin(async move {
                let params: GetContextParams = rmcp::handler::server::tool::parse_json_object(
                    ctx.arguments.unwrap_or_default(),
                )?;

                // Build config
                let mut config = EncoderConfig::default();

                if let Some(lines) = params.truncate_lines {
                    config.truncate_lines = lines;
                }

                // Create lens manager for priority resolution
                let mut lens_manager = LensManager::new();

                // Apply lens if specified
                if let Some(ref lens_name) = params.lens {
                    lens_manager.apply_lens(lens_name).map_err(|e| {
                        rmcp::ErrorData::invalid_params(
                            format!("Invalid lens '{}': {}", lens_name, e),
                            None,
                        )
                    })?;
                }

                // Convert files to tuples
                let files: Vec<(String, String)> = params
                    .files
                    .into_iter()
                    .map(|f| (f.path, f.content))
                    .collect();

                // Apply token budget if specified
                let selected_files = if let Some(ref budget_str) = params.token_budget {
                    let budget = parse_token_budget(budget_str).map_err(|e| {
                        rmcp::ErrorData::invalid_params(
                            format!("Invalid token budget '{}': {}", budget_str, e),
                            None,
                        )
                    })?;

                    let strategy = params.budget_strategy.as_deref().unwrap_or("drop");
                    let (selected, _report) = apply_token_budget(files, budget, &lens_manager, strategy);
                    selected
                } else {
                    files
                };

                // Create engine with optional lens
                let engine = if let Some(lens_name) = params.lens {
                    ContextEngine::with_lens(config, &lens_name).map_err(|e| {
                        rmcp::ErrorData::invalid_params(
                            format!("Invalid lens '{}': {}", lens_name, e),
                            None,
                        )
                    })?
                } else {
                    ContextEngine::new(config)
                };

                // Generate context
                let context = engine.generate_context(&selected_files);

                Ok(CallToolResult::success(vec![Content::text(context)]))
            })
        })
    }

    fn list_lenses_route() -> rmcp::handler::server::tool::ToolRoute<Self> {
        let tool = Tool::new(
            "list_lenses",
            "Get a list of available context lenses with their descriptions.",
            rmcp::handler::server::tool::schema_for_type::<ListLensesParams>(),
        );

        rmcp::handler::server::tool::ToolRoute::new_dyn(tool, |_ctx| {
            Box::pin(async move {
                let lenses = vec![
                    ("architecture", "Signatures only - best for understanding structure"),
                    ("debug", "Full content - for debugging and deep analysis"),
                    ("security", "Auth, crypto, validation focus"),
                    ("minimal", "Entry points only - smallest context"),
                    ("onboarding", "Balanced view for new contributors"),
                ];

                let output = lenses
                    .iter()
                    .map(|(name, desc)| format!("- {}: {}", name, desc))
                    .collect::<Vec<_>>()
                    .join("\n");

                let header = format!(
                    "pm_encoder v{} - Available Lenses:\n\n{}",
                    pm_encoder::version(),
                    output
                );

                Ok(CallToolResult::success(vec![Content::text(header)]))
            })
        })
    }
}

impl ServerHandler for PmEncoderServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "pm_encoder".into(),
                version: pm_encoder::version().into(),
                title: Some("pm_encoder Context Serializer".into()),
                icons: None,
                website_url: Some("https://github.com/alanbld/pm_encoder".into()),
            },
            instructions: Some(
                "Use get_context to serialize code files into LLM-optimized context. \
                 Use list_lenses to see available context lenses."
                    .into(),
            ),
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            Ok(ListToolsResult {
                tools: self.tool_router.list_all(),
                next_cursor: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: CallToolRequestParam,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            let tool_context =
                rmcp::handler::server::tool::ToolCallContext::new(self, request, context);
            self.tool_router.call(tool_context).await
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the MCP server
    let server = PmEncoderServer::new();

    // Log to stderr so stdout is clean for MCP protocol
    eprintln!("pm_encoder MCP Server v{} starting...", pm_encoder::version());

    // Set up stdio transport for MCP
    let transport = (stdin(), stdout());

    // Serve the MCP protocol
    let service = server.serve(transport).await?;

    // Wait for the client to disconnect
    let _quit_reason = service.waiting().await?;

    Ok(())
}
