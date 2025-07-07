use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde_json::json;

use crate::config::AuthConfig;

/// Simple API key authentication middleware
pub async fn auth_middleware(
    State(auth_config): State<AuthConfig>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Skip auth if disabled
    if !auth_config.enabled {
        return Ok(next.run(request).await);
    }
    
    // Check for API key in headers
    let api_key = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            // Also check Authorization header with Bearer format
            headers
                .get("Authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|auth| auth.strip_prefix("Bearer "))
        });
    
    match (api_key, &auth_config.api_key) {
        (Some(provided_key), Some(expected_key)) if provided_key == expected_key => {
            Ok(next.run(request).await)
        }
        (None, Some(_)) => Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "Missing API key",
                "message": "Provide API key via X-API-Key header or Authorization: Bearer <key>"
            })
            .to_string(),
        )),
        (Some(_), Some(_)) => Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "error": "Invalid API key",
                "message": "The provided API key is invalid"
            })
            .to_string(),
        )),
        _ => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "error": "Auth configuration error",
                "message": "Authentication is enabled but no API key is configured"
            })
            .to_string(),
        )),
    }
}
