use crate::{Config, config::*, error::*};

mod api_tests;

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5432);
        assert_eq!(config.database.user, "postgres");
        assert_eq!(config.database.database, "docfusiondb");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.database.min_connections, 1);
    }

    #[test]
    fn test_database_connection_string() {
        let config = DatabaseConfig {
            host: "testhost".to_string(),
            port: 5433,
            user: "testuser".to_string(),
            password: "testpass".to_string(),
            database: "testdb".to_string(),
            ..Default::default()
        };

        let conn_str = config.connection_string();
        assert_eq!(
            conn_str,
            "host=testhost port=5433 user=testuser password=testpass dbname=testdb"
        );
    }

    #[test]
    fn test_database_config_from_url() {
        let url = "postgres://user:pass@localhost:5432/mydb";
        let config = DatabaseConfig::from_url(url).unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.user, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.database, "mydb");
    }

    #[test]
    fn test_database_config_from_url_with_default_port() {
        let url = "postgres://user:pass@localhost/mydb";
        let config = DatabaseConfig::from_url(url).unwrap();

        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.user, "user");
        assert_eq!(config.password, "pass");
        assert_eq!(config.database, "mydb");
    }

    #[test]
    fn test_database_config_from_invalid_url() {
        let url = "invalid://url";
        let result = DatabaseConfig::from_url(url);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DocFusionError::Config { .. }));
    }

    #[test]
    fn test_error_is_retryable() {
        // Test retryable errors
        assert!(DocFusionError::ConnectionTimeout.is_retryable());
        assert!(DocFusionError::OperationTimeout.is_retryable());

        // Test non-retryable errors
        assert!(!DocFusionError::invalid_query("test").is_retryable());
        assert!(!DocFusionError::document_not_found(1).is_retryable());
        assert!(!DocFusionError::invalid_document("test").is_retryable());
    }

    #[test]
    fn test_error_constructors() {
        let config_error = DocFusionError::config("test message");
        assert!(matches!(config_error, DocFusionError::Config { .. }));

        let query_error = DocFusionError::invalid_query("bad query");
        assert!(matches!(query_error, DocFusionError::InvalidQuery { .. }));

        let not_found_error = DocFusionError::document_not_found(42);
        assert!(matches!(
            not_found_error,
            DocFusionError::DocumentNotFound { id: 42 }
        ));

        let doc_error = DocFusionError::invalid_document("bad doc");
        assert!(matches!(doc_error, DocFusionError::InvalidDocument { .. }));

        let internal_error = DocFusionError::internal("internal issue");
        assert!(matches!(internal_error, DocFusionError::Internal { .. }));
    }

    #[test]
    fn test_config_save_and_load() {
        use tempfile::NamedTempFile;

        let config = Config::default();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        // Save config
        config.save_to_file(path).unwrap();

        // Load config
        let loaded_config = Config::from_file(path).unwrap();

        assert_eq!(config.database.host, loaded_config.database.host);
        assert_eq!(config.database.port, loaded_config.database.port);
        assert_eq!(config.server.port, loaded_config.server.port);
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn test_config_from_env() {
        // Store original values to restore later
        let original_host = env::var("DB_HOST").ok();
        let original_port = env::var("DB_PORT").ok();
        let original_user = env::var("DB_USER").ok();
        let original_password = env::var("DB_PASSWORD").ok();
        let original_name = env::var("DB_NAME").ok();
        let original_server_port = env::var("SERVER_PORT").ok();
        
        // Set environment variables for test
        unsafe {
            env::set_var("DB_HOST", "testhost");
            env::set_var("DB_PORT", "5433");
            env::set_var("DB_USER", "testuser");
            env::set_var("DB_PASSWORD", "testpass");
            env::set_var("DB_NAME", "testdb");
            env::set_var("SERVER_PORT", "9090");
        }

        let config = Config::from_env().unwrap();

        assert_eq!(config.database.host, "testhost");
        assert_eq!(config.database.port, 5433);
        assert_eq!(config.database.user, "testuser");
        assert_eq!(config.database.password, "testpass");
        assert_eq!(config.database.database, "testdb");
        assert_eq!(config.server.port, 9090);

        // Restore original values or remove if they didn't exist
        unsafe {
            match original_host {
                Some(val) => env::set_var("DB_HOST", val),
                None => env::remove_var("DB_HOST"),
            }
            match original_port {
                Some(val) => env::set_var("DB_PORT", val),
                None => env::remove_var("DB_PORT"),
            }
            match original_user {
                Some(val) => env::set_var("DB_USER", val),
                None => env::remove_var("DB_USER"),
            }
            match original_password {
                Some(val) => env::set_var("DB_PASSWORD", val),
                None => env::remove_var("DB_PASSWORD"),
            }
            match original_name {
                Some(val) => env::set_var("DB_NAME", val),
                None => env::remove_var("DB_NAME"),
            }
            match original_server_port {
                Some(val) => env::set_var("SERVER_PORT", val),
                None => env::remove_var("SERVER_PORT"),
            }
        }
    }

    #[tokio::test]
    async fn test_config_from_database_url() {
        // Store original value to restore later
        let original_database_url = env::var("DATABASE_URL").ok();
        
        unsafe {
            env::set_var("DATABASE_URL", "postgres://user:pass@localhost:5433/testdb");
        }

        let config = Config::from_env().unwrap();

        assert_eq!(config.database.host, "localhost");
        assert_eq!(config.database.port, 5433);
        assert_eq!(config.database.user, "user");
        assert_eq!(config.database.password, "pass");
        assert_eq!(config.database.database, "testdb");

        // Restore original value or remove if it didn't exist
        unsafe {
            match original_database_url {
                Some(val) => env::set_var("DATABASE_URL", val),
                None => env::remove_var("DATABASE_URL"),
            }
        }
    }
}
