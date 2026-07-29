use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = "Simple Management Information System";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub status: String,
}

impl HealthStatus {
    pub fn ok(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            status: "ok".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginResponse {
    pub success: bool,
    pub token: Option<String>,
    pub username: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionInfo {
    pub token: String,
    pub username: String,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevokeResponse {
    pub success: bool,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_set() {
        assert_eq!(APP_NAME, "Simple Management Information System");
    }

    #[test]
    fn health_status_ok_builder_sets_ok() {
        assert_eq!(
            HealthStatus::ok("backend"),
            HealthStatus {
                service: "backend".to_string(),
                status: "ok".to_string(),
            }
        );
    }
}
