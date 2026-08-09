use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::service::ServiceState;

/// Error returned by SCM operations, serialized to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, Type, thiserror::Error)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ServiceError {
    #[error("Windows error 0x{code:08X}: {message}")]
    Windows { code: u32, message: String },
    #[error("internal error: {message}")]
    Internal { message: String },
    #[error("timed out waiting for service '{service}' to reach {target:?}")]
    Timeout { service: String, target: ServiceState },
}

impl ServiceError {
    /// Windows error 1060 (`ERROR_SERVICE_DOES_NOT_EXIST`).
    pub fn service_not_found(name: &str) -> Self {
        ServiceError::Windows {
            code: 1060,
            message: format!("the specified service does not exist: '{name}'"),
        }
    }
}

impl From<::windows::core::Error> for ServiceError {
    fn from(value: ::windows::core::Error) -> Self {
        // SCM functions report Win32 error codes, which live in the low 16 bits of the HRESULT.
        ServiceError::Windows { code: value.code().0 as u32 & 0xFFFF, message: value.message() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_windows_error() {
        let value = serde_json::to_value(ServiceError::Windows {
            code: 5,
            message: "Access is denied".into(),
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "kind": "windows", "code": 5, "message": "Access is denied" })
        );
    }

    #[test]
    fn serializes_internal_error() {
        let value = serde_json::to_value(ServiceError::Internal { message: "boom".into() }).unwrap();
        assert_eq!(value, serde_json::json!({ "kind": "internal", "message": "boom" }));
    }

    #[test]
    fn serializes_timeout_error() {
        let value = serde_json::to_value(ServiceError::Timeout {
            service: "redis".into(),
            target: ServiceState::Stopped,
        })
        .unwrap();
        assert_eq!(
            value,
            serde_json::json!({ "kind": "timeout", "service": "redis", "target": "stopped" })
        );
    }

    #[test]
    fn from_windows_error_preserves_win32_code() {
        let windows_error = windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(5));
        let error: ServiceError = windows_error.into();
        match error {
            ServiceError::Windows { code, message } => {
                assert_eq!(code, 5);
                assert!(!message.is_empty(), "expected a non-empty error message");
            }
            other => panic!("expected Windows variant, got {other:?}"),
        }
    }
}
