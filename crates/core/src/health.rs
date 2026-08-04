use serde::{Deserialize, Serialize};

/// Whether a dependency the service needs is currently usable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Up,
    Down,
}

impl ServiceState {
    #[must_use]
    pub const fn is_up(self) -> bool {
        matches!(self, Self::Up)
    }
}

/// Payload of the readiness endpoint.
///
/// Liveness and readiness are separate concerns: the process can be alive while
/// its database is unreachable. Orchestrators need to distinguish "restart me"
/// from "stop sending me traffic", so `database` is reported explicitly rather
/// than folded into a single boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    pub service: String,
    pub version: String,
    pub database: ServiceState,
}

impl HealthStatus {
    #[must_use]
    pub fn new(
        service: impl Into<String>,
        version: impl Into<String>,
        database: ServiceState,
    ) -> Self {
        Self {
            service: service.into(),
            version: version.into(),
            database,
        }
    }

    /// True when every dependency is usable and the service should receive traffic.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.database.is_up()
    }
}
