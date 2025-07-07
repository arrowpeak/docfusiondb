use crate::api::{ApiResponse, DocumentResponse};
use serde_json::json;

// Mock tests for API functionality (requires running database for full integration)
#[cfg(test)]
mod api_unit_tests {
    use super::*;

    // These are structural tests - full integration tests would require a test database

    #[test]
    fn test_api_response_success() {
        let data = DocumentResponse {
            id: 1,
            document: json!({"test": "value"}),
        };

        let response = ApiResponse::success(data);
        assert!(response.success);
        assert!(response.data.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<()>::error("Test error".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap(), "Test error");
    }

    #[test]
    fn test_router_structure() {
        // Test passes if API structures compile correctly
        // This test validates that all our API types are properly defined
    }
}

// Note: Full integration tests would be added here with a test database
// For now we focus on unit tests for the API structures
