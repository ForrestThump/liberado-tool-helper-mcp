use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use turbomcp::prelude::*;

use crate::config::ServerConfig;

const GENERAL_USER_ID: &str = "openclaw";
const PROCEDURAL_AGENT_ID: &str = "tool_guidance";

#[derive(Serialize)]
pub(crate) struct Message {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Serialize)]
pub(crate) struct AddRequest {
    pub(crate) messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) metadata: Option<HashMap<String, Value>>,
}

#[derive(Serialize)]
pub(crate) struct SearchRequest {
    pub(crate) query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) agent_id: Option<String>,
}

#[derive(Clone)]
pub struct LiberadoToolHelperServer {
    mem0_url: Arc<String>,
    client: Client,
}

impl LiberadoToolHelperServer {
    pub fn new(config: ServerConfig) -> Self {
        let mem0_url = config.mem0_url.trim_end_matches('/').to_string();
        Self {
            mem0_url: Arc::new(mem0_url),
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    fn build_search_memory_request(query: String) -> (&'static str, SearchRequest) {
        (
            "/search",
            SearchRequest {
                query,
                user_id: Some(GENERAL_USER_ID.into()),
                agent_id: None,
            },
        )
    }

    fn build_add_memory_request(content: String) -> (&'static str, AddRequest) {
        (
            "/memories",
            AddRequest {
                messages: vec![Message {
                    role: "user".into(),
                    content,
                }],
                user_id: Some(GENERAL_USER_ID.into()),
                agent_id: None,
                metadata: None,
            },
        )
    }

    fn build_search_tool_guidance_request(query: String) -> (&'static str, SearchRequest) {
        (
            "/search",
            SearchRequest {
                query,
                user_id: None,
                agent_id: Some(PROCEDURAL_AGENT_ID.into()),
            },
        )
    }

    fn build_save_tool_guidance_request(
        description: String,
        task_type: Option<String>,
        tools_used: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> (&'static str, AddRequest) {
        let mut meta: HashMap<String, Value> = HashMap::new();
        meta.insert("memory_type".into(), json!("tool_selection"));
        meta.insert("success".into(), json!(true));
        if let Some(tt) = task_type {
            meta.insert("task_type".into(), json!(tt));
        }
        if let Some(tu) = tools_used {
            meta.insert("tools_used".into(), json!(tu));
        }
        if let Some(t) = tags {
            meta.insert("tags".into(), json!(t));
        }

        (
            "/memories",
            AddRequest {
                messages: vec![Message {
                    role: "user".into(),
                    content: description,
                }],
                user_id: None,
                agent_id: Some(PROCEDURAL_AGENT_ID.into()),
                metadata: Some(meta),
            },
        )
    }

    fn build_delete_memory_url(&self, memory_id: &str) -> McpResult<String> {
        if !memory_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(McpError::invalid_params(
                "memory_id contains invalid characters",
            ));
        }
        Ok(format!("{}/memories/{}", self.mem0_url, memory_id))
    }

    #[cfg(test)]
    pub(crate) fn mem0_url(&self) -> &str {
        &self.mem0_url
    }

    pub async fn post_json<T: Serialize>(&self, path: &str, body: &T) -> McpResult<String> {
        let url = format!("{}{}", self.mem0_url, path);
        let resp = self
            .client
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| McpError::internal(format!("HTTP request to mem0 failed: {}", e)))?;
        let status = resp.status();
        let response_text = resp
            .text()
            .await
            .map_err(|e| McpError::internal(format!("Failed to read mem0 response body: {}", e)))?;
        if !status.is_success() {
            return Err(McpError::internal(format!(
                "mem0 API error ({}): {}",
                status, response_text
            )));
        }
        Ok(response_text)
    }

    pub async fn delete_via_http(&self, url: &str) -> McpResult<String> {
        let resp = self
            .client
            .delete(url)
            .send()
            .await
            .map_err(|e| McpError::internal(format!("HTTP request to mem0 failed: {}", e)))?;
        let status = resp.status();
        let response_text = resp
            .text()
            .await
            .map_err(|e| McpError::internal(format!("Failed to read mem0 response body: {}", e)))?;
        if !status.is_success() {
            return Err(McpError::internal(format!(
                "mem0 API error ({}): {}",
                status, response_text
            )));
        }
        Ok(response_text)
    }
}

#[server(name = "liberado-tool-helper-mcp", version = "0.1.0")]
impl LiberadoToolHelperServer {
    #[tool("Search general memories: facts, history, preferences, past conversations. Use for personal context about the user or prior session details.")]
    async fn search_memory(&self, query: String) -> McpResult<String> {
        let (path, req) = Self::build_search_memory_request(query);
        self.post_json(path, &req).await
    }

    #[tool("Save a general memory: a fact, preference, or event worth remembering for future sessions.")]
    async fn add_memory(&self, content: String) -> McpResult<String> {
        let (path, req) = Self::build_add_memory_request(content);
        self.post_json(path, &req).await
    }

    #[tool("Look up prescriptive tool guidance: which tool to use for a given task type, and how to structure the work. Call this when the right tool is not immediately obvious from tool descriptions alone. Returns directives like 'Use X for Y tasks'.")]
    async fn search_tool_guidance(&self, query: String) -> McpResult<String> {
        let (path, req) = Self::build_search_tool_guidance_request(query);
        self.post_json(path, &req).await
    }

    #[tool("Save prescriptive tool guidance for future reference. Write guidance as a directive: 'Use [tool] for [task]' — not as a log of what happened. Call this after figuring out the right tool for a non-obvious task so future instances skip the discovery step. Provide: the guidance as a directive (guidance), the task_type (e.g. 'shopping_list'), which tools to use (tools_used), and optional tags.")]
    async fn save_tool_guidance(
        &self,
        guidance: String,
        task_type: Option<String>,
        tools_used: Option<Vec<String>>,
        tags: Option<Vec<String>>,
    ) -> McpResult<String> {
        let (path, req) =
            Self::build_save_tool_guidance_request(guidance, task_type, tools_used, tags);
        self.post_json(path, &req).await
    }

    #[tool("Delete a specific memory by its ID. Use only when a memory is wrong or outdated.")]
    async fn delete_memory(&self, memory_id: String) -> McpResult<String> {
        let url = self.build_delete_memory_url(&memory_id)?;
        self.delete_via_http(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerConfig, TransportConfig};

    #[test]
    fn test_general_user_id_constant() {
        assert_eq!(GENERAL_USER_ID, "openclaw");
    }

    #[test]
    fn test_procedural_agent_id_constant() {
        assert_eq!(PROCEDURAL_AGENT_ID, "tool_guidance");
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message {
            role: "user".into(),
            content: "hello".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn test_add_request_minimal() {
        let req = AddRequest {
            messages: vec![Message {
                role: "user".into(),
                content: "test".into(),
            }],
            user_id: Some("openclaw".into()),
            agent_id: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "test");
        assert_eq!(json["user_id"], "openclaw");
        assert!(json.get("agent_id").is_none());
        assert!(json.get("metadata").is_none());
    }

    #[test]
    fn test_add_request_with_agent_and_metadata() {
        let mut meta = HashMap::new();
        meta.insert("key".into(), json!("val"));
        let req = AddRequest {
            messages: vec![Message {
                role: "user".into(),
                content: "test".into(),
            }],
            user_id: None,
            agent_id: Some("agent_x".into()),
            metadata: Some(meta),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("user_id").is_none());
        assert_eq!(json["agent_id"], "agent_x");
        assert_eq!(json["metadata"]["key"], "val");
    }

    #[test]
    fn test_search_request_with_user_id() {
        let req = SearchRequest {
            query: "find this".into(),
            user_id: Some("openclaw".into()),
            agent_id: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["query"], "find this");
        assert_eq!(json["user_id"], "openclaw");
        assert!(json.get("agent_id").is_none());
    }

    #[test]
    fn test_search_request_with_agent_id() {
        let req = SearchRequest {
            query: "tool help".into(),
            user_id: None,
            agent_id: Some("tool_guidance".into()),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["query"], "tool help");
        assert!(json.get("user_id").is_none());
        assert_eq!(json["agent_id"], "tool_guidance");
    }

    #[test]
    fn test_build_search_memory_request() {
        let (path, req) = LiberadoToolHelperServer::build_search_memory_request("my query".into());
        assert_eq!(path, "/search");
        assert_eq!(req.query, "my query");
        assert_eq!(req.user_id, Some("openclaw".into()));
        assert_eq!(req.agent_id, None);
    }

    #[test]
    fn test_build_add_memory_request() {
        let (path, req) =
            LiberadoToolHelperServer::build_add_memory_request("remember this".into());
        assert_eq!(path, "/memories");
        assert_eq!(req.messages[0].content, "remember this");
        assert_eq!(req.messages[0].role, "user");
        assert_eq!(req.user_id, Some("openclaw".into()));
        assert_eq!(req.agent_id, None);
        assert!(req.metadata.is_none());
    }

    #[test]
    fn test_build_search_tool_guidance_request() {
        let (path, req) =
            LiberadoToolHelperServer::build_search_tool_guidance_request("which tool".into());
        assert_eq!(path, "/search");
        assert_eq!(req.query, "which tool");
        assert_eq!(req.user_id, None);
        assert_eq!(req.agent_id, Some("tool_guidance".into()));
    }

    #[test]
    fn test_build_save_tool_guidance_request_all_fields() {
        let (path, req) = LiberadoToolHelperServer::build_save_tool_guidance_request(
            "used X for Y".into(),
            Some("shopping".into()),
            Some(vec!["tool_a".into(), "tool_b".into()]),
            Some(vec!["tag1".into()]),
        );
        assert_eq!(path, "/memories");
        assert_eq!(req.messages[0].content, "used X for Y");
        assert_eq!(req.user_id, None);
        assert_eq!(req.agent_id, Some("tool_guidance".into()));
        let meta = req.metadata.unwrap();
        assert_eq!(meta["memory_type"], "tool_selection");
        assert_eq!(meta["success"], true);
        assert_eq!(meta["task_type"], "shopping");
        assert_eq!(meta["tools_used"], json!(["tool_a", "tool_b"]));
        assert_eq!(meta["tags"], json!(["tag1"]));
    }

    #[test]
    fn test_build_save_tool_guidance_request_minimal() {
        let (path, req) = LiberadoToolHelperServer::build_save_tool_guidance_request(
            "basic".into(),
            None,
            None,
            None,
        );
        assert_eq!(path, "/memories");
        assert_eq!(req.messages[0].content, "basic");
        assert_eq!(req.agent_id, Some("tool_guidance".into()));
        let meta = req.metadata.unwrap();
        assert_eq!(meta["memory_type"], "tool_selection");
        assert_eq!(meta["success"], true);
        assert!(!meta.contains_key("task_type"));
        assert!(!meta.contains_key("tools_used"));
        assert!(!meta.contains_key("tags"));
    }

    #[test]
    fn test_build_delete_memory_url() {
        let config = ServerConfig {
            mem0_url: "http://mem0:8000".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        let url = server.build_delete_memory_url("abc-123").unwrap();
        assert_eq!(url, "http://mem0:8000/memories/abc-123");
    }

    #[test]
    fn test_build_delete_memory_url_rejects_path_traversal() {
        let config = ServerConfig {
            mem0_url: "http://mem0:8000".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        assert!(server.build_delete_memory_url("../search").is_err());
        assert!(server.build_delete_memory_url("abc/def").is_err());
        assert!(server.build_delete_memory_url("id with spaces").is_err());
    }

    #[test]
    fn test_mem0_url_accessor() {
        let config = ServerConfig {
            mem0_url: "http://test:9999".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        assert_eq!(server.mem0_url(), "http://test:9999");
    }

    #[test]
    fn test_new_strips_trailing_slash_from_mem0_url() {
        let config = ServerConfig {
            mem0_url: "http://mem0:8000/".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        assert_eq!(server.mem0_url(), "http://mem0:8000");
    }

    // Tool-level tests: verify each MCP tool sends the correct scope and endpoint.
    // These live here (not in tests/) because the tool methods are private.

    #[tokio::test]
    async fn test_search_memory_uses_general_scope() {
        let mock = wiremock::MockServer::start().await;
        let expected = serde_json::json!({"query": "my fact", "user_id": "openclaw"});
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/search"))
            .and(wiremock::matchers::body_json(&expected))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"results": []})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        server.search_memory("my fact".into()).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_memory_uses_general_scope() {
        let mock = wiremock::MockServer::start().await;
        let expected = serde_json::json!({
            "messages": [{"role": "user", "content": "user prefers dark mode"}],
            "user_id": "openclaw"
        });
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/memories"))
            .and(wiremock::matchers::body_json(&expected))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "m1"})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        server
            .add_memory("user prefers dark mode".into())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_search_tool_guidance_uses_procedural_scope() {
        let mock = wiremock::MockServer::start().await;
        let expected = serde_json::json!({"query": "shopping list", "agent_id": "tool_guidance"});
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/search"))
            .and(wiremock::matchers::body_json(&expected))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"results": []})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        server
            .search_tool_guidance("shopping list".into())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_save_tool_guidance_uses_procedural_scope() {
        let mock = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/memories"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "g1"})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        let result = server
            .save_tool_guidance(
                "Use caldav-mcp for calendar tasks".into(),
                Some("calendar".into()),
                Some(vec!["caldav-mcp".into()]),
                None,
            )
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["id"], "g1");
    }

    #[tokio::test]
    async fn test_delete_memory_sends_delete_request() {
        let mock = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .and(wiremock::matchers::path("/memories/abc-123"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"deleted": true})),
            )
            .expect(1)
            .mount(&mock)
            .await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        server.delete_memory("abc-123".into()).await.unwrap();
    }

    #[tokio::test]
    async fn test_delete_memory_rejects_invalid_id() {
        let mock = wiremock::MockServer::start().await;
        let server = LiberadoToolHelperServer::new(ServerConfig {
            mem0_url: mock.uri(),
            transport: TransportConfig::Stdio,
        });
        assert!(server.delete_memory("../search".into()).await.is_err());
        assert!(server.delete_memory("a/b".into()).await.is_err());
    }

    #[tokio::test]
    async fn test_post_json_connection_refused() {
        let config = ServerConfig {
            mem0_url: "http://127.0.0.1:1".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        let body = serde_json::json!({"test": true});
        let result = server.post_json("/search", &body).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("connection refused")
                || msg.contains("Connection refused")
                || msg.contains("failed")
        );
    }

    #[tokio::test]
    async fn test_delete_via_http_connection_refused() {
        let config = ServerConfig {
            mem0_url: "http://127.0.0.1:1".into(),
            transport: TransportConfig::Stdio,
        };
        let server = LiberadoToolHelperServer::new(config);
        let result = server
            .delete_via_http("http://127.0.0.1:1/memories/x")
            .await;
        assert!(result.is_err());
    }
}
