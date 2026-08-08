use std::mem::size_of;
use std::ptr;

use tracing::{debug, warn};
use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA};
use windows::Win32::System::Services::{
    CloseServiceHandle, ENUM_SERVICE_STATUS_PROCESSW, ENUM_SERVICE_TYPE, EnumServicesStatusExW,
    OpenSCManagerW, OpenServiceW, QUERY_SERVICE_CONFIGW, QueryServiceConfigW, SC_ENUM_PROCESS_INFO,
    SC_HANDLE, SC_MANAGER_ENUMERATE_SERVICE, SERVICE_AUTO_START, SERVICE_BOOT_START,
    SERVICE_CONTINUE_PENDING, SERVICE_DEMAND_START, SERVICE_DISABLED, SERVICE_FILE_SYSTEM_DRIVER,
    SERVICE_KERNEL_DRIVER, SERVICE_PAUSE_PENDING, SERVICE_PAUSED, SERVICE_QUERY_CONFIG,
    SERVICE_RECOGNIZER_DRIVER, SERVICE_RUNNING, SERVICE_START_PENDING, SERVICE_START_TYPE,
    SERVICE_STATE_ALL, SERVICE_STATUS_CURRENT_STATE, SERVICE_STOP_PENDING, SERVICE_STOPPED,
    SERVICE_SYSTEM_START, SERVICE_WIN32_OWN_PROCESS, SERVICE_WIN32_SHARE_PROCESS,
};
use windows::core::{HRESULT, PCWSTR};

use crate::domain::error::ServiceError;
use crate::domain::repository::ServiceRepository;
use crate::domain::service::{ServiceInfo, ServiceKind, ServiceStartType, ServiceState};

/// Lists services via the Windows Service Control Manager (advapi32).
pub struct WindowsServiceRepository;

impl ServiceRepository for WindowsServiceRepository {
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
        let manager = unsafe {
            OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE)
        }?;
        let _guard = ScHandle(manager);
        enumerate_services(manager)
    }
}

/// RAII wrapper closing an SCM handle on drop.
struct ScHandle(SC_HANDLE);

impl Drop for ScHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseServiceHandle(self.0);
        }
    }
}

fn enumerate_services(manager: SC_HANDLE) -> Result<Vec<ServiceInfo>, ServiceError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut needed = 0u32;
    let mut returned = 0u32;
    let mut resume = 0u32;

    loop {
        let result = unsafe {
            EnumServicesStatusExW(
                manager,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32_OWN_PROCESS | SERVICE_WIN32_SHARE_PROCESS,
                SERVICE_STATE_ALL,
                Some(&mut buffer),
                &mut needed,
                &mut returned,
                Some(&mut resume),
                PCWSTR::null(),
            )
        };
        match result {
            Ok(()) => break,
            Err(e) if e.code() == HRESULT::from_win32(ERROR_MORE_DATA.0) => {
                buffer.resize(needed as usize, 0);
            }
            Err(e) => return Err(e.into()),
        }
    }

    let entry_size = size_of::<ENUM_SERVICE_STATUS_PROCESSW>();
    let mut services = Vec::with_capacity(returned as usize);
    for i in 0..returned as usize {
        let entry = unsafe {
            ptr::read_unaligned(
                buffer.as_ptr().add(i * entry_size) as *const ENUM_SERVICE_STATUS_PROCESSW
            )
        };
        let Ok(name) = (unsafe { entry.lpServiceName.to_string() }) else {
            warn!(
                step = "lpServiceName",
                index = i,
                "skipping service with malformed name"
            );
            continue;
        };
        let display_name =
            unsafe { entry.lpDisplayName.to_string() }.unwrap_or_else(|_| name.clone());
        let start_type = start_type_of(manager, &name);
        services.push(ServiceInfo {
            name,
            display_name,
            state: map_state(entry.ServiceStatusProcess.dwCurrentState),
            start_type,
            kind: map_kind(entry.ServiceStatusProcess.dwServiceType),
            pid: (entry.ServiceStatusProcess.dwProcessId != 0)
                .then_some(entry.ServiceStatusProcess.dwProcessId),
        });
    }
    services.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    debug!(count = services.len(), "service enumeration complete");
    Ok(services)
}

/// Queries an individual service's start type. Returns `None` if the service
/// cannot be opened or queried (e.g. it was deleted mid-enumeration).
fn start_type_of(manager: SC_HANDLE, name: &str) -> Option<ServiceStartType> {
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let service =
        match unsafe { OpenServiceW(manager, PCWSTR(name_wide.as_ptr()), SERVICE_QUERY_CONFIG) } {
            Ok(service) => service,
            Err(error) => {
                debug!(
                    service = name,
                    error = %error,
                    "cannot open service; start type unavailable"
                );
                return None;
            }
        };
    let _guard = ScHandle(service);

    let mut buffer: Vec<u8> = Vec::new();
    let mut needed = 0u32;
    loop {
        let result = unsafe {
            let buffer_ptr = if buffer.is_empty() {
                None
            } else {
                Some(buffer.as_mut_ptr() as *mut QUERY_SERVICE_CONFIGW)
            };
            QueryServiceConfigW(service, buffer_ptr, buffer.len() as u32, &mut needed)
        };
        match result {
            Ok(()) => break,
            Err(error) if error.code() == HRESULT::from_win32(ERROR_INSUFFICIENT_BUFFER.0) => {
                buffer.resize(needed as usize, 0);
            }
            Err(error) => {
                debug!(
                    service = name,
                    error = %error,
                    "cannot query service config; start type unavailable"
                );
                return None;
            }
        }
    }

    let config = unsafe { ptr::read_unaligned(buffer.as_ptr() as *const QUERY_SERVICE_CONFIGW) };
    Some(map_start_type(config.dwStartType))
}

fn map_state(raw: SERVICE_STATUS_CURRENT_STATE) -> ServiceState {
    match raw {
        SERVICE_STOPPED => ServiceState::Stopped,
        SERVICE_START_PENDING => ServiceState::StartPending,
        SERVICE_STOP_PENDING => ServiceState::StopPending,
        SERVICE_RUNNING => ServiceState::Running,
        SERVICE_CONTINUE_PENDING => ServiceState::ContinuePending,
        SERVICE_PAUSE_PENDING => ServiceState::PausePending,
        SERVICE_PAUSED => ServiceState::Paused,
        _ => ServiceState::Unknown,
    }
}

fn map_kind(raw: ENUM_SERVICE_TYPE) -> ServiceKind {
    if raw.contains(SERVICE_WIN32_OWN_PROCESS) {
        ServiceKind::Win32OwnProcess
    } else if raw.contains(SERVICE_WIN32_SHARE_PROCESS) {
        ServiceKind::Win32ShareProcess
    } else if raw.contains(SERVICE_KERNEL_DRIVER) {
        ServiceKind::KernelDriver
    } else if raw.contains(SERVICE_FILE_SYSTEM_DRIVER) {
        ServiceKind::FileSystemDriver
    } else if raw.contains(SERVICE_RECOGNIZER_DRIVER) {
        ServiceKind::RecognizerDriver
    } else {
        ServiceKind::Unknown
    }
}

fn map_start_type(raw: SERVICE_START_TYPE) -> ServiceStartType {
    match raw {
        SERVICE_BOOT_START => ServiceStartType::Boot,
        SERVICE_SYSTEM_START => ServiceStartType::System,
        SERVICE_AUTO_START => ServiceStartType::Automatic,
        SERVICE_DEMAND_START => ServiceStartType::Manual,
        SERVICE_DISABLED => ServiceStartType::Disabled,
        _ => ServiceStartType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_state_known_states() {
        let cases = [
            (SERVICE_STOPPED, ServiceState::Stopped),
            (SERVICE_START_PENDING, ServiceState::StartPending),
            (SERVICE_STOP_PENDING, ServiceState::StopPending),
            (SERVICE_RUNNING, ServiceState::Running),
            (SERVICE_CONTINUE_PENDING, ServiceState::ContinuePending),
            (SERVICE_PAUSE_PENDING, ServiceState::PausePending),
            (SERVICE_PAUSED, ServiceState::Paused),
        ];
        for (raw, expected) in cases {
            assert_eq!(map_state(raw), expected);
        }
    }

    #[test]
    fn map_state_unknown_bit_pattern() {
        assert_eq!(
            map_state(SERVICE_STATUS_CURRENT_STATE(0xDEAD)),
            ServiceState::Unknown
        );
    }

    #[test]
    fn map_kind_known_types() {
        let cases = [
            (SERVICE_WIN32_OWN_PROCESS, ServiceKind::Win32OwnProcess),
            (SERVICE_WIN32_SHARE_PROCESS, ServiceKind::Win32ShareProcess),
            (
                SERVICE_WIN32_OWN_PROCESS | SERVICE_WIN32_SHARE_PROCESS,
                ServiceKind::Win32OwnProcess,
            ),
            (SERVICE_KERNEL_DRIVER, ServiceKind::KernelDriver),
            (SERVICE_FILE_SYSTEM_DRIVER, ServiceKind::FileSystemDriver),
            (SERVICE_RECOGNIZER_DRIVER, ServiceKind::RecognizerDriver),
        ];
        for (raw, expected) in cases {
            assert_eq!(map_kind(raw), expected);
        }
    }

    #[test]
    fn map_kind_unknown_or_empty_type() {
        assert_eq!(
            map_kind(ENUM_SERVICE_TYPE(0xDEAD << 16)),
            ServiceKind::Unknown
        );
        assert_eq!(map_kind(ENUM_SERVICE_TYPE(0)), ServiceKind::Unknown);
    }

    #[test]
    fn map_start_type_known_types() {
        let cases = [
            (SERVICE_BOOT_START, ServiceStartType::Boot),
            (SERVICE_SYSTEM_START, ServiceStartType::System),
            (SERVICE_AUTO_START, ServiceStartType::Automatic),
            (SERVICE_DEMAND_START, ServiceStartType::Manual),
            (SERVICE_DISABLED, ServiceStartType::Disabled),
        ];
        for (raw, expected) in cases {
            assert_eq!(map_start_type(raw), expected);
        }
    }

    #[test]
    fn map_start_type_unknown_value() {
        assert_eq!(
            map_start_type(SERVICE_START_TYPE(0xDEAD)),
            ServiceStartType::Unknown
        );
    }
}
