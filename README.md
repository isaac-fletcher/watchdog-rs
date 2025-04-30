# watchdog-rs

## about
This is a reimplementation of Watchdog in Rust. Watchdog injects a DLL into a process of choice. After the thread for the DLL is created, it is watched to ensure that it is still running. If Watchdog detects that the thread is killed, it will attempt to create it again.

If the DLL is missing from the system, it Watchdog will redownload it from a remote source and then attempt to inject it.

Behavior of Watchdog is configurable from a set of variables defined at the top of the main file:
```
const PROCESS_NAME : &str = "notepad.exe";
const DLL_PATH : &str = "C:\\Windows\\System32\\hijack.dll";
const SERVICE_NAME : &str = "Watchdog";
const DOWNLOAD_URL : &str = "http://127.0.0.1";
```

## usage
- Clone the repo and edit the main file with your desired configuration
- Compile the Watchdog executable with `cargo compile`
- Compile an x64 DLL to be used for injection
- Create the service as shown below:
```
sc.exe create "Watchdog" binPath= "C:\PATH\TO\EXECUTABLE" start=auto
```

## credits
- Inspiration for the project from the [RITSEC Red Team](github.com/RITRedteam/Watchdog/tree/master/Watchdog).
- Process injection code derived from [0xflux](https://fluxsec.red/remote-process-dll-injection).
