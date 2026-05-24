use tracing::info;
use turbomcp::prelude::*;

use liberado_tool_helper_mcp::{LiberadoToolHelperServer, ServerConfig, TransportConfig};

fn init_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init();
}

fn log_startup(config: &ServerConfig) {
    info!(
        mem0_url = %config.mem0_url,
        "liberado-tool-helper-mcp starting"
    );
}

async fn run(config: ServerConfig) {
    log_startup(&config);
    let transport_cfg = config.transport.clone();
    let server = LiberadoToolHelperServer::new(config);

    let builder = server.builder().with_protocol(ProtocolConfig {
        allow_fallback: true,
        ..Default::default()
    });

    let server = match transport_cfg {
        TransportConfig::Stdio => builder.transport(turbomcp::Transport::stdio()),
        TransportConfig::Http { host, port } => {
            let addr = format!("{host}:{port}");
            info!(addr = %addr, "HTTP transport enabled");
            builder
                .transport(turbomcp::Transport::http(addr))
                .allow_any_origin(true)
        }
    };

    server.serve().await.unwrap();
}

#[tokio::main]
async fn main() {
    init_tracing();
    let config = ServerConfig::from_env();
    run(config).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_init_tracing_does_not_panic() {
        init_tracing();
    }

    #[test]
    fn test_log_startup_does_not_panic() {
        let config = ServerConfig {
            mem0_url: "http://mem0:8000".to_string(),
            transport: TransportConfig::Stdio,
        };
        log_startup(&config);
    }

    #[tokio::test]
    async fn test_run_http_binds_and_cancels() {
        let config = ServerConfig {
            mem0_url: "http://mem0:8000".to_string(),
            transport: TransportConfig::Http {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
        };

        let handle = tokio::spawn(async {
            run(config).await;
        });

        let start = std::time::Instant::now();
        while !handle.is_finished() && start.elapsed() < std::time::Duration::from_secs(5) {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(!handle.is_finished());
        handle.abort();
        let result = handle.await;
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_main_binds_and_cancels() {
        std::env::set_var("MEM0_URL", "http://mem0:8000");
        std::env::set_var("MCP_TRANSPORT", "http");
        std::env::set_var("MCP_HTTP_HOST", "127.0.0.1");
        std::env::set_var("MCP_HTTP_PORT", "0");

        let handle = std::thread::spawn(|| {
            main();
        });

        let start = std::time::Instant::now();
        while !handle.is_finished() && start.elapsed() < std::time::Duration::from_secs(5) {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        // main() creates its own tokio runtime, so we can't directly abort.
        // Just verify it started without panicking by checking the thread is alive.
        assert!(!handle.is_finished());
    }
}
