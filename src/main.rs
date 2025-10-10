//! Deliberate Thinking MCP Server
//!
//! A Model Context Protocol (MCP) server that enables dynamic and reflective problem-solving
//! through a structured thinking process. This server provides a tool for breaking down complex
//! problems into manageable thoughts that can be revised, branched, and evolved as understanding
//! deepens.
//!
//! ## Architecture
//!
//! - **DeliberateThinkingServer**: Main server implementation handling MCP protocol
//! - **DeliberateThinkingState**: Manages thought history, branching, and revisions
//! - **DeliberateThinkingRequest**: Input parameters for each thinking step
//! - **DeliberateThinkingResponse**: Response tracking thought progress and state
//! - **ThoughtData**: Internal representation of a single thought with metadata

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters, ServerHandler},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData as McpError, Implementation,
        InitializeResult, ProtocolVersion, ServerCapabilities, ToolsCapability,
    },
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
    ServiceExt,
};
use serde::{Deserialize, Serialize};

// JSON-RPC 2.0 error codes
// See: https://www.jsonrpc.org/specification#error_object
/// Invalid method parameter(s)
const JSONRPC_INVALID_PARAMS: i32 = -32602;
/// Internal JSON-RPC error
const JSONRPC_INTERNAL_ERROR: i32 = -32603;

/// Deliberate thinking request parameters
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct DeliberateThinkingRequest {
    #[schemars(description = "Current thinking step")]
    pub thought: String,
    #[serde(rename = "nextThoughtNeeded")]
    #[schemars(description = "Whether another thought step is needed")]
    pub next_thought_needed: bool,
    #[serde(rename = "thoughtNumber")]
    #[schemars(description = "Current thought number (minimum 1)", range(min = 1))]
    pub thought_number: u32,
    #[serde(rename = "totalThoughts")]
    #[schemars(description = "Estimated total thoughts needed (minimum 1)", range(min = 1))]
    pub total_thoughts: u32,
    #[serde(rename = "isRevision", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Whether this revises previous thinking")]
    pub is_revision: Option<bool>,
    #[serde(rename = "revisesThought", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Which thought number is being reconsidered")]
    pub revises_thought: Option<u32>,
    #[serde(rename = "branchFromThought", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Branching point thought number")]
    pub branch_from_thought: Option<u32>,
    #[serde(rename = "branchId", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Branch identifier")]
    pub branch_id: Option<String>,
    #[serde(rename = "needsMoreThoughts", skip_serializing_if = "Option::is_none")]
    #[schemars(description = "If more thoughts are needed")]
    pub needs_more_thoughts: Option<bool>,
}

impl DeliberateThinkingRequest {
    /// Validates the request parameters
    fn validate(&self) -> Result<(), McpError> {
        validate_min_value("thoughtNumber", self.thought_number, 1)?;
        validate_min_value("totalThoughts", self.total_thoughts, 1)?;

        if let Some(revises) = self.revises_thought {
            validate_min_value("revisesThought", revises, 1)?;
        }

        if let Some(branch_from) = self.branch_from_thought {
            validate_min_value("branchFromThought", branch_from, 1)?;
        }

        Ok(())
    }
}

/// Response for deliberate thinking tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberateThinkingResponse {
    #[serde(rename = "thoughtNumber")]
    pub thought_number: u32,
    #[serde(rename = "totalThoughts")]
    pub total_thoughts: u32,
    #[serde(rename = "nextThoughtNeeded")]
    pub next_thought_needed: bool,
    pub branches: Vec<String>,
    #[serde(rename = "thoughtHistoryLength")]
    pub thought_history_length: u32,
}

impl DeliberateThinkingResponse {
    /// Creates a new response from a request and state info
    fn new(
        request: &DeliberateThinkingRequest,
        branches: Vec<String>,
        thought_history_length: u32,
    ) -> Self {
        Self {
            thought_number: request.thought_number,
            total_thoughts: request.total_thoughts,
            next_thought_needed: request.next_thought_needed,
            branches,
            thought_history_length,
        }
    }
}

/// Internal thought data for tracking
#[derive(Debug, Clone)]
pub struct ThoughtData {
    pub thought: String,
    pub thought_number: u32,
    pub total_thoughts: u32,
    pub next_thought_needed: bool,
    pub is_revision: Option<bool>,
    pub revises_thought: Option<u32>,
    pub branch_from_thought: Option<u32>,
    pub branch_id: Option<String>,
    pub needs_more_thoughts: Option<bool>,
}

impl From<DeliberateThinkingRequest> for ThoughtData {
    fn from(req: DeliberateThinkingRequest) -> Self {
        Self {
            thought: req.thought,
            thought_number: req.thought_number,
            total_thoughts: req.total_thoughts,
            next_thought_needed: req.next_thought_needed,
            is_revision: req.is_revision,
            revises_thought: req.revises_thought,
            branch_from_thought: req.branch_from_thought,
            branch_id: req.branch_id,
            needs_more_thoughts: req.needs_more_thoughts,
        }
    }
}

/// Deliberate thinking server state
#[derive(Debug, Default)]
pub struct DeliberateThinkingState {
    thought_history: Vec<ThoughtData>,
    branches: HashMap<String, Vec<ThoughtData>>,
    current_branch: Option<String>,
}

impl DeliberateThinkingState {
    /// Gets the current thought history (from branch or main)
    fn get_current_history(&self) -> &[ThoughtData] {
        match &self.current_branch {
            Some(branch_id) => self.branches
                .get(branch_id)
                .map(|v| v.as_slice())
                .unwrap_or(&self.thought_history),
            None => &self.thought_history,
        }
    }

    /// Gets the current thought history length
    fn get_history_length(&self) -> u32 {
        // Safe cast: thought history is bounded by practical memory limits
        // and will never exceed u32::MAX in realistic usage
        #[allow(clippy::cast_possible_truncation)]
        let length = self.get_current_history().len() as u32;
        length
    }

    /// Handles branching logic
    fn handle_branching(
        &mut self,
        branch_from: u32,
        branch_id: String,
        thought_data: ThoughtData,
    ) {
        // Create branch if it doesn't exist
        if !self.branches.contains_key(&branch_id) {
            let branch_base: Vec<ThoughtData> = self
                .thought_history
                .iter()
                .take_while(|t| t.thought_number <= branch_from)
                .cloned()
                .collect();
            self.branches.insert(branch_id.clone(), branch_base);
        }

        // Add thought to the branch
        if let Some(branch) = self.branches.get_mut(&branch_id) {
            branch.push(thought_data);
        }

        self.current_branch = Some(branch_id);
    }

    /// Handles revision of existing thoughts
    fn handle_revision(&mut self, revises: u32, thought_data: ThoughtData) {
        match &self.current_branch {
            Some(branch_id) => {
                if let Some(branch) = self.branches.get_mut(branch_id) {
                    Self::revise_or_append(branch, revises, thought_data);
                }
            }
            None => {
                Self::revise_or_append(&mut self.thought_history, revises, thought_data);
            }
        }
    }

    /// Helper to revise a thought in a list or append if not found
    fn revise_or_append(
        thoughts: &mut Vec<ThoughtData>,
        revises: u32,
        thought_data: ThoughtData,
    ) {
        if let Some(thought) = thoughts.iter_mut().find(|t| t.thought_number == revises) {
            *thought = thought_data;
        } else {
            thoughts.push(thought_data);
        }
    }

    /// Adds a regular thought to the current context
    fn add_thought(&mut self, thought_data: ThoughtData) {
        match &self.current_branch {
            Some(branch_id) => {
                if let Some(branch) = self.branches.get_mut(branch_id) {
                    branch.push(thought_data);
                }
            }
            None => {
                self.thought_history.push(thought_data);
            }
        }
    }

    /// Gets all branch names
    fn get_branch_names(&self) -> Vec<String> {
        self.branches.keys().cloned().collect()
    }
}

/// Deliberate thinking server implementation
#[derive(Clone)]
pub struct DeliberateThinkingServer {
    state: Arc<Mutex<DeliberateThinkingState>>,
    tool_router: ToolRouter<Self>,
}

impl DeliberateThinkingServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(DeliberateThinkingState::default())),
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for DeliberateThinkingServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to validate minimum values
fn validate_min_value(field_name: &str, value: u32, min: u32) -> Result<(), McpError> {
    if value < min {
        Err(create_validation_error(
            &format!("{} must be at least {}", field_name, min)
        ))
    } else {
        Ok(())
    }
}

/// Helper function to create validation errors
fn create_validation_error(message: &str) -> McpError {
    McpError {
        code: ErrorCode(JSONRPC_INVALID_PARAMS),
        message: message.to_string().into(),
        data: None,
    }
}

/// Helper function to create serialization errors
fn create_serialization_error(error: impl std::fmt::Display) -> McpError {
    McpError {
        code: ErrorCode(JSONRPC_INTERNAL_ERROR),
        message: format!("Failed to serialize response: {}", error).into(),
        data: None,
    }
}

#[tool_router]
impl DeliberateThinkingServer {
    /// Deliberate thinking tool for dynamic and reflective problem-solving
    #[tool(
        name = "deliberatethinking",
        description = "A detailed tool for dynamic and reflective problem-solving through thoughts.
This tool helps analyze problems through a flexible thinking process that can adapt and evolve.
Each thought can build on, question, or revise previous insights as understanding deepens.

When to use this tool:
- Breaking down complex problems into steps
- Planning and design with room for revision
- Analysis that might need course correction
- Problems where the full scope might not be clear initially
- Problems that require a multi-step solution
- Tasks that need to maintain context over multiple steps
- Situations where irrelevant information needs to be filtered out

Key features:
- You can adjust total_thoughts up or down as you progress
- You can question or revise previous thoughts
- You can add more thoughts even after reaching what seemed like the end
- You can express uncertainty and explore alternative approaches
- Not every thought needs to build linearly - you can branch or backtrack
- Generates a solution hypothesis
- Verifies the hypothesis based on the Chain of Thought steps
- Repeats the process until satisfied
- Provides a correct answer"
    )]
    pub async fn deliberate_thinking(
        &self,
        Parameters(request): Parameters<DeliberateThinkingRequest>,
    ) -> Result<CallToolResult, McpError> {
        // Validate parameters
        request.validate()?;

        // Convert request to thought data (consumes the request)
        let thought_data = ThoughtData::from(request.clone());

        let mut state = self.state.lock().await;

        // Process the thought based on its type
        if let (Some(branch_from), Some(branch_id)) = (request.branch_from_thought, &request.branch_id) {
            // Branching case: create or add to a branch
            state.handle_branching(branch_from, branch_id.clone(), thought_data);
        } else if let Some(revises) = request.revises_thought {
            // Revision case: update an existing thought
            state.handle_revision(revises, thought_data);
        } else {
            // Regular thought case: add to current history
            state.add_thought(thought_data);
        }

        // Create response
        let response = DeliberateThinkingResponse::new(
            &request,
            state.get_branch_names(),
            state.get_history_length(),
        );

        // Log the thought for debugging
        log_thought_info(&request);

        // Serialize response
        let response_json = serde_json::to_value(response)
            .map_err(create_serialization_error)?;

        Ok(CallToolResult::success(vec![Content::text(
            response_json.to_string(),
        )]))
    }
}

/// Logs information about the current thought
fn log_thought_info(request: &DeliberateThinkingRequest) {
    log::info!(
        "Deliberate Thinking Step {}/{}: {}",
        request.thought_number,
        request.total_thoughts,
        request.thought
    );

    if let Some(ref branch_id) = request.branch_id {
        log::info!("  Branch: {}", branch_id);
    }

    if request.is_revision.unwrap_or(false) {
        if let Some(revises) = request.revises_thought {
            log::info!("  Revision of thought {}", revises);
        }
    }
}

#[tool_handler]
impl ServerHandler for DeliberateThinkingServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            server_info: Implementation {
                name: "deliberate-thinking-rust".to_string(),
                version: "0.1.0".to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let server = DeliberateThinkingServer::new();

    log::info!("Starting Deliberate Thinking MCP Server");

    // Run the server using stdio transport
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_rejects_invalid_thought_number() {
        let request = DeliberateThinkingRequest {
            thought: "test".to_string(),
            next_thought_needed: true,
            thought_number: 0, // Invalid: must be >= 1
            total_thoughts: 5,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        };

        let result = request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_rejects_invalid_total_thoughts() {
        let request = DeliberateThinkingRequest {
            thought: "test".to_string(),
            next_thought_needed: true,
            thought_number: 1,
            total_thoughts: 0, // Invalid: must be >= 1
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        };

        let result = request.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_add_thought_to_state() {
        let mut state = DeliberateThinkingState::default();

        let thought_data = ThoughtData {
            thought: "First thought".to_string(),
            thought_number: 1,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        };

        state.add_thought(thought_data);

        assert_eq!(state.get_history_length(), 1);
        assert_eq!(state.get_current_history()[0].thought, "First thought");
    }

    #[test]
    fn test_branching_creates_new_branch() {
        let mut state = DeliberateThinkingState::default();

        // Add base thought
        let base_thought = ThoughtData {
            thought: "Base thought".to_string(),
            thought_number: 1,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        };
        state.add_thought(base_thought);

        // Create branch
        let branch_thought = ThoughtData {
            thought: "Branch thought".to_string(),
            thought_number: 2,
            total_thoughts: 3,
            next_thought_needed: true,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: Some(1),
            branch_id: Some("alt-path".to_string()),
            needs_more_thoughts: None,
        };

        state.handle_branching(1, "alt-path".to_string(), branch_thought);

        let branch_names = state.get_branch_names();
        assert_eq!(branch_names.len(), 1);
        assert!(branch_names.contains(&"alt-path".to_string()));
        assert_eq!(state.current_branch, Some("alt-path".to_string()));
    }

    #[test]
    fn test_validate_accepts_valid_request() {
        let request = DeliberateThinkingRequest {
            thought: "Valid thought".to_string(),
            next_thought_needed: true,
            thought_number: 1,
            total_thoughts: 5,
            is_revision: None,
            revises_thought: None,
            branch_from_thought: None,
            branch_id: None,
            needs_more_thoughts: None,
        };

        let result = request.validate();
        assert!(result.is_ok());
    }
}