# liberado-tool-helper-mcp — Architecture

## Overview

A lightweight Rust binary that implements the [Model Context Protocol (MCP)](https://modelcontextprotocol.io)
to bridge AI agents (LibreChat, OpenClaw) with the mem0 memory API. It is a **scope-enforcing gateway** —
all memory operations are pre-scoped to one of two isolated stores, and the model never
manages identity or filter parameters.

## Code Layout

```
src/
├── main.rs        # Entry point: tracing init, config load, server dispatch
├── lib.rs         # Re-exports for crate API surface
├── config.rs      # ServerConfig, TransportConfig, env-based config
└── server.rs      # MCP server definition (tools) + mem0 API client
```

### Key files

**`main.rs`** — Bootstraps the server. Reads `ServerConfig::from_env()`, chooses transport
(stdio or HTTP), and calls `server.builder().serve().await`.

**`config.rs`** — Defines `TransportConfig` (Stdio vs Http) and `ServerConfig` (mem0
base URL + transport). `from_env()` reads from environment variables with sensible
defaults. All defaults point at the Docker Compose service name `mem0:8000`.

**`server.rs`** — The bulk of the crate. Contains:
- `LiberadoToolHelperServer` struct wrapping a `reqwest::Client` and `mem0_url`
- 5 `#[tool]` annotated methods (the MCP tool surface)
- Request models: `Message`, `AddRequest`, `SearchRequest`
- All tool calls are pre-scoped via hardcoded `GENERAL_USER_ID` (`"openclaw"`) and
  `PROCEDURAL_AGENT_ID` (`"tool_guidance"`) constants

## Data Flow

```
AI Agent (LibreChat / OpenClaw)
    │
    │  MCP (streamable-http, port 8000)
    ▼
liberado-tool-helper-mcp      ←── pre-scopes user_id / agent_id
    │
    │  HTTP (JSON)
    ▼
mem0 REST API (port 8000)
    │
    ├── POST /memories          (add_memory, save_tool_guidance)
    ├── POST /search            (search_memory, search_tool_guidance)
    └── DELETE /memories/{id}   (delete_memory)
    │
    ▼
librechat-vectordb (pgvector / mem0db)
```

## Memory Scopes

Two fully isolated stores — invisible to the model:

| Scope | Identity | Purpose | Tools |
|---|---|---|---|
| General | `user_id="openclaw"` | Episodic: facts, history, preferences, past conversations | `search_memory`, `add_memory` |
| Procedural | `agent_id="tool_guidance"` | How-to: tool selection, proven workflows, task-to-tool mappings | `search_tool_guidance`, `save_tool_guidance` |

## MCP Transport

The server supports two transports selectable via `MCP_TRANSPORT`:

| Transport | Use case |
|---|---|
| `stdio` | Local agent use (e.g. Claude Code, OpenClaw binary MCP) |
| `http` (default) | Deployed container for LibreChat / OpenClaw remote MCP |

In Docker, the default is `http` on port 8000. The service is registered in
LibreChat's `librechat.yaml` as `streamable-http` and in OpenClaw's `openclaw.json`
as `streamable-http`. TurboMCP serves both `/mcp` (streamable-http) and `/sse`
on the same port.

## Dependencies

- **turbomcp** — MCP SDK (HTTP + stdio transports, macro-based tool definitions)
- **reqwest** — HTTP client for mem0 API calls
- **tokio** — Async runtime
- **serde** / **serde_json** — Serialization for request construction and response handling
- **tracing** / **tracing-subscriber** — Structured logging

## Testing

```bash
cd services/liberado-tool-helper-mcp
cargo test    # 8 unit tests covering config defaults, env parsing, transport selection
```

## Deployment

```bash
docker compose --env-file .env -f compose/docker-compose.ai.yml build mem0-mcp
docker compose --env-file .env -f compose/docker-compose.ai.yml up -d mem0-mcp
```

The container name is `mem0-mcp` for backward compatibility — no URL changes needed
in LibreChat or OpenClaw configs. See the plan in `mem0-procedural-tools-plan.md`
for the seed-data procedure to bootstrap the guidance store.
