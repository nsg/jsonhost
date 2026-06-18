use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_stathost_url")]
    pub stathost_url: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8090
}

fn default_stathost_url() -> String {
    "http://localhost:8080".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            stathost_url: default_stathost_url(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
}

impl AppConfig {
    pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let path = path.unwrap_or(Path::new("jsonhost.toml"));
        let mut config = if path.exists() {
            let content = std::fs::read_to_string(path)?;
            toml::from_str(&content)?
        } else {
            AppConfig {
                server: ServerConfig::default(),
            }
        };
        config.apply_env();
        Ok(config)
    }

    /// Environment variables override file/default values, mainly for containers.
    fn apply_env(&mut self) {
        if let Ok(v) = std::env::var("JSONHOST_STATHOST_URL") {
            self.server.stathost_url = v;
        }
        if let Ok(v) = std::env::var("JSONHOST_HOST") {
            self.server.host = v;
        }
        if let Ok(v) = std::env::var("JSONHOST_PORT")
            && let Ok(p) = v.parse()
        {
            self.server.port = p;
        }
    }
}
