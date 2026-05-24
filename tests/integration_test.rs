use liberado_tool_helper_mcp::{LiberadoToolHelperServer, ServerConfig, TransportConfig};
use serde_json::json;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn make_server(config: ServerConfig) -> LiberadoToolHelperServer {
    LiberadoToolHelperServer::new(config)
}

#[tokio::test]
async fn test_post_json_sends_request_and_receives_response() {
    let mock = MockServer::start().await;

    let expected_body = json!({"query": "hello", "user_id": "openclaw"});
    Mock::given(method("POST"))
        .and(path("/search"))
        .and(body_json(&expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"results": []})))
        .expect(1)
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let result = server.post_json("/search", &expected_body).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["results"], json!([]));
}

#[tokio::test]
async fn test_post_json_to_memories() {
    let mock = MockServer::start().await;

    let payload = json!({
        "messages": [{"role": "user", "content": "remember this"}],
        "user_id": "openclaw"
    });
    let response_body = json!({"id": "mem-1", "message": "created"});
    Mock::given(method("POST"))
        .and(path("/memories"))
        .and(body_json(&payload))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .expect(1)
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let result = server.post_json("/memories", &payload).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["id"], "mem-1");
}

#[tokio::test]
async fn test_delete_via_http() {
    let mock = MockServer::start().await;

    let delete_path = format!("/memories/{}", "mem-42");
    Mock::given(method("DELETE"))
        .and(path(&delete_path))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
        .expect(1)
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let url = format!("{}/memories/{}", mock.uri(), "mem-42");
    let result = server.delete_via_http(&url).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["deleted"], true);
}

#[tokio::test]
async fn test_post_json_handles_500_error() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let result = server.post_json("/search", &json!({"query": "test"})).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("500"));
    assert!(err.contains("Internal Server Error"));
}

#[tokio::test]
async fn test_post_json_handles_404_error() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let result = server.post_json("/nonexistent", &json!({"x": 1})).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("404"));
}

#[tokio::test]
async fn test_delete_via_http_handles_500_error() {
    let mock = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/memories/bad-id"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Delete failed"))
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let url = format!("{}/memories/{}", mock.uri(), "bad-id");
    let result = server.delete_via_http(&url).await;
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(err.contains("500"));
}

#[tokio::test]
async fn test_server_with_different_base_urls() {
    let mock_a = MockServer::start().await;
    let mock_b = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"source": "A"})))
        .expect(1)
        .mount(&mock_a)
        .await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"source": "B"})))
        .expect(1)
        .mount(&mock_b)
        .await;

    let config_a = ServerConfig {
        mem0_url: mock_a.uri(),
        transport: TransportConfig::Stdio,
    };
    let server_a = make_server(config_a);

    let config_b = ServerConfig {
        mem0_url: mock_b.uri(),
        transport: TransportConfig::Stdio,
    };
    let server_b = make_server(config_b);

    let result_a = server_a
        .post_json("/search", &json!({"query": "test"}))
        .await
        .unwrap();
    assert!(result_a.contains("A"));

    let result_b = server_b
        .post_json("/search", &json!({"query": "test"}))
        .await
        .unwrap();
    assert!(result_b.contains("B"));
}

#[tokio::test]
async fn test_post_json_sends_correct_content_type() {
    let mock = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
        .expect(1)
        .mount(&mock)
        .await;

    let config = ServerConfig {
        mem0_url: mock.uri(),
        transport: TransportConfig::Stdio,
    };
    let server = make_server(config);

    let result = server
        .post_json("/search", &json!({"query": "x"}))
        .await
        .unwrap();
    assert!(result.contains("ok"));
}
