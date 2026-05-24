use tracing;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum TransportConfig {
    #[default]
    Stdio,
    Http {
        host: String,
        port: u16,
    },
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub mem0_url: String,
    pub transport: TransportConfig,
}

fn default_http_port() -> u16 {
    8000
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            mem0_url: "http://mem0:8000".to_string(),
            transport: TransportConfig::Stdio,
        }
    }
}

impl ServerConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(val) = std::env::var("MEM0_URL") {
            config.mem0_url = val;
        }
        if let Ok(val) = std::env::var("MCP_TRANSPORT") {
            match val.to_lowercase().as_str() {
                "http" => {
                    let host =
                        std::env::var("MCP_HTTP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
                    let port = match std::env::var("MCP_HTTP_PORT") {
                        Ok(v) => v.parse::<u16>().unwrap_or_else(|_| {
                            tracing::warn!(
                                "invalid MCP_HTTP_PORT '{}', falling back to default {}",
                                v,
                                default_http_port()
                            );
                            default_http_port()
                        }),
                        Err(_) => default_http_port(),
                    };
                    config.transport = TransportConfig::Http { host, port };
                }
                other => {
                    tracing::warn!(
                        "unrecognized MCP_TRANSPORT '{}', falling back to Stdio",
                        other
                    );
                    config.transport = TransportConfig::Stdio;
                }
            }
        }

        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_default_transport_is_stdio() {
        let config = ServerConfig::default();
        assert_eq!(config.transport, TransportConfig::Stdio);
    }

    #[test]
    #[serial]
    fn test_from_env_defaults() {
        std::env::remove_var("MEM0_URL");
        std::env::remove_var("MCP_TRANSPORT");
        let config = ServerConfig::from_env();
        assert_eq!(config.mem0_url, "http://mem0:8000");
        assert_eq!(config.transport, TransportConfig::Stdio);
    }

    #[test]
    #[serial]
    fn test_from_env_custom_mem0_url() {
        std::env::set_var("MEM0_URL", "http://custom-mem0:9000");
        std::env::remove_var("MCP_TRANSPORT");
        let config = ServerConfig::from_env();
        assert_eq!(config.mem0_url, "http://custom-mem0:9000");
        assert_eq!(config.transport, TransportConfig::Stdio);
    }

    #[test]
    #[serial]
    fn test_from_env_http_transport_defaults() {
        std::env::set_var("MCP_TRANSPORT", "http");
        std::env::remove_var("MCP_HTTP_HOST");
        std::env::remove_var("MCP_HTTP_PORT");
        let config = ServerConfig::from_env();
        assert_eq!(
            config.transport,
            TransportConfig::Http {
                host: "0.0.0.0".to_string(),
                port: 8000,
            }
        );
    }

    #[test]
    #[serial]
    fn test_from_env_http_transport_custom() {
        std::env::set_var("MCP_TRANSPORT", "http");
        std::env::set_var("MCP_HTTP_HOST", "127.0.0.1");
        std::env::set_var("MCP_HTTP_PORT", "9000");
        let config = ServerConfig::from_env();
        assert_eq!(
            config.transport,
            TransportConfig::Http {
                host: "127.0.0.1".to_string(),
                port: 9000,
            }
        );
    }

    #[test]
    #[serial]
    fn test_from_env_invalid_transport_falls_back_to_stdio() {
        std::env::set_var("MCP_TRANSPORT", "tcp");
        let config = ServerConfig::from_env();
        assert_eq!(config.transport, TransportConfig::Stdio);
    }

    #[test]
    #[serial]
    fn test_from_env_http_transport_case_insensitive() {
        std::env::set_var("MCP_TRANSPORT", "HTTP");
        std::env::remove_var("MCP_HTTP_HOST");
        std::env::remove_var("MCP_HTTP_PORT");
        let config = ServerConfig::from_env();
        assert_eq!(
            config.transport,
            TransportConfig::Http {
                host: "0.0.0.0".to_string(),
                port: 8000,
            }
        );
    }

    #[test]
    #[serial]
    fn test_from_env_invalid_port_falls_back_to_default() {
        std::env::set_var("MCP_TRANSPORT", "http");
        std::env::set_var("MCP_HTTP_PORT", "not-a-port");
        std::env::remove_var("MCP_HTTP_HOST");
        let config = ServerConfig::from_env();
        assert_eq!(
            config.transport,
            TransportConfig::Http {
                host: "0.0.0.0".to_string(),
                port: 8000,
            }
        );
    }
}
