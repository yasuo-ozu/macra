//! Windows hook via DLL injection + IAT patching.
//!
//! When this DLL is loaded into a rustc process (via DllMain), it patches
//! the Import Address Table (IAT) to redirect `GetProcAddress` calls through
//! our hook. The hook checks for `__rustc_proc_macro_decls_*` symbols and
//! wraps them via `trampoline::intercept_proc_macro_table()`.

use crate::trampoline;
use std::ffi::CStr;
use std::sync::atomic::{AtomicPtr, Ordering};

// PE header constants (stable ABI)
const IMAGE_DOS_SIGNATURE: u16 = 0x5A4D; // "MZ"
const IMAGE_NT_SIGNATURE: u32 = 0x00004550; // "PE\0\0"

// Offsets within IMAGE_OPTIONAL_HEADER64 (PE32+)
const OPTIONAL_HEADER_NUMBER_OF_RVA_AND_SIZES_OFFSET: usize = 108;
const OPTIONAL_HEADER_DATA_DIRECTORY_OFFSET: usize = 112;
const DATA_DIRECTORY_ENTRY_SIZE: usize = 8;
const IMAGE_DIRECTORY_ENTRY_IMPORT: usize = 1;

// IMAGE_FILE_HEADER is 20 bytes; comes after the 4-byte signature
const FILE_HEADER_SIZE: usize = 20;
// Offset of SizeOfOptionalHeader within IMAGE_FILE_HEADER
const FILE_HEADER_SIZE_OF_OPTIONAL_HEADER_OFFSET: usize = 16;

// IMAGE_IMPORT_DESCRIPTOR layout (20 bytes each)
const IMPORT_DESC_ORIGINAL_FIRST_THUNK: usize = 0;
const IMPORT_DESC_NAME: usize = 12;
const IMPORT_DESC_FIRST_THUNK: usize = 16;
const IMPORT_DESC_SIZE: usize = 20;

/// Saved pointer to the real `GetProcAddress`.
static REAL_GET_PROC_ADDRESS: AtomicPtr<()> = AtomicPtr::new(std::ptr::null_mut());

type GetProcAddressFn = unsafe extern "system" fn(
    windows_sys::Win32::Foundation::HMODULE,
    *const u8,
) -> Option<unsafe extern "system" fn()>;

/// Read a u16 from a pointer offset (unaligned-safe).
unsafe fn read_u16(base: *const u8, offset: usize) -> u16 {
    unsafe { (base.add(offset) as *const u16).read_unaligned() }
}

/// Read a u32 from a pointer offset (unaligned-safe).
unsafe fn read_u32(base: *const u8, offset: usize) -> u32 {
    unsafe { (base.add(offset) as *const u32).read_unaligned() }
}

/// Read a usize from a pointer offset (unaligned-safe).
unsafe fn read_usize(base: *const u8, offset: usize) -> usize {
    unsafe { (base.add(offset) as *const usize).read_unaligned() }
}

/// Called from DllMain(DLL_PROCESS_ATTACH) to install the IAT hook.
pub(crate) fn install_hook() {
    unsafe {
        let kernel32 = windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(
            b"kernel32.dll\0".as_ptr(),
        );
        if kernel32.is_null() {
            return;
        }

        let real_gpa = windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            kernel32,
            b"GetProcAddress\0".as_ptr(),
        );
        let real_gpa = match real_gpa {
            Some(f) => f,
            None => return,
        };

        REAL_GET_PROC_ADDRESS.store(real_gpa as *mut (), Ordering::Release);

        patch_all_modules(real_gpa as usize);
    }
}

/// Enumerate loaded modules and patch IAT in each.
unsafe fn patch_all_modules(real_gpa_addr: usize) {
    let snapshot = unsafe {
        windows_sys::Win32::System::Diagnostics::ToolHelp::CreateToolhelp32Snapshot(
            windows_sys::Win32::System::Diagnostics::ToolHelp::TH32CS_SNAPMODULE,
            0,
        )
    };
    if snapshot == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
        // Fallback: patch the main exe module only
        let main_module = unsafe {
            windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(std::ptr::null())
        };
        if !main_module.is_null() {
            unsafe { patch_module_iat(main_module as *const u8, real_gpa_addr) };
        }
        return;
    }

    let mut entry: windows_sys::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32W =
        unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<
        windows_sys::Win32::System::Diagnostics::ToolHelp::MODULEENTRY32W,
    >() as u32;

    let mut ok = unsafe {
        windows_sys::Win32::System::Diagnostics::ToolHelp::Module32FirstW(snapshot, &mut entry)
    };

    while ok != 0 {
        let base = entry.modBaseAddr;
        if !base.is_null() {
            unsafe { patch_module_iat(base, real_gpa_addr) };
        }
        ok = unsafe {
            windows_sys::Win32::System::Diagnostics::ToolHelp::Module32NextW(snapshot, &mut entry)
        };
    }

    unsafe {
        windows_sys::Win32::Foundation::CloseHandle(snapshot);
    }
}

/// Patch a single module's IAT to redirect GetProcAddress to our hook.
///
/// Uses raw offset-based PE parsing to avoid struct layout bugs.
unsafe fn patch_module_iat(base: *const u8, real_gpa_addr: usize) {
    // Validate DOS header
    if unsafe { read_u16(base, 0) } != IMAGE_DOS_SIGNATURE {
        return;
    }

    // e_lfanew is at offset 60 in IMAGE_DOS_HEADER
    let e_lfanew = unsafe { read_u32(base, 60) } as usize;
    let nt_headers = unsafe { base.add(e_lfanew) };

    // Validate NT signature
    if unsafe { read_u32(nt_headers, 0) } != IMAGE_NT_SIGNATURE {
        return;
    }

    // Optional header starts after signature (4) + file header (20)
    let optional_header = unsafe { nt_headers.add(4 + FILE_HEADER_SIZE) };

    // Verify this is PE32+ (magic == 0x20b) — we only support 64-bit
    let magic = unsafe { read_u16(optional_header, 0) };
    if magic != 0x20b {
        return;
    }

    // Read NumberOfRvaAndSizes
    let num_dirs =
        unsafe { read_u32(optional_header, OPTIONAL_HEADER_NUMBER_OF_RVA_AND_SIZES_OFFSET) }
            as usize;
    if num_dirs <= IMAGE_DIRECTORY_ENTRY_IMPORT {
        return;
    }

    // Read import directory entry (RVA + Size)
    let import_dir_offset = OPTIONAL_HEADER_DATA_DIRECTORY_OFFSET
        + IMAGE_DIRECTORY_ENTRY_IMPORT * DATA_DIRECTORY_ENTRY_SIZE;
    let import_rva = unsafe { read_u32(optional_header, import_dir_offset) } as usize;
    let import_size = unsafe { read_u32(optional_header, import_dir_offset + 4) } as usize;

    if import_rva == 0 || import_size == 0 {
        return;
    }

    // Walk import descriptors
    let mut desc_ptr = unsafe { base.add(import_rva) };

    loop {
        let first_thunk = unsafe { read_u32(desc_ptr, IMPORT_DESC_FIRST_THUNK) } as usize;
        let original_first_thunk =
            unsafe { read_u32(desc_ptr, IMPORT_DESC_ORIGINAL_FIRST_THUNK) } as usize;

        if first_thunk == 0 && original_first_thunk == 0 {
            break; // End of import descriptor array
        }

        let name_rva = unsafe { read_u32(desc_ptr, IMPORT_DESC_NAME) } as usize;
        if name_rva != 0 {
            let dll_name_ptr = unsafe { base.add(name_rva) } as *const i8;
            if let Ok(dll_name) = unsafe { CStr::from_ptr(dll_name_ptr) }.to_str() {
                let lower = dll_name.to_ascii_lowercase();
                if lower == "kernel32.dll"
                    || lower.starts_with("api-ms-win-core-libraryloader")
                {
                    unsafe {
                        patch_import_thunks(
                            base,
                            original_first_thunk,
                            first_thunk,
                            real_gpa_addr,
                        );
                    }
                }
            }
        }

        desc_ptr = unsafe { desc_ptr.add(IMPORT_DESC_SIZE) };
    }
}

/// Patch GetProcAddress thunks in a single import descriptor.
unsafe fn patch_import_thunks(
    base: *const u8,
    original_first_thunk_rva: usize,
    first_thunk_rva: usize,
    real_gpa_addr: usize,
) {
    // Use OriginalFirstThunk (INT) for name lookup, FirstThunk (IAT) for patching.
    let int_rva = if original_first_thunk_rva != 0 {
        original_first_thunk_rva
    } else {
        first_thunk_rva
    };

    let mut int_entry = unsafe { base.add(int_rva) } as *const usize;
    let mut iat_entry = unsafe { base.add(first_thunk_rva) } as *mut usize;

    loop {
        let thunk_data = unsafe { *int_entry };
        if thunk_data == 0 {
            break;
        }

        // Check if this is an ordinal import (high bit set) — skip those
        let is_ordinal = thunk_data & (1usize << (usize::BITS - 1)) != 0;

        if !is_ordinal {
            // thunk_data is an RVA to IMAGE_IMPORT_BY_NAME
            // The name starts at offset 2 (after the Hint u16)
            let name_ptr = unsafe { base.add(thunk_data + 2) } as *const i8;
            if let Ok(name) = unsafe { CStr::from_ptr(name_ptr) }.to_str() {
                if name == "GetProcAddress" {
                    let current = unsafe { *iat_entry };
                    if current == real_gpa_addr {
                        let mut old_protect: u32 = 0;
                        let ok = unsafe {
                            windows_sys::Win32::System::Memory::VirtualProtect(
                                iat_entry as *const _,
                                std::mem::size_of::<usize>(),
                                windows_sys::Win32::System::Memory::PAGE_READWRITE,
                                &mut old_protect,
                            )
                        };
                        if ok != 0 {
                            unsafe {
                                *iat_entry = hooked_get_proc_address as usize;
                            }
                            let mut dummy: u32 = 0;
                            unsafe {
                                windows_sys::Win32::System::Memory::VirtualProtect(
                                    iat_entry as *const _,
                                    std::mem::size_of::<usize>(),
                                    old_protect,
                                    &mut dummy,
                                );
                            }
                        }
                    }
                }
            }
        }

        int_entry = unsafe { int_entry.add(1) };
        iat_entry = unsafe { iat_entry.add(1) };
    }
}

/// Our hooked GetProcAddress. Calls the real GetProcAddress, then checks
/// if the symbol matches `__rustc_proc_macro_decls_*` to intercept proc
/// macro tables.
unsafe extern "system" fn hooked_get_proc_address(
    hmodule: windows_sys::Win32::Foundation::HMODULE,
    lp_proc_name: *const u8,
) -> Option<unsafe extern "system" fn()> {
    let real_fn_ptr = REAL_GET_PROC_ADDRESS.load(Ordering::Acquire);
    if real_fn_ptr.is_null() {
        return None;
    }

    let real_gpa: GetProcAddressFn = unsafe { std::mem::transmute(real_fn_ptr) };
    let result = unsafe { real_gpa(hmodule, lp_proc_name) };

    // Only check named imports (not ordinals). Ordinal imports have the
    // high word of lp_proc_name as 0 (HIWORD == 0).
    if lp_proc_name.is_null() || (lp_proc_name as usize) < 0x10000 {
        return result;
    }

    let result = match result {
        Some(f) => f,
        None => return None,
    };

    let sym_name = unsafe { CStr::from_ptr(lp_proc_name as *const i8) };
    let sym_bytes = sym_name.to_bytes();

    if !sym_bytes.starts_with(b"__rustc_proc_macro_decls_") {
        return Some(result);
    }

    // Intercept the proc macro table
    let intercepted =
        unsafe { trampoline::intercept_proc_macro_table(result as *mut libc::c_void) };
    Some(unsafe { std::mem::transmute(intercepted) })
}
