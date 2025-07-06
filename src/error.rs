use thiserror::Error;

/// DocFusionDB error types
#[derive(Error, Debug)]
pub enum DocFusionError {
    #[error("Database error: {0}")]
    Database(#[from] tokio_postgres::Error),
    
    #[error("Connection pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),
    
    #[error("Connection pool config error: {0}")]
    PoolConfig(#[from] deadpool_postgres::CreatePoolError),
    
    #[error("DataFusion error: {0}")]
    DataFusion(#[from] datafusion::error::DataFusionError),
    
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    
    #[error("Configuration error: {message}")]
    Config { message: String },
    
    #[error("Invalid query: {message}")]
    InvalidQuery { message: String },
    
    #[error("Document not found: id={id}")]
    DocumentNotFound { id: i32 },
    
    #[error("Invalid document format: {message}")]
    InvalidDocument { message: String },
    
    #[error("Connection timeout")]
    ConnectionTimeout,
    
    #[error("Operation timeout")]
    OperationTimeout,
    
    #[error("Internal error: {message}")]
    Internal { message: String },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl DocFusionError {
    /// Create a new configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config { message: message.into() }
    }
    
    /// Create a new invalid query error
    pub fn invalid_query(message: impl Into<String>) -> Self {
        Self::InvalidQuery { message: message.into() }
    }
    
    /// Create a new document not found error
    pub fn document_not_found(id: i32) -> Self {
        Self::DocumentNotFound { id }
    }
    
    /// Create a new invalid document error
    pub fn invalid_document(message: impl Into<String>) -> Self {
        Self::InvalidDocument { message: message.into() }
    }
    
    /// Create a new internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal { message: message.into() }
    }
    
    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            DocFusionError::ConnectionTimeout
            | DocFusionError::OperationTimeout
            | DocFusionError::Pool(_) => true,
            DocFusionError::Database(e) => {
                // Check for specific PostgreSQL error codes that are retryable
                if let Some(code) = e.code() {
                    matches!(code.code(), 
                        "40001" | // serialization_failure
                        "40P01" | // deadlock_detected
                        "53300" | // too_many_connections
                        "08006" | // connection_failure
                        "08001"   // sqlclient_unable_to_establish_sqlconnection
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Result type for DocFusionDB operations
pub type DocFusionResult<T> = Result<T, DocFusionError>;
