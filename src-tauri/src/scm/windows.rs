use std::mem::size_of;
use std::ptr;

use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::ERROR_MORE_DATA;
use windows::Win32::System::Services::{
    CloseServiceHandle, EnumServicesStatusExW, OpenSCManagerW, ENUM_SERVICE_STATUS_PROCESSW,
    ENUM_SERVICE_TYPE, SC_ENUM_PROCESS_INFO, SC_HANDLE, SC_MANAGER_ENUMERATE_SERVICE,
    SERVICE_CONTINUE_PENDING, SERVICE_FILE_SYSTEM_DRIVER, SERVICE_KERNEL_DRIVER, SERVICE_PAUSED,
    SERVICE_PAUSE_PENDING, SERVICE_RECOGNIZER_DRIVER, SERVICE_RUNNING, SERVICE_START_PENDING,
    SERVICE_STATE_ALL, SERVICE_STATUS_CURRENT_STATE, SERVICE_STOPPED, SERVICE_STOP_PENDING,
    SERVICE_WIN32, SERVICE_WIN32_OWN_PROCESS, SERVICE_WIN32_SHARE_PROCESS,
};

use crate::domain::service::{ServiceInfo, ServiceKind, ServiceState};

use super::{ScmError, ServiceRepository};

/// Lists services via the Windows Service Control Manager (advapi32).
pub struct WindowsServiceRepository;

impl ServiceRepository for WindowsServiceRepository {
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ScmError> {
        let manager =
            unsafe { OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), SC_MANAGER_ENUMERATE_SERVICE) }?;
        let _guard = ScHandle(manager);
        enumerate_services(manager)
    }
}

/// RAII wrapper closing an SCM handle on drop.
struct ScHandle(SC_HANDLE);

impl Drop for ScHandle {
    fn drop(&mut self) {
        unsafe { let _ = CloseServiceHandle(self.0); }
    }
}

fn enumerate_services(manager: SC_HANDLE) -> Result<Vec<ServiceInfo>, ScmError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut needed = 0u32;
    let mut returned = 0u32;
    let mut resume = 0u32;

    loop {
        let result = unsafe {
            EnumServicesStatusExW(
                manager,
                SC_ENUM_PROCESS_INFO,
                SERVICE_WIN32,
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
        services.push(ServiceInfo {
            name: unsafe { entry.lpServiceName.to_string().unwrap_or_default() },
            display_name: unsafe { entry.lpDisplayName.to_string().unwrap_or_default() },
            state: map_state(entry.ServiceStatusProcess.dwCurrentState),
            start_type: None,
            kind: map_kind(entry.ServiceStatusProcess.dwServiceType),
            pid: (entry.ServiceStatusProcess.dwProcessId != 0)
                .then_some(entry.ServiceStatusProcess.dwProcessId),
        });
    }
    Ok(services)
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
