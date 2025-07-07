use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

use crate::error::{DocFusionError, DocFusionResult};

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: String,
    pub max_connections: usize,
    pub min_connections: usize,
    pub connection_timeout: u64,
    pub idle_timeout: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            user: "postgres".to_string(),
            password: "yourpassword".to_string(),
            database: "docfusiondb".to_string(),
            max_connections: 10,
            min_connections: 1,
            connection_timeout: 30,
            idle_timeout: 600,
        }
    }
}

impl DatabaseConfig {
    /// Build a connection string from the configuration
    pub fn connection_string(&self) -> String {
        format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.user, self.password, self.database
        )
    }
    
    /// Create configuration from environment variables
    pub fn from_env() -> DocFusionResult<Self> {
        if let Ok(url) = env::var("DATABASE_URL") {
            Self::from_url(&url)
        } else {
            Ok(Self {
                host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                port: env::var("DB_PORT")
                    .unwrap_or_else(|_| "5432".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid DB_PORT"))?,
                user: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                password: env::var("DB_PASSWORD").unwrap_or_else(|_| "yourpassword".to_string()),
                database: env::var("DB_NAME").unwrap_or_else(|_| "docfusiondb".to_string()),
                max_connections: env::var("DB_MAX_CONNECTIONS")
                    .unwrap_or_else(|_| "10".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid DB_MAX_CONNECTIONS"))?,
                min_connections: env::var("DB_MIN_CONNECTIONS")
                    .unwrap_or_else(|_| "1".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid DB_MIN_CONNECTIONS"))?,
                connection_timeout: env::var("DB_CONNECTION_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid DB_CONNECTION_TIMEOUT"))?,
                idle_timeout: env::var("DB_IDLE_TIMEOUT")
                    .unwrap_or_else(|_| "600".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid DB_IDLE_TIMEOUT"))?,
            })
        }
    }
    
    /// Parse a PostgreSQL URL into configuration
    pub fn from_url(url: &str) -> DocFusionResult<Self> {
        // Simple URL parsing - in production, consider using a proper URL parser
        if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
            return Err(DocFusionError::config("Invalid PostgreSQL URL format"));
        }
        
        let url = url.strip_prefix("postgres://").or_else(|| url.strip_prefix("postgresql://")).unwrap();
        let parts: Vec<&str> = url.split('@').collect();
        
        if parts.len() != 2 {
            return Err(DocFusionError::config("Invalid PostgreSQL URL format"));
        }
        
        let credentials: Vec<&str> = parts[0].split(':').collect();
        if credentials.len() != 2 {
            return Err(DocFusionError::config("Invalid PostgreSQL URL credentials"));
        }
        
        let host_db: Vec<&str> = parts[1].split('/').collect();
        if host_db.len() != 2 {
            return Err(DocFusionError::config("Invalid PostgreSQL URL host/database"));
        }
        
        let host_port: Vec<&str> = host_db[0].split(':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1].parse().map_err(|_| DocFusionError::config("Invalid port in URL"))?
        } else {
            5432
        };
        
        Ok(Self {
            host,
            port,
            user: credentials[0].to_string(),
            password: credentials[1].to_string(),
            database: host_db[1].to_string(),
            ..Default::default()
        })
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: num_cpus::get(),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
    pub file: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            file: None,
        }
    }
}

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub logging: LogConfig,
    pub auth: AuthConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig::default(),
            server: ServerConfig::default(),
            logging: LogConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> DocFusionResult<Self> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
    
    /// Load configuration from environment variables
    pub fn from_env() -> DocFusionResult<Self> {
        Ok(Self {
            database: DatabaseConfig::from_env()?,
            server: ServerConfig {
                host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("SERVER_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid SERVER_PORT"))?,
                workers: env::var("SERVER_WORKERS")
                    .unwrap_or_else(|_| num_cpus::get().to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid SERVER_WORKERS"))?,
            },
            logging: LogConfig {
                level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
                format: env::var("LOG_FORMAT").unwrap_or_else(|_| "json".to_string()),
                file: env::var("LOG_FILE").ok(),
            },
            auth: AuthConfig {
                enabled: env::var("AUTH_ENABLED")
                    .unwrap_or_else(|_| "false".to_string())
                    .parse()
                    .map_err(|_| DocFusionError::config("Invalid AUTH_ENABLED"))?,
                api_key: env::var("API_KEY").ok(),
            },
        })
    }
    
    /// Load configuration with fallback order: file -> env -> defaults
    pub fn load() -> DocFusionResult<Self> {
        // Try to load from config file first
        if let Ok(config) = Self::from_file("config.yaml") {
            return Ok(config);
        }
        
        // Fall back to environment variables
        Self::from_env()
    }
    
    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> DocFusionResult<()> {
        let contents = serde_yaml::to_string(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
