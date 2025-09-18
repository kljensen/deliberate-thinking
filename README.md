# Deliberate Thinking MCP Server (Rust)

A Rust implementation of the Deliberate Thinking MCP server that matches the functionality of the TypeScript version. This server provides a structured approach to problem-solving through dynamic and reflective thinking processes.

## Features

- **Dynamic Thinking Process**: Break down complex problems into manageable sequential steps
- **Thought Revision**: Revise and refine previous thoughts as understanding deepens
- **Branching Logic**: Explore alternative reasoning paths through thought branches
- **State Management**: Track thought history, branches, and progression
- **Parameter Validation**: Comprehensive input validation with proper error handling
- **JSON Response Format**: Compatible response format matching TypeScript implementation

## Tool Parameters

The `deliberatethinking` tool accepts the following parameters:

### Required Parameters
- `thought` (string): Current thinking step content
- `nextThoughtNeeded` (boolean): Whether another thought step is needed
- `thoughtNumber` (u32): Current thought number (minimum 1)
- `totalThoughts` (u32): Estimated total thoughts needed (minimum 1)

### Optional Parameters
- `isRevision` (boolean): Whether this revises previous thinking
- `revisesThought` (u32): Which thought number is being reconsidered
- `branchFromThought` (u32): Branching point thought number
- `branchId` (string): Branch identifier
- `needsMoreThoughts` (boolean): If more thoughts are needed

## Response Format

```json
{
  "thoughtNumber": number,
  "totalThoughts": number,
  "nextThoughtNeeded": boolean,
  "branches": array,
  "thoughtHistoryLength": number
}
```

## Building and Running

### Prerequisites
- Rust 1.70 or higher
- Cargo

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run
```

Or run the binary directly:
```bash
./target/debug/deliberate-thinking-server
```

## Claude Desktop Integration

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "deliberate-thinking-rust": {
      "command": "/path/to/deliberate-thinking-rust/target/debug/deliberate-thinking-server",
      "args": []
    }
  }
}
```

## Architecture

- **Server Handler**: Implements the MCP `ServerHandler` trait
- **Tool Router**: Uses `#[tool_router]` macro for automatic tool registration
- **State Management**: Thread-safe state management with `Arc<Mutex<>>`
- **Transport**: Uses stdio transport for communication
- **Validation**: Input validation with proper MCP error responses

## Implementation Details

- Built with the official `rmcp` crate (v0.6.4)
- Async/await throughout using tokio runtime
- Comprehensive error handling with proper MCP error codes
- Memory-safe state management
- Efficient branching and revision handling

## Usage Example

The deliberate thinking tool helps break down complex problems:

1. **Initial Thought**: Start with thought #1, set total thoughts estimate
2. **Progressive Thinking**: Continue with subsequent thoughts
3. **Revision**: Revise previous thoughts by setting `isRevision: true` and `revisesThought`
4. **Branching**: Create alternative reasoning paths with `branchFromThought` and `branchId`
5. **Dynamic Adjustment**: Adjust `totalThoughts` as understanding evolves

This implementation provides a practical, working solution that efficiently handles structured thinking processes for AI assistants.