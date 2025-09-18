# Deliberate Thinking MCP Server

A structured thinking [Model Context Protocol](https://modelcontextprotocol.io/docs/getting-started/intro)
tool for AI assistants that breaks down complex problems into sequential,
revisable thoughts.

## Notes

* This is based on the [Sequential Thinking](https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking)
MCP
* The project goal is merely to be useful to _me_ and _my_ work. It's
  easy to fork for your work.
* This is written in Rust merely for low-latency start-up times
  and for fun.


## Quick Start

### Install

```bash
# Clone the repository
git clone https://github.com/kljensen/deliberate-thinking.git
cd deliberate-thinking
cargo build --release
```

### Adding deliberate thinking to your AI assistant

You can find instructions for your assistants at these links:
- [Claude Code MCP instructions](https://docs.claude.com/en/docs/claude-code/mcp)
- [OpenAI Codex MCP instructions](https://github.com/openai/codex/blob/main/docs/advanced.md#model-context-protocol-mcp)
- [GitHub Copilot MCP instructions](https://docs.github.com/en/copilot/how-tos/provide-context/use-mcp/extend-copilot-chat-with-mcp)

For Claude Code, I often have a `.mcp.json` file in my working directory with the following content.

```json
{
  "mcpServers": {
    "deliberate-thinking": {
      "command": "/your/path/to/deliberate-thinking-server",
      "args": []
    }
}
```

## License

The [Unlicense](https://unlicense.org/).
