//! Linux dlsym hook via LD_PRELOAD.
//!
//! We export a `dlsym` function that intercepts lookups for
//! `__rustc_proc_macro_decls_*` symbols, wrapping the returned
//! proc macro table with our trampoline interceptors.

use crate::trampoline;
use std::cell::Cell;
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicPtr, Ordering};

unsafe extern "C" {
    fn dlvsym(
        handle: *mut libc::c_void,
        symbol: *const libc::c_char,
        version: *const libc::c_char,
    ) -> *mut libc::c_void;
}

type DlsymFn = unsafe extern "C" fn(*mut libc::c_void, *const libc::c_char) -> *mut libc::c_void;

/// The real `dlsym`, resolved once per process. Null until the first call.
static REAL_DLSYM: AtomicPtr<libc::c_void> = AtomicPtr::new(std::ptr::null_mut());

thread_local! {
    /// Set while this thread is inside `resolve_real_dlsym`.
    ///
    /// The fallback in there reads libc from disk through our statically linked
    /// `std`, and `std` looks up weak symbols (`statx`, `__pthread_get_minstack`, ..)
    /// with `dlsym` — which is *this* hook, since LD_PRELOAD puts us first. Without
    /// the guard that lookup would recurse into an unfinished resolution forever.
    static RESOLVING: Cell<bool> = const { Cell::new(false) };
}

/// Call the real `dlsym`.
///
/// Our `dlsym` replaces the default one, so calling `dlsym(RTLD_NEXT, "dlsym")` would
/// recurse. `dlvsym` with an explicit version string bypasses the hook, and it is the
/// only libc-provided way to reach the real one: `_dl_sym` is not exported since 2.34.
unsafe fn real_dlsym(handle: *mut libc::c_void, symbol: *const libc::c_char) -> *mut libc::c_void {
    let mut ptr = REAL_DLSYM.load(Ordering::Acquire);
    if ptr.is_null() {
        if RESOLVING.with(|r| r.get()) {
            // A `dlsym` issued by our own resolution code (see `RESOLVING`). Only our
            // `std`'s weak-symbol probes can get here, and null is the answer they
            // are built to handle: they fall back to a raw syscall or a default.
            return std::ptr::null_mut();
        }
        RESOLVING.with(|r| r.set(true));
        ptr = resolve_real_dlsym();
        RESOLVING.with(|r| r.set(false));
        // Two threads racing here both find the same address; storing twice is fine.
        REAL_DLSYM.store(ptr, Ordering::Release);
    }

    let real: DlsymFn = unsafe { std::mem::transmute(ptr) };
    unsafe { real(handle, symbol) }
}

/// Find the real `dlsym` through `dlvsym`, or abort the process.
///
/// `dlvsym` needs the version node the symbol is defined under, and that is glibc's
/// *baseline* for the architecture, which differs per target: `GLIBC_2.2.5` is
/// x86_64 only. Hard-coding it once made `dlvsym` return null everywhere else, and
/// since that failure fell through to returning null for *every* symbol, the hook
/// broke all `dlsym` in the process — rustc could not load any dylib. The known
/// baselines below fixed that for the architectures that were known at the time.
///
/// A list can never be complete, though. `GLIBC_2.34` exists on every architecture
/// whose baseline predates 2.34 (it is where `dlsym` moved into libc), but a port
/// whose baseline is *later* has no such node: loongarch64 starts at `GLIBC_2.36`,
/// and the next port will start later still. So the list is only the fast path; if
/// nothing on it resolves, the version names are read from the verdef records of the
/// libc that is actually loaded, which by construction include whatever node its
/// `dlsym` is defined under. That needs `dladdr` and a file read, hence the cache in
/// `REAL_DLSYM` and the ordering: no I/O on any platform the list already covers.
///
/// If even that fails, abort with a message naming macra. LD_PRELOAD is set on
/// `cargo` itself, so this hook is inside every rustc, build script, `cc` and `ld`
/// of the build; a `dlsym` that answers null to all of them turns into a fan of
/// unrelated-looking failures ("cannot load dylib", linker errors) with nothing
/// pointing back here. One visible abort that says what it is and how to get out is
/// strictly better than silently corrupting an unrelated toolchain.
fn resolve_real_dlsym() -> *mut libc::c_void {
    let dlsym_name = c"dlsym".as_ptr();
    let resolve_with =
        |version: &CStr| unsafe { dlvsym(libc::RTLD_NEXT, dlsym_name, version.as_ptr()) };

    const GLIBC_BASELINES: [&CStr; 7] = [
        c"GLIBC_2.34",  // every arch with a baseline < 2.34 (dlsym moved into libc)
        c"GLIBC_2.2.5", // x86_64
        c"GLIBC_2.17",  // aarch64, ppc64le
        c"GLIBC_2.27",  // riscv64
        c"GLIBC_2.4",   // arm
        c"GLIBC_2.2",   // s390x
        c"GLIBC_2.0",   // i686
    ];
    for version in GLIBC_BASELINES {
        let ptr = resolve_with(version);
        if !ptr.is_null() {
            return ptr;
        }
    }

    // Slow path: ask the loaded libc which version nodes it defines.
    let (libc_path, versions) = libc_version_definitions().unwrap_or_default();
    for version in &versions {
        let ptr = resolve_with(version);
        if !ptr.is_null() {
            return ptr;
        }
    }

    eprintln!(
        "[macra-hook] cannot resolve the real `dlsym`: `dlvsym(RTLD_NEXT, \"dlsym\", v)` \
         returned null for every known glibc baseline {:?} and for every version node \
         defined by {} ({:?}).\n\
         [macra-hook] Aborting rather than answering null to every `dlsym` in this \
         process, which would break every dylib load in the build. This is the \
         cargo-macra proc-macro hook (LD_PRELOAD); run the build without `cargo macra` \
         to proceed, and please report your architecture and glibc version at \
         https://github.com/yasuo-ozu/macra/issues",
        GLIBC_BASELINES,
        if libc_path.is_empty() {
            "an unlocatable libc".to_string()
        } else {
            libc_path
        },
        versions,
    );
    std::process::abort();
}

/// The path of the object that provides `dlvsym` (libc.so.6, or libdl.so.2 before
/// glibc 2.34 — `dlsym` lives in the same one) and the version nodes it defines.
///
/// The names come from the file's `SHT_GNU_VERDEF` section rather than from the
/// in-memory dynamic section: `DT_VERDEF` is never relocated in place by ld.so while
/// `DT_STRTAB` is on some architectures and not others (`DL_RO_DYN_SECTION`), so
/// reading them from memory would need per-architecture guesses — the very thing
/// this function exists to avoid.
fn libc_version_definitions() -> Option<(String, Vec<CString>)> {
    let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
    if unsafe { libc::dladdr(dlvsym as *const libc::c_void, &mut info) } == 0
        || info.dli_fname.is_null()
    {
        return None;
    }
    let path = unsafe { CStr::from_ptr(info.dli_fname) }
        .to_str()
        .ok()?
        .to_string();
    let data = std::fs::read(&path).ok()?;
    let names = match object::FileKind::parse(&*data).ok()? {
        object::FileKind::Elf32 => {
            verdef_names::<object::elf::FileHeader32<object::Endianness>>(&data)?
        }
        object::FileKind::Elf64 => {
            verdef_names::<object::elf::FileHeader64<object::Endianness>>(&data)?
        }
        _ => return None,
    };
    Some((path, names))
}

/// Every version name an ELF object defines, in definition order.
fn verdef_names<Elf: object::read::elf::FileHeader<Endian = object::Endianness>>(
    data: &[u8],
) -> Option<Vec<CString>> {
    use object::read::elf::ElfFile;
    let file = ElfFile::<Elf>::parse(data).ok()?;
    let endian = file.endian();
    let sections = file.elf_section_table();
    let (defs, strings_index) = sections.gnu_verdef(endian, data).ok()??;
    let strings = sections.strings(endian, data, strings_index).ok()?;

    let mut names = Vec::new();
    for def in defs {
        let (def, mut aux) = def.ok()?;
        // The base definition is the object's own soname, not a version node.
        if def.vd_flags.get(endian) & object::elf::VER_FLG_BASE != 0 {
            continue;
        }
        // The first auxiliary entry names the version; later ones name its parents.
        if let Ok(Some(aux)) = aux.next() {
            if let Ok(name) = aux.name(endian, strings) {
                if let Ok(name) = CString::new(name) {
                    names.push(name);
                }
            }
        }
    }
    Some(names)
}

/// Hooked `dlsym` — exported as the public dlsym symbol via LD_PRELOAD.
///
/// When rustc calls `dlsym(handle, "__rustc_proc_macro_decls_...")`, we intercept
/// the result and wrap the proc macro table with our trampolines.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dlsym(
    handle: *mut libc::c_void,
    symbol: *const libc::c_char,
) -> *mut libc::c_void {
    // Always call through to the real dlsym first
    let result = unsafe { real_dlsym(handle, symbol) };

    if result.is_null() || symbol.is_null() {
        return result;
    }

    // Check if this is a proc macro decls symbol
    let sym_name = unsafe { CStr::from_ptr(symbol) };
    let sym_bytes = sym_name.to_bytes();

    if !sym_bytes.starts_with(b"__rustc_proc_macro_decls_") {
        return result;
    }

    // Only reinterpret the table when cargo-macra has told us which layout this
    // compiler uses. Without the handshake the safe move is to hand rustc its own
    // pointer back untouched: guessing walks the table at the wrong stride and
    // aborts the compiler, which surfaces as an unrelated crate failing to build.
    let Some(abi) = crate::types::selected_abi() else {
        if std::env::var_os("MACRA_HOOK_DEBUG").is_some() {
            eprintln!(
                "[macra-hook] saw decls symbol but MACRA_ABI is {:?}",
                std::env::var("MACRA_ABI")
            );
        }
        return result;
    };

    // result is a pointer to `static DECLS: &[ProcMacro]` (a thin pointer to a fat pointer).
    // Pass it to our interception logic which returns a pointer to a new fat pointer.
    match abi.table {
        cargo_macra::TableLayout::ProcMacroEnum => unsafe {
            trampoline::intercept_proc_macro_table(result)
        },
        cargo_macra::TableLayout::ClientSlice => unsafe {
            trampoline::intercept_client_slice_table(result)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The slow path must work on its own, not only as an untested fallback: it is
    /// what a port whose baseline post-dates every entry in `GLIBC_BASELINES` runs.
    #[test]
    fn loaded_libc_defines_a_version_under_which_dlsym_resolves() {
        let (path, versions) = libc_version_definitions().expect("dladdr + verdef scan");
        assert!(!path.is_empty());
        assert!(
            versions
                .iter()
                .any(|v| v.to_bytes().starts_with(b"GLIBC_2.")),
            "no GLIBC_2.* node among {versions:?} from {path}"
        );
        let found = versions.iter().find_map(|v| {
            let ptr = unsafe { dlvsym(libc::RTLD_NEXT, c"dlsym".as_ptr(), v.as_ptr()) };
            (!ptr.is_null()).then_some(ptr)
        });
        assert!(
            found.is_some(),
            "none of {versions:?} from {path} defines dlsym"
        );
    }

    #[test]
    fn resolved_dlsym_is_the_real_one() {
        let ptr = resolve_real_dlsym();
        assert!(!ptr.is_null());
        let real: DlsymFn = unsafe { std::mem::transmute(ptr) };
        // Something every libc exports, looked up without going through our hook.
        let malloc = unsafe { real(libc::RTLD_DEFAULT, c"malloc".as_ptr()) };
        assert!(!malloc.is_null());
    }

    /// The exported hook itself — the test binary's `dlsym` *is* this function —
    /// must forward ordinary lookups and hand back null for unknown symbols instead
    /// of, say, aborting or recursing.
    #[test]
    fn hooked_dlsym_forwards_ordinary_lookups() {
        let malloc = unsafe { dlsym(libc::RTLD_DEFAULT, c"malloc".as_ptr()) };
        assert!(!malloc.is_null());
        let none = unsafe { dlsym(libc::RTLD_DEFAULT, c"__macra_no_such_symbol__".as_ptr()) };
        assert!(none.is_null());
    }
}
