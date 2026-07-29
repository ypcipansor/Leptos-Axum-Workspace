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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_name_is_set() {
        assert!(!APP_NAME.is_empty());
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
