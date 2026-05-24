# liberado-tool-helper-mcp

Rust MCP server that wraps the [mem0](https://mem0.ai) memory API with purpose-built,
scope-hardcoded tools. Exposes two isolated memory stores — **general** (user facts,
history, preferences) and **procedural** (tool-selection guidance, proven workflows) —
so AI agents never need to manage `user_id`, `agent_id`, or filter parameters.

## Tools

| Tool | Scope | Description |
|---|---|---|
| `search_memory` | general | Search episodic memories: facts, history, past conversations |
| `add_memory` | general | Save a fact, preference, or event for future sessions |
| `search_tool_guidance` | procedural | Find the right tool or workflow for a task |
| `save_tool_guidance` | procedural | Save a prescriptive directive for future instances (guidance, task_type, tools_used, tags) |
| `delete_memory` | any | Delete a specific memory by ID |

## Environment

| Variable | Default | Description |
|---|---|---|
| `MEM0_URL` | `http://mem0:8000` | mem0 REST API base URL |
| `MCP_TRANSPORT` | `stdio` (code) / `http` (deployed via env file) | Transport: `stdio` or `http` |
| `MCP_HTTP_HOST` | `0.0.0.0` | HTTP bind address |
| `MCP_HTTP_PORT` | `8000` | HTTP port |

## Motivation

The previous Python `mem0-mcp` wrapper exposed raw CRUD tools that required the model to
pass `user_id="openclaw"` on every call. This meant the model could accidentally scope
memories incorrectly, and procedural/tool-selection knowledge was mixed into the same
store as general episodic memories. This service eliminates both problems by hardcoding
the scope boundaries inside the MCP layer.

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for code layout, data flow, and design decisions.
