use std::collections::HashMap;
use std::mem::size_of;
use std::ptr;
use std::sync::Mutex;

use tokio::sync::mpsc;
use tracing::{debug, warn};
use windows::Win32::Foundation::{
    ERROR_INSUFFICIENT_BUFFER, ERROR_MORE_DATA, ERROR_SERVICE_DOES_NOT_EXIST,
};
use windows::Win32::System::Services::{
    ChangeServiceConfigW, CloseServiceHandle, ControlService, ENUM_SERVICE_STATUS_PROCESSW,
    ENUM_SERVICE_TYPE, EnumServicesStatusExW, OpenSCManagerW, OpenServiceW,
    PSC_NOTIFICATION_REGISTRATION, QUERY_SERVICE_CONFIGW, QueryServiceConfigW,
    QueryServiceStatusEx, SC_ENUM_PROCESS_INFO, SC_EVENT_DATABASE_CHANGE, SC_EVENT_PROPERTY_CHANGE,
    SC_EVENT_STATUS_CHANGE, SC_EVENT_TYPE, SC_HANDLE, SC_MANAGER_CONNECT,
    SC_MANAGER_ENUMERATE_SERVICE, SC_STATUS_PROCESS_INFO, SERVICE_AUTO_START, SERVICE_BOOT_START,
    SERVICE_CHANGE_CONFIG, SERVICE_CONTINUE_PENDING, SERVICE_CONTROL_STOP, SERVICE_DEMAND_START,
    SERVICE_DISABLED, SERVICE_ERROR, SERVICE_FILE_SYSTEM_DRIVER, SERVICE_KERNEL_DRIVER,
    SERVICE_NO_CHANGE, SERVICE_PAUSE_PENDING, SERVICE_PAUSED, SERVICE_QUERY_CONFIG,
    SERVICE_QUERY_STATUS, SERVICE_RECOGNIZER_DRIVER, SERVICE_RUNNING, SERVICE_START,
    SERVICE_START_PENDING, SERVICE_START_TYPE, SERVICE_STATE_ALL, SERVICE_STATUS,
    SERVICE_STATUS_CURRENT_STATE, SERVICE_STATUS_PROCESS, SERVICE_STOP, SERVICE_STOP_PENDING,
    SERVICE_STOPPED, SERVICE_SYSTEM_START, SERVICE_WIN32_OWN_PROCESS,
    SERVICE_WIN32_SHARE_PROCESS, StartServiceW, SubscribeServiceChangeNotifications,
    UnsubscribeServiceChangeNotifications,
};
use windows::core::{HRESULT, PCWSTR};

use crate::domain::error::ServiceError;
use crate::domain::repository::ServiceRepository;
use crate::domain::service::{
    ServiceConfig, ServiceInfo, ServiceKind, ServiceRuntimeStatus, ServiceStartType, ServiceState,
};
use crate::domain::watcher::{ServiceWatcher, WatcherSignal};
use crate::scm::error::ScmError;

/// Lists services via the Windows Service Control Manager (advapi32).
pub struct WindowsServiceRepository;

impl ServiceRepository for WindowsServiceRepository {
    fn list_services(&self) -> Result<Vec<ServiceInfo>, ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let entries = enumerate_entries(manager)?;
        Ok(entries
            .into_iter()
            .map(|entry| {
                let (start_type, binary_path, start_name) = config_of(manager, &entry.name);
                ServiceInfo {
                    start_type,
                    name: entry.name,
                    display_name: entry.display_name,
                    state: entry.state,
                    kind: entry.kind,
                    pid: entry.pid,
                    binary_path,
                    start_name,
                }
            })
            .collect())
    }

    fn list_states(&self) -> Result<Vec<ServiceRuntimeStatus>, ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let entries = enumerate_entries(manager)?;
        Ok(entries
            .into_iter()
            .map(|entry| ServiceRuntimeStatus {
                name: entry.name,
                state: entry.state,
                pid: entry.pid,
            })
            .collect())
    }

    fn query_service_status(&self, name: &str) -> Result<Option<ServiceRuntimeStatus>, ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let Some(service) = open_service_opt(manager, name, SERVICE_QUERY_STATUS)? else {
            return Ok(None);
        };
        let _service_guard = ScHandle(service);

        let mut status = SERVICE_STATUS_PROCESS::default();
        let mut needed = 0u32;
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                (&mut status as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                size_of::<SERVICE_STATUS_PROCESS>(),
            )
        };
        unsafe { QueryServiceStatusEx(service, SC_STATUS_PROCESS_INFO, Some(buffer), &mut needed) }
            .map_err(ScmError::from)?;
        Ok(Some(ServiceRuntimeStatus {
            name: name.to_owned(),
            state: map_state(status.dwCurrentState),
            pid: (status.dwProcessId != 0).then_some(status.dwProcessId),
        }))
    }

    fn query_config(&self, name: &str) -> Result<Option<ServiceConfig>, ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let Some(service) = open_service_opt(manager, name, SERVICE_QUERY_CONFIG)? else {
            return Ok(None);
        };
        let _service_guard = ScHandle(service);

        let config = query_config_w(service)?;
        let display_name =
            unsafe { config.lpDisplayName.to_string() }.unwrap_or_else(|_| name.to_owned());
        let binary_path = unsafe { config.lpBinaryPathName.to_string() }.unwrap_or_default();
        Ok(Some(ServiceConfig {
            display_name,
            binary_path,
            start_type: map_start_type(config.dwStartType),
        }))
    }

    fn start_service(&self, name: &str) -> Result<(), ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let service = open_service(manager, name, SERVICE_START)?;
        let _service_guard = ScHandle(service);
        unsafe { StartServiceW(service, None) }.map_err(ScmError::from)?;
        Ok(())
    }

    fn stop_service(&self, name: &str) -> Result<(), ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let service = open_service(manager, name, SERVICE_STOP)?;
        let _service_guard = ScHandle(service);
        let mut status = SERVICE_STATUS::default();
        unsafe { ControlService(service, SERVICE_CONTROL_STOP, &mut status) }
            .map_err(ScmError::from)?;
        Ok(())
    }

    fn set_start_type(&self, name: &str, start_type: ServiceStartType) -> Result<(), ServiceError> {
        let manager = open_manager()?;
        let _guard = ScHandle(manager);
        let service = open_service(manager, name, SERVICE_CHANGE_CONFIG)?;
        let _service_guard = ScHandle(service);
        unsafe {
            ChangeServiceConfigW(
                service,
                ENUM_SERVICE_TYPE(SERVICE_NO_CHANGE),
                SERVICE_START_TYPE(raw_start_type(start_type)?),
                SERVICE_ERROR(SERVICE_NO_CHANGE),
                PCWSTR::null(),
                PCWSTR::null(),
                None,
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
                PCWSTR::null(),
            )
        }
        .map_err(ScmError::from)?;
        Ok(())
    }
}

fn open_manager() -> Result<SC_HANDLE, ScmError> {
    // CONNECT is required to open individual service handles through the manager.
    unsafe {
        OpenSCManagerW(
            PCWSTR::null(),
            PCWSTR::null(),
            SC_MANAGER_CONNECT | SC_MANAGER_ENUMERATE_SERVICE,
        )
    }
    .map_err(Into::into)
}

/// Opens a service handle, requiring it to exist; maps "does not exist" to an error.
fn open_service(manager: SC_HANDLE, name: &str, access: u32) -> Result<SC_HANDLE, ScmError> {
    match open_service_opt(manager, name, access)? {
        Some(service) => Ok(service),
        None => Err(ScmError::service_not_found(name)),
    }
}

/// Opens a service handle, mapping "service does not exist" to `Ok(None)`.
fn open_service_opt(
    manager: SC_HANDLE,
    name: &str,
    access: u32,
) -> Result<Option<SC_HANDLE>, ScmError> {
    let name_wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    match unsafe { OpenServiceW(manager, PCWSTR(name_wide.as_ptr()), access) } {
        Ok(service) => Ok(Some(service)),
        Err(error) if error.code() == HRESULT::from_win32(ERROR_SERVICE_DOES_NOT_EXIST.0) => {
            Ok(None)
        }
        Err(error) => Err(error.into()),
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

/// A service as reported by a single `EnumServicesStatusEx` call, without per-service config.
struct RawServiceEntry {
    name: String,
    display_name: String,
    state: ServiceState,
    kind: ServiceKind,
    pid: Option<u32>,
}

fn enumerate_entries(manager: SC_HANDLE) -> Result<Vec<RawServiceEntry>, ScmError> {
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
    let mut entries = Vec::with_capacity(returned as usize);
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
        entries.push(RawServiceEntry {
            name,
            display_name,
            state: map_state(entry.ServiceStatusProcess.dwCurrentState),
            kind: map_kind(entry.ServiceStatusProcess.dwServiceType),
            pid: (entry.ServiceStatusProcess.dwProcessId != 0)
                .then_some(entry.ServiceStatusProcess.dwProcessId),
        });
    }
    entries.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    debug!(count = entries.len(), "service enumeration complete");
    Ok(entries)
}

/// Queries an individual service's start type, binary path and account. Returns
/// `None` if the service cannot be opened or queried (e.g. it was deleted mid-enumeration).
fn config_of(manager: SC_HANDLE, name: &str) -> (Option<ServiceStartType>, String, Option<String>) {
    let service = match open_service_opt(manager, name, SERVICE_QUERY_CONFIG) {
        Ok(Some(service)) => service,
        Ok(None) => {
            debug!(service = name, "service no longer exists; start type unavailable");
            return (None, String::new(), None);
        }
        Err(error) => {
            debug!(service = name, error = %error, "cannot open service; start type unavailable");
            return (None, String::new(), None);
        }
    };
    let _guard = ScHandle(service);
    match query_config_w(service) {
        Ok(config) => (
            Some(map_start_type(config.dwStartType)),
            unsafe { config.lpBinaryPathName.to_string() }.unwrap_or_default(),
            start_name_of(&config),
        ),
        Err(error) => {
            debug!(service = name, error = %error, "cannot query service config; start type unavailable");
            (None, String::new(), None)
        }
    }
}

/// The account a service runs under; `None` when empty (drivers, system services).
fn start_name_of(config: &QUERY_SERVICE_CONFIGW) -> Option<String> {
    if config.lpServiceStartName.is_null() {
        return None;
    }
    let raw = unsafe { config.lpServiceStartName.to_string() }.unwrap_or_default();
    (!raw.is_empty()).then_some(raw)
}

fn query_config_w(service: SC_HANDLE) -> Result<QUERY_SERVICE_CONFIGW, ScmError> {
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
            Err(error) => return Err(error.into()),
        }
    }
    Ok(unsafe { ptr::read_unaligned(buffer.as_ptr() as *const QUERY_SERVICE_CONFIGW) })
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

/// Inverse of [`map_start_type`], for writing a start type back to SCM.
fn raw_start_type(start_type: ServiceStartType) -> Result<u32, ScmError> {
    let raw = match start_type {
        ServiceStartType::Boot => SERVICE_BOOT_START,
        ServiceStartType::System => SERVICE_SYSTEM_START,
        ServiceStartType::Automatic => SERVICE_AUTO_START,
        ServiceStartType::Manual => SERVICE_DEMAND_START,
        ServiceStartType::Disabled => SERVICE_DISABLED,
        ServiceStartType::Unknown => {
            return Err(ScmError::Internal {
                message: "cannot write an unknown startup type".into(),
            })
        }
    };
    Ok(raw.0)
}

/// Subscribes to SCM change notifications (`SubscribeServiceChangeNotifications`).
///
/// The callback runs on a threadpool owned by the SCM host, so it only ever
/// forwards into the channel. Contexts are leaked for the app lifetime: callbacks
/// may still be in flight after unsubscription, so the box is never reclaimed.
/// The 600-odd contexts per app run cost a few dozen KB at most.
struct SubscriptionCtx {
    tx: mpsc::Sender<WatcherSignal>,
    signal: WatcherSignal,
}

unsafe extern "system" fn notification_callback(
    _dw_notify: u32,
    context: *const core::ffi::c_void,
) {
    // Safety: the context pointer is only ever constructed as a leaked `Box<SubscriptionCtx>`.
    let ctx = unsafe { &*(context as *const SubscriptionCtx) };
    // Never block the SCM threadpool; a full channel means the poll will catch up.
    let _ = ctx.tx.try_send(ctx.signal.clone());
}

fn subscribe_service(
    handle: SC_HANDLE,
    event_type: SC_EVENT_TYPE,
    signal: WatcherSignal,
    tx: &mpsc::Sender<WatcherSignal>,
) -> Result<PSC_NOTIFICATION_REGISTRATION, ScmError> {
    let ctx = Box::leak(Box::new(SubscriptionCtx {
        tx: tx.clone(),
        signal,
    }));
    let mut registration = PSC_NOTIFICATION_REGISTRATION(0);
    let code = unsafe {
        SubscribeServiceChangeNotifications(
            handle,
            event_type,
            Some(notification_callback),
            Some(core::ptr::from_ref(ctx).cast::<core::ffi::c_void>()),
            &mut registration,
        )
    };
    if code != 0 {
        Err(ScmError::from(windows::core::Error::from_hresult(
            HRESULT::from_win32(code),
        )))
    } else {
        Ok(registration)
    }
}

fn unsubscribe_service(registration: &PSC_NOTIFICATION_REGISTRATION) {
    if registration.0 != 0 {
        unsafe { UnsubscribeServiceChangeNotifications(*registration) };
    }
}

/// Watches every service in the database for status, config and add/remove changes.
struct WatcherInner {
    manager: ScHandle,
    tx: mpsc::Sender<WatcherSignal>,
    subscriptions: HashMap<String, ServiceSubscription>,
    database: PSC_NOTIFICATION_REGISTRATION,
}

struct ServiceSubscription {
    /// Keeps the service handle open for the subscription's lifetime; closed on drop.
    #[allow(dead_code)]
    handle: ScHandle,
    status: PSC_NOTIFICATION_REGISTRATION,
    config: PSC_NOTIFICATION_REGISTRATION,
}

pub struct WindowsServiceWatcher {
    inner: Mutex<WatcherInner>,
}

// Sound because all SCM handles are only ever accessed while holding `inner`, and
// the notification callback only reads its own leaked context and the channel.
unsafe impl Send for WindowsServiceWatcher {}
unsafe impl Sync for WindowsServiceWatcher {}

impl WindowsServiceWatcher {
    pub fn new(tx: mpsc::Sender<WatcherSignal>) -> Result<Self, ScmError> {
        let manager = open_manager()?;
        let database = subscribe_service(
            manager,
            SC_EVENT_DATABASE_CHANGE,
            WatcherSignal::Database,
            &tx,
        )?;
        Ok(Self {
            inner: Mutex::new(WatcherInner {
                manager: ScHandle(manager),
                tx,
                subscriptions: HashMap::new(),
                database,
            }),
        })
    }
}

impl ServiceWatcher for WindowsServiceWatcher {
    fn watch_service(&self, name: &str) -> Result<(), ServiceError> {
        let mut inner = self.inner.lock().expect("watcher mutex poisoned");
        if inner.subscriptions.contains_key(name) {
            return Ok(());
        }
        let Some(handle) = open_service_opt(
            inner.manager.0,
            name,
            SERVICE_QUERY_STATUS | SERVICE_QUERY_CONFIG,
        )?
        else {
            return Ok(()); // deleted since it was enumerated
        };

        let mut subscription = ServiceSubscription {
            handle: ScHandle(handle),
            status: PSC_NOTIFICATION_REGISTRATION(0),
            config: PSC_NOTIFICATION_REGISTRATION(0),
        };
        match subscribe_service(
            handle,
            SC_EVENT_STATUS_CHANGE,
            WatcherSignal::Status { name: name.to_owned() },
            &inner.tx,
        ) {
            Ok(registration) => subscription.status = registration,
            Err(error) => warn!(service = name, error = %error, "status subscription failed"),
        }
        match subscribe_service(
            handle,
            SC_EVENT_PROPERTY_CHANGE,
            WatcherSignal::Config { name: name.to_owned() },
            &inner.tx,
        ) {
            Ok(registration) => subscription.config = registration,
            Err(error) => warn!(service = name, error = %error, "config subscription failed"),
        }

        if subscription.status.0 == 0 && subscription.config.0 == 0 {
            return Ok(()); // nothing subscribed; the handle is dropped with the guard
        }
        inner.subscriptions.insert(name.to_owned(), subscription);
        Ok(())
    }

    fn unwatch_service(&self, name: &str) {
        let mut inner = self.inner.lock().expect("watcher mutex poisoned");
        if let Some(subscription) = inner.subscriptions.remove(name) {
            unsubscribe_service(&subscription.status);
            unsubscribe_service(&subscription.config);
        }
    }
}

impl Drop for WindowsServiceWatcher {
    fn drop(&mut self) {
        let inner = self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        unsubscribe_service(&inner.database);
        for subscription in inner.subscriptions.values() {
            unsubscribe_service(&subscription.status);
            unsubscribe_service(&subscription.config);
        }
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
