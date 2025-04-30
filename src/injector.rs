use std::{
    ffi::c_void,
    error::Error
};

use windows::{
    core::{s},
    Win32::{
        Foundation::{HANDLE},
        System::{
            Diagnostics::{
                Debug::{WriteProcessMemory}
            },
            Threading::{CreateRemoteThread, OpenProcess, PROCESS_VM_OPERATION, PROCESS_VM_WRITE},
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE}
        }
    }
};

pub fn inject_process(dll_path: &str, pid: u32) -> Result<HANDLE, Box<dyn Error>> {

    // Get writable handle to process
    let process_handle = unsafe { OpenProcess(PROCESS_VM_OPERATION | PROCESS_VM_WRITE, false, pid) };
    let process_handle = match process_handle {
        Ok(h) => {
            println!("[+] Got handle to process ID {pid}, handle: {:?}", h);
            h
        },
        Err(e) => panic!("[-] Could not get handle to pid {pid}, error: {e}"),
    };

    // Get handle to kernel32.dll
    let kernel32_handle = unsafe { GetModuleHandleA(s!("Kernel32.dll")) };
    let kernel32_handle = match kernel32_handle {
        Ok(h) => {
            println!("[+] Handle to Kernel32.dll: {:?}", h);
            h
        }
        Err (e) => panic!("[-] Could not get handle to Kernel32.dll, {e}"),
    };

    // Find LoadLibraryA address in kernel32.dll
    let load_library_address = unsafe { GetProcAddress(kernel32_handle, s!("LoadLibraryA")) };
    let load_library_address = match load_library_address {
        None => panic!("[-] Could not resolve the address of LoadLibraryA"),
        Some(address) => {
            let address = address as *const ();
            println!("[+] Address of LoadLibraryA: {:p}", address);
            address
        }
    };

    // Allocate memory in process to write dll path
    let base_address = unsafe {
        VirtualAllocEx(process_handle,
                       None,
                       size_of_val(dll_path),
                       MEM_COMMIT | MEM_RESERVE,
                       PAGE_EXECUTE_READWRITE,
        )
    };

    if base_address.is_null() {
        panic!("[-] Failed allocating memory into remote process for DLL Path");
    }

    println!("[+] Remote buffer base address: {:p}", base_address);

    // Write the dll path into the allocated memory space
    let mut bytes : usize = 0;
    let result = unsafe {
        WriteProcessMemory(
            process_handle,
            base_address,
            dll_path.as_ptr() as *const c_void,
            size_of_val(dll_path),
            Some(&mut bytes as *mut usize)
        )
    };

    match result {
        Ok(_) => println!("[+] Bytes written to remote process: {:?}", bytes),
        Err(e) => panic!("[-] Error writing remote process memory: {e}"),
    }

    // Cast the LoadLibraryA address to a function pointer
    let load_library_address : Option<unsafe extern "system" fn(*mut c_void) -> u32> = Some (
        unsafe { std::mem::transmute(load_library_address) }
    );

    // Create thread in process to run the dll
    let mut thread : u32 = 0;
    let thread_handle = unsafe {
        CreateRemoteThread(
            process_handle,
            None,
            0,
            load_library_address,
            Some(base_address),
            0,
            Some(&mut thread as *mut u32)
        )
    };

    match thread_handle {
        Ok(h) => {
            println!("[+] Thread started, handle: {:?}", h);
            Ok(h)
        },
        Err(e) => panic!("[-] Error occurred creating thread: {e}"),
    }
}
