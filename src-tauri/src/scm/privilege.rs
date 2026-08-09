use std::mem::size_of;

use tracing::debug;
use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_CANCELLED};
use windows::Win32::System::Services::{CloseServiceHandle, OpenSCManagerW, SC_MANAGER_ALL_ACCESS};
use windows::Win32::UI::Shell::{ShellExecuteExW, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

use crate::domain::error::ServiceError;

/// True when the current process can administer the SCM, i.e. runs elevated.
///
/// Probes by opening the SCM database with full access; the default ACL denies
/// that to non-elevated tokens (`ERROR_ACCESS_DENIED`).
pub fn is_elevated() -> bool {
    let manager = unsafe {
        OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ALL_ACCESS)
    };
    match manager {
        Ok(manager) => {
            unsafe {
                let _ = CloseServiceHandle(manager);
            }
            true
        }
        Err(error) if error.code() == HRESULT::from_win32(ERROR_ACCESS_DENIED.0) => false,
        Err(error) => {
            debug!(error = %error, "elevation probe failed; assuming not elevated");
            false
        }
    }
}

/// Outcome of an elevation relaunch attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelaunchOutcome {
    /// A new elevated process was launched; the caller should exit the current one.
    Launched,
    /// The user dismissed the UAC prompt; nothing changed, keep running.
    Cancelled,
}

/// Relaunches the current executable with the `runas` verb (UAC prompt).
///
/// Returns [`RelaunchOutcome::Launched`] when a new elevated process was
/// launched, or [`RelaunchOutcome::Cancelled`] when the user dismissed the
/// prompt.
pub fn relaunch_elevated() -> Result<RelaunchOutcome, ServiceError> {
    let exe = std::env::current_exe().map_err(|error| ServiceError::Internal {
        message: format!("cannot resolve current executable: {error}"),
    })?;
    let file: Vec<u16> = exe
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let verb: Vec<u16> = "runas".encode_utf16().chain(std::iter::once(0)).collect();
    let parameters: Vec<u16> = "--elevated"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(parameters.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    if let Err(error) = unsafe { ShellExecuteExW(&mut info) } {
        if error.code() == HRESULT::from_win32(ERROR_CANCELLED.0) {
            debug!("user cancelled the elevation prompt");
            return Ok(RelaunchOutcome::Cancelled);
        }
        return Err(ServiceError::Internal {
            message: format!("failed to relaunch elevated: {error}"),
        });
    }
    Ok(RelaunchOutcome::Launched)
}
