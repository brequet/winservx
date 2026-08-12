use crate::domain::error::ServiceError;

/// Error from a Windows SCM operation, before lifting to the domain vocabulary.
///
/// Owns the Win32-specific mapping (`HRESULT` → Win32 code) so the domain layer
/// never touches the `windows` crate.
#[derive(Debug, thiserror::Error)]
pub enum ScmError {
    #[error("Windows error 0x{code:08X}: {message}")]
    Windows { code: u32, message: String },
    #[error("internal error: {message}")]
    Internal { message: String },
}

impl ScmError {
    /// Windows error 1060 (`ERROR_SERVICE_DOES_NOT_EXIST`).
    pub fn service_not_found(name: &str) -> Self {
        ScmError::Windows {
            code: 1060,
            message: format!("the specified service does not exist: '{name}'"),
        }
    }
}

impl From<::windows::core::Error> for ScmError {
    fn from(value: ::windows::core::Error) -> Self {
        // SCM functions report Win32 error codes, which live in the low 16 bits of the HRESULT.
        ScmError::Windows {
            code: value.code().0 as u32 & 0xFFFF,
            message: value.message(),
        }
    }
}

impl From<ScmError> for ServiceError {
    fn from(value: ScmError) -> Self {
        match value {
            ScmError::Windows { code, message } => ServiceError::Windows { code, message },
            ScmError::Internal { message } => ServiceError::Internal { message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_windows_error_preserves_win32_code() {
        let windows_error =
            windows::core::Error::from_hresult(windows::core::HRESULT::from_win32(5));
        let error: ScmError = windows_error.into();
        match error {
            ScmError::Windows { code, message } => {
                assert_eq!(code, 5);
                assert!(!message.is_empty(), "expected a non-empty error message");
            }
            other => panic!("expected Windows variant, got {other:?}"),
        }
    }

    #[test]
    fn lifts_to_service_error() {
        let error: ServiceError = ScmError::Windows {
            code: 5,
            message: "Access is denied".into(),
        }
        .into();
        assert!(
            matches!(&error, ServiceError::Windows { code: 5, message } if message == "Access is denied"),
            "expected windows error, got {error:?}"
        );
    }
}
