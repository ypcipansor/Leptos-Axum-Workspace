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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevokeRequest {
    pub token: String,
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

    #[test]
    fn register_request_roundtrips() {
        let req = RegisterRequest {
            username: "alice".to_string(),
            password: "secret".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: RegisterRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn register_response_roundtrips() {
        let resp = RegisterResponse {
            success: true,
            message: "ok".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: RegisterResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }

    #[test]
    fn login_request_roundtrips() {
        let req = LoginRequest {
            username: "alice".to_string(),
            password: "secret".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: LoginRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn login_response_roundtrips_with_token() {
        let resp = LoginResponse {
            success: true,
            token: Some("tok".to_string()),
            username: Some("alice".to_string()),
            message: "ok".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: LoginResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }

    #[test]
    fn login_response_roundtrips_without_token() {
        let resp = LoginResponse {
            success: false,
            token: None,
            username: None,
            message: "invalid".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: LoginResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }

    #[test]
    fn session_info_roundtrips() {
        let s = SessionInfo {
            token: "tok".to_string(),
            username: "alice".to_string(),
            user_agent: Some("ua".to_string()),
            ip_address: None,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            is_current: true,
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: SessionInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn revoke_request_roundtrips() {
        let req = RevokeRequest {
            token: "tok".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let back: RevokeRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(req, back);
    }

    #[test]
    fn revoke_response_roundtrips() {
        let resp = RevokeResponse {
            success: true,
            message: "done".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: RevokeResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(resp, back);
    }
}
