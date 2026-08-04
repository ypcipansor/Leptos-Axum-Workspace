use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// One active sign-in, as shown in the session management table.
///
/// Timestamps are real `DateTime<Utc>` values rather than pre-formatted
/// strings, so the browser can render them in the viewer's locale and timezone
/// instead of echoing whatever the server happened to format.
///
/// There is deliberately no `username` field: every session in the list belongs
/// to the requesting user, so repeating it on each row carried no information.
///
/// The `id` here is the session's public identifier. It is **not** the session
/// token -- the token lives only in an `HttpOnly` cookie and is never serialized
/// to the browser's JavaScript context, which is what makes it safe to list and
/// address sessions from the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// Whether this row is the session making the request. The UI uses it to
    /// label the row and to warn that revoking it signs the user out here.
    pub is_current: bool,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    #[test]
    fn round_trips_through_json() {
        let summary = SessionSummary {
            id: Uuid::nil(),
            user_agent: Some("Mozilla/5.0".to_owned()),
            ip_address: Some("203.0.113.7".to_owned()),
            created_at: at(1_700_000_000),
            last_seen_at: at(1_700_003_600),
            expires_at: at(1_700_600_000),
            is_current: true,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert_eq!(
            serde_json::from_str::<SessionSummary>(&json).unwrap(),
            summary
        );
    }

    #[test]
    fn round_trips_with_absent_client_metadata() {
        // A request arriving without a User-Agent, or from a source whose
        // address we decline to trust, yields None rather than a fabricated
        // placeholder.
        let summary = SessionSummary {
            id: Uuid::nil(),
            user_agent: None,
            ip_address: None,
            created_at: at(1_700_000_000),
            last_seen_at: at(1_700_000_000),
            expires_at: at(1_700_600_000),
            is_current: false,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert_eq!(
            serde_json::from_str::<SessionSummary>(&json).unwrap(),
            summary
        );
    }

    #[test]
    fn timestamps_serialize_as_rfc3339() {
        let summary = SessionSummary {
            id: Uuid::nil(),
            user_agent: None,
            ip_address: None,
            created_at: at(1_700_000_000),
            last_seen_at: at(1_700_000_000),
            expires_at: at(1_700_600_000),
            is_current: false,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("2023-11-14T22:13:20Z"), "got {json}");
    }
}
