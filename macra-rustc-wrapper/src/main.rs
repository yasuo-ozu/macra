//! RUSTC_WRAPPER binary for macra-hook DLL injection on Windows.
//!
//! Cargo invokes this wrapper for every `rustc` call. On Windows, the wrapper
//! creates the real rustc as a suspended process, injects the hook DLL via
//! `CreateRemoteThread(LoadLibraryW)`, then resumes rustc.
//!
//! On non-Windows platforms, this simply execs the real rustc directly.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // RUSTC_WRAPPER convention: argv[1] = rustc path, argv[2..] = rustc args
    if args.len() < 2 {
        eprintln!("[macra-rustc-wrapper] usage: wrapper <rustc> [args...]");
        std::process::exit(1);
    }
    let rustc = &args[1];
    let rustc_args = &args[2..];

    #[cfg(windows)]
    {
        std::process::exit(windows_inject::run(rustc, rustc_args));
    }

    #[cfg(not(windows))]
    {
        // On non-Windows, just exec the real rustc
        let status = std::process::Command::new(rustc)
            .args(rustc_args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("[macra-rustc-wrapper] failed to run rustc: {}", e);
                std::process::exit(1);
            });
        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(windows)]
mod windows_inject {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::System::Diagnostics::Debug::*;
    use windows_sys::Win32::System::Memory::*;
    use windows_sys::Win32::System::Threading::*;

    /// Build a null-terminated UTF-16 string from an `&str`.
    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
    }

    /// Build the command line string for CreateProcessW.
    fn build_command_line(rustc: &str, rustc_args: &[String]) -> Vec<u16> {
        let mut cmdline = format!("\"{}\"", rustc);
        for arg in rustc_args {
            cmdline.push(' ');
            if arg.contains(' ') || arg.contains('"') {
                cmdline.push('"');
                cmdline.push_str(&arg.replace('"', "\\\""));
                cmdline.push('"');
            } else {
                cmdline.push_str(arg);
            }
        }
        to_wide(&cmdline)
    }

    pub fn run(rustc: &str, rustc_args: &[String]) -> i32 {
        let dll_path = match std::env::var("MACRA_HOOK_DLL_PATH") {
            Ok(p) if !p.is_empty() => p,
            _ => {
                // No DLL to inject — just run rustc normally
                return run_passthrough(rustc, rustc_args);
            }
        };

        // Verify the DLL exists
        if !std::path::Path::new(&dll_path).exists() {
            eprintln!("[macra-rustc-wrapper] DLL not found: {}", dll_path);
            return run_passthrough(rustc, rustc_args);
        }

        unsafe { run_with_injection(rustc, rustc_args, &dll_path) }
    }

    fn run_passthrough(rustc: &str, rustc_args: &[String]) -> i32 {
        let status = std::process::Command::new(rustc)
            .args(rustc_args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("[macra-rustc-wrapper] failed to run rustc: {}", e);
                std::process::exit(1);
            });
        status.code().unwrap_or(1)
    }

    unsafe fn run_with_injection(rustc: &str, rustc_args: &[String], dll_path: &str) -> i32 {
        let mut cmdline = build_command_line(rustc, rustc_args);
        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        // Create rustc process in SUSPENDED state
        let ok = unsafe {
            CreateProcessW(
                std::ptr::null(),
                cmdline.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                TRUE, // inherit handles
                CREATE_SUSPENDED,
                std::ptr::null(),
                std::ptr::null(),
                &si,
                &mut pi,
            )
        };

        if ok == 0 {
            eprintln!(
                "[macra-rustc-wrapper] CreateProcessW failed: {}",
                unsafe { GetLastError() }
            );
            return run_passthrough(rustc, rustc_args);
        }

        // Inject the DLL
        let injected = unsafe { inject_dll(pi.hProcess, dll_path) };
        if !injected {
            eprintln!("[macra-rustc-wrapper] DLL injection failed, continuing without hook");
        }

        // Resume the main thread
        unsafe { ResumeThread(pi.hThread) };

        // Wait for the process to exit
        unsafe { WaitForSingleObject(pi.hProcess, INFINITE) };

        let mut exit_code: u32 = 1;
        unsafe { GetExitCodeProcess(pi.hProcess, &mut exit_code) };

        unsafe {
            CloseHandle(pi.hThread);
            CloseHandle(pi.hProcess);
        }

        exit_code as i32
    }

    /// Inject a DLL into a suspended process using CreateRemoteThread + LoadLibraryW.
    unsafe fn inject_dll(process: HANDLE, dll_path: &str) -> bool {
        let dll_wide = to_wide(dll_path);
        let dll_bytes_len = dll_wide.len() * 2; // size in bytes

        // Allocate memory in the remote process for the DLL path
        let remote_mem = unsafe {
            VirtualAllocEx(
                process,
                std::ptr::null(),
                dll_bytes_len,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };

        if remote_mem.is_null() {
            eprintln!(
                "[macra-rustc-wrapper] VirtualAllocEx failed: {}",
                unsafe { GetLastError() }
            );
            return false;
        }

        // Write the DLL path into the remote process
        let mut written: usize = 0;
        let ok = unsafe {
            WriteProcessMemory(
                process,
                remote_mem,
                dll_wide.as_ptr() as *const _,
                dll_bytes_len,
                &mut written,
            )
        };

        if ok == 0 {
            eprintln!(
                "[macra-rustc-wrapper] WriteProcessMemory failed: {}",
                unsafe { GetLastError() }
            );
            unsafe { VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE) };
            return false;
        }

        // Get the address of LoadLibraryW in kernel32.dll.
        // kernel32.dll is mapped at the same address in all processes of the
        // same architecture on the same boot, so the function pointer is valid
        // in the remote process.
        let kernel32 = to_wide("kernel32.dll");
        let k32_handle = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(kernel32.as_ptr())
        };
        if k32_handle.is_null() {
            eprintln!("[macra-rustc-wrapper] GetModuleHandleW(kernel32) failed");
            unsafe { VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE) };
            return false;
        }

        let load_library_addr = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetProcAddress(
                k32_handle,
                b"LoadLibraryW\0".as_ptr(),
            )
        };
        if load_library_addr.is_none() {
            eprintln!("[macra-rustc-wrapper] GetProcAddress(LoadLibraryW) failed");
            unsafe { VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE) };
            return false;
        }

        // Create a remote thread that calls LoadLibraryW(dll_path)
        let thread = unsafe {
            CreateRemoteThread(
                process,
                std::ptr::null(),
                0,
                Some(std::mem::transmute(load_library_addr.unwrap())),
                remote_mem,
                0,
                std::ptr::null_mut(),
            )
        };

        if thread.is_null() {
            eprintln!(
                "[macra-rustc-wrapper] CreateRemoteThread failed: {}",
                unsafe { GetLastError() }
            );
            unsafe { VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE) };
            return false;
        }

        // Wait for the remote thread (DLL loading) to complete
        unsafe { WaitForSingleObject(thread, INFINITE) };

        // Clean up
        unsafe {
            CloseHandle(thread);
            VirtualFreeEx(process, remote_mem, 0, MEM_RELEASE);
        }

        true
    }
}
