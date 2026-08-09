use std::ffi::OsString;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::exit;
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

const SERVICES: [(&str, &str); 3] = [
    ("brequet-service-1", "Brequet Service 1"),
    ("brequet-service-2", "Brequet Service 2"),
    ("brequet-service-3", "Brequet Service 3"),
];
const MIN_DELAY_SECS: u64 = 1;
const MAX_DELAY_SECS: u64 = 10;
const STATUS_TICK: Duration = Duration::from_millis(500);

fn main() -> windows_service::Result<()> {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    match args.first().and_then(|a| a.to_str()) {
        Some("install") => install_all(),
        Some("uninstall") => uninstall_all(),
        Some("run") => {
            let name = match args.get(1).and_then(|a| a.to_str()) {
                Some(n) if SERVICES.iter().any(|(s, _)| *s == n) => n,
                _ => usage(),
            };
            service_dispatcher::start(name, ffi_service_main)
        }
        _ => usage(),
    }
}

fn usage() -> ! {
    eprintln!("usage: dummy-service [install | uninstall | run <service-name>]");
    exit(2);
}

fn install_all() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )?;
    let exe = std::env::current_exe().expect("cannot locate own executable");
    for (name, display_name) in SERVICES {
        let info = ServiceInfo {
            name: name.into(),
            display_name: display_name.into(),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::OnDemand,
            error_control: ServiceErrorControl::Normal,
            executable_path: exe.clone(),
            launch_arguments: vec![OsString::from("run"), OsString::from(name)],
            dependencies: vec![],
            account_name: None,
            account_password: None,
        };
        match manager.create_service(&info, ServiceAccess::CHANGE_CONFIG) {
            Ok(service) => {
                service.set_description(format!(
                    "Dummy service for WinServX testing. Simulated start/stop delay: {MIN_DELAY_SECS}-{MAX_DELAY_SECS}s."
                ))?;
                println!("{name}: installed (binPath = {:?} run {name})", exe.display());
            }
            Err(e) => println!("{name}: install failed: {e}"),
        }
    }
    Ok(())
}

fn uninstall_all() -> windows_service::Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)?;
    for (name, _) in SERVICES {
        match manager.open_service(
            name,
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
        ) {
            Ok(service) => {
                let state = service.query_status()?.current_state;
                if state != ServiceState::Stopped {
                    println!("{name}: stopping ({state:?})...");
                    service.stop()?;
                }
                service.delete()?;
                drop(service);
                println!("{name}: deleted");
            }
            Err(e) => println!("{name}: skipped: {e}"),
        }
    }
    Ok(())
}

define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(arguments: Vec<OsString>) {
    let service_name = arguments
        .first()
        .and_then(|a| a.to_str())
        .unwrap_or("unknown");
    if let Err(e) = run_service(service_name) {
        log(&format!("{service_name}: fatal: {e}"));
    }
}

fn run_service(service_name: &str) -> windows_service::Result<()> {
    let (stop_tx, stop_rx) = mpsc::channel::<ServiceControl>();
    let event_handler = move |control: ServiceControl| -> ServiceControlHandlerResult {
        match control {
            ServiceControl::Stop => {
                let _ = stop_tx.send(control);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };
    let status = service_control_handler::register(service_name, event_handler)?;

    let start_delay = random_delay();
    log(&format!("{service_name}: start requested, simulated start delay {start_delay:?}"));
    set_status(&status, ServiceState::StartPending, ServiceControlAccept::STOP, 1)?;
    pending_sleep(&status, ServiceState::StartPending, start_delay)?;
    set_status(&status, ServiceState::Running, ServiceControlAccept::STOP, 0)?;
    log(&format!("{service_name}: running"));

    loop {
        match stop_rx.recv_timeout(STATUS_TICK) {
            Ok(_) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let stop_delay = random_delay();
    log(&format!("{service_name}: stop requested, simulated stop delay {stop_delay:?}"));
    set_status(&status, ServiceState::StopPending, ServiceControlAccept::STOP, 1)?;
    pending_sleep(&status, ServiceState::StopPending, stop_delay)?;
    set_status(&status, ServiceState::Stopped, ServiceControlAccept::empty(), 0)?;
    log(&format!("{service_name}: stopped"));

    Ok(())
}

fn pending_sleep(
    status: &ServiceStatusHandle,
    state: ServiceState,
    delay: Duration,
) -> windows_service::Result<()> {
    let mut checkpoint = 1u32;
    let start = Instant::now();
    while start.elapsed() < delay {
        std::thread::sleep(STATUS_TICK);
        checkpoint += 1;
        set_status(status, state, ServiceControlAccept::STOP, checkpoint)?;
    }
    Ok(())
}

fn set_status(
    status: &ServiceStatusHandle,
    state: ServiceState,
    accepted: ServiceControlAccept,
    checkpoint: u32,
) -> windows_service::Result<()> {
    status.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: accepted,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint,
        wait_hint: Duration::from_secs(10),
        process_id: None,
    })
}

fn random_delay() -> Duration {
    let secs = rand::thread_rng().gen_range(MIN_DELAY_SECS..=MAX_DELAY_SECS);
    Duration::from_secs(secs)
}

fn log(msg: &str) {
    let path = std::env::temp_dir().join("brequet-services.log");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = writeln!(f, "[{now}] {msg}");
    }
}
