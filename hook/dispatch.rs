//! Dispatch closure interception for capturing proc macro input/output strings.
//!
//! The rustc bridge protocol sends RPC requests through a dispatch closure.
//! We wrap this closure to intercept token stream operations and capture
//! the string representations of proc macro inputs and outputs.
//!
//! Protocol format (two-byte method tags, matching `with_api!` enum order):
//! - Byte 0: `Method` enum variant (0=FreeFunctions, 1=TokenStream, 2=Span, 3=Symbol)
//! - Byte 1: method index within the group
//! - Handle: u32_le
//! - String encoding: usize_le length + UTF-8 bytes

use crate::types::{Buffer, Closure};
use std::cell::RefCell;

/// Captured data from a proc macro invocation
#[derive(Debug, Default, Clone)]
pub struct CapturedStrings {
    /// Strings passed to `ts_from_str` (proc macro creating output tokens from string)
    pub from_str_calls: Vec<String>,
    /// Strings from `ts_to_string` calls (if the proc macro calls it explicitly)
    pub to_string_results: Vec<String>,
}

thread_local! {
    /// The original dispatch closure for the current invocation
    static ORIGINAL_DISPATCH: RefCell<Option<OriginalDispatch>> = const { RefCell::new(None) };

    /// Captured strings from the current invocation
    static CAPTURED: RefCell<CapturedStrings> = RefCell::new(CapturedStrings::default());
}

struct OriginalDispatch {
    call: unsafe extern "C" fn(*mut u8, Buffer) -> Buffer,
    env: *mut u8,
}

// Safety: The dispatch closure is only used within a single thread during a proc macro invocation.
unsafe impl Send for OriginalDispatch {}

/// Bridge API method group tag for TokenStream methods.
const METHOD_TOKEN_STREAM: u8 = 0x01;

/// TokenStream method indices (from `with_api!` enum order):
/// drop=0, clone=1, is_empty=2, expand_expr=3, from_str=4, to_string=5, ...
const TS_FROM_STR: u8 = 0x04;
const TS_TO_STRING: u8 = 0x05;

/// Flat `ApiTags` indices used from rustc 1.100.
///
/// `with_api!` used to nest methods under a type (`TokenStream { .. }`), so a
/// request began with a group tag and a method tag. It is now one flat list
/// declared as `#[repr(u8)] enum ApiTags`, encoded as a single byte:
///
/// ```text
/// 0 track_env_var  2 literal_from_str  4 ts_drop   6 ts_is_empty     8 ts_from_str
/// 1 track_path     3 emit_diagnostic   5 ts_clone  7 ts_expand_expr  9 ts_to_string
/// ```
const FLAT_TS_FROM_STR: u8 = 8;
const FLAT_TS_TO_STRING: u8 = 9;

/// How the running compiler encodes an RPC request tag.
fn flat_tags() -> bool {
    matches!(
        crate::types::selected_abi(),
        Some(crate::types::TableAbi::ClientSlice)
    )
}

/// Split a request into `(from_str, to_string)` predicates for the ABI in use.
///
/// Older compilers prefix the tag with a group byte; newer ones do not, so the
/// argument payload also starts one byte earlier.
fn classify_request(data: &[u8]) -> (bool, bool, usize) {
    if flat_tags() {
        let tag = data.first().copied();
        (
            tag == Some(FLAT_TS_FROM_STR),
            tag == Some(FLAT_TS_TO_STRING),
            1,
        )
    } else {
        let group = data.first().copied();
        let index = data.get(1).copied();
        let is_ts = group == Some(METHOD_TOKEN_STREAM);
        (
            is_ts && index == Some(TS_FROM_STR),
            is_ts && index == Some(TS_TO_STRING),
            2,
        )
    }
}

/// Encode a `ts_to_string` request for the ABI in use.
fn ts_to_string_request(handle: u32) -> Vec<u8> {
    let mut request = if flat_tags() {
        vec![FLAT_TS_TO_STRING]
    } else {
        vec![METHOD_TOKEN_STREAM, TS_TO_STRING]
    };
    request.extend_from_slice(&handle.to_le_bytes());
    request
}

/// Read a usize in little-endian from a byte slice at the given offset.
/// On 64-bit systems, usize is 8 bytes.
fn read_usize_le(data: &[u8], offset: usize) -> Option<usize> {
    let size = std::mem::size_of::<usize>();
    if offset + size > data.len() {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes[..size].copy_from_slice(&data[offset..offset + size]);
    Some(usize::from_le_bytes(bytes))
}

/// Extract a length-prefixed UTF-8 string from a buffer at the given offset.
/// Format: usize_le length + utf8 bytes
fn extract_string(data: &[u8], offset: usize) -> Option<String> {
    let len = read_usize_le(data, offset)?;
    let size = std::mem::size_of::<usize>();
    let start = offset + size;
    if len > 10_000_000 || start + len > data.len() {
        return None;
    }
    String::from_utf8(data[start..start + len].to_vec()).ok()
}

/// The wrapped dispatch function that intercepts bridge RPC calls
unsafe extern "C" fn wrapped_dispatch(env: *mut u8, request: Buffer) -> Buffer {
    let request_data = request.as_slice().to_vec();
    let (is_from_str, is_to_string, arg_offset) = classify_request(&request_data);

    // Forward to original dispatch
    let response = ORIGINAL_DISPATCH.with(|orig| {
        let orig = orig.borrow();
        let orig = orig.as_ref().expect("original dispatch not set");
        unsafe { (orig.call)(orig.env, request) }
    });

    // POST-FORWARD: Check the method tags and capture strings
    {
        if is_from_str {
            // ts_from_str request: [tag(s)] [usize_le len] [UTF-8 string]
            if let Some(s) = extract_string(&request_data, arg_offset) {
                CAPTURED.with(|c| {
                    c.borrow_mut().from_str_calls.push(s);
                });
            }
        } else if is_to_string {
            // ts_to_string: response = [0x00 Ok] [usize_le len] [UTF-8 string]
            let response_data = response.as_slice();
            if response_data.first() == Some(&0x00) {
                if let Some(s) = extract_string(response_data, 1) {
                    CAPTURED.with(|c| {
                        c.borrow_mut().to_string_results.push(s);
                    });
                }
            }
        }
    }

    let _ = env;
    response
}

/// Install the dispatch interceptor, replacing the closure in the BridgeConfig.
///
/// Returns a new Closure that wraps the original dispatch.
///
/// # Safety
/// The original closure must be valid for the lifetime of the returned closure.
pub unsafe fn install_dispatch_interceptor<'a>(
    original: &Closure<'a, Buffer, Buffer>,
) -> Closure<'a, Buffer, Buffer> {
    // Store the original dispatch in thread-local
    ORIGINAL_DISPATCH.with(|orig| {
        *orig.borrow_mut() = Some(OriginalDispatch {
            call: original.call,
            env: original.env,
        });
    });

    // Clear any previously captured strings
    CAPTURED.with(|c| {
        *c.borrow_mut() = CapturedStrings::default();
    });

    Closure {
        call: wrapped_dispatch,
        env: std::ptr::null_mut(),
        _marker: std::marker::PhantomData,
    }
}

/// Call to_string on a TokenStream handle through the original dispatch.
///
/// Must be called while the dispatch interceptor is installed (between
/// install_dispatch_interceptor and cleanup_dispatch).
///
/// # Safety
/// The dispatch closure's env pointer must still be valid.
pub unsafe fn call_to_string_on_handle(handle: u32) -> Option<String> {
    let buf = Buffer::from_vec(ts_to_string_request(handle));

    let response = ORIGINAL_DISPATCH.with(|orig| {
        let orig = orig.borrow();
        let orig = orig.as_ref()?;
        Some(unsafe { (orig.call)(orig.env, buf) })
    })?;

    let resp_data = response.as_slice();
    // Response format: [0x00 Ok marker] [usize_le len] [UTF-8 string]
    if resp_data.first() == Some(&0x00) {
        extract_string(resp_data, 1)
    } else {
        None
    }
}

/// Take the captured strings from the current invocation, resetting the state.
pub fn take_captured() -> CapturedStrings {
    CAPTURED.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Clean up the original dispatch reference
pub fn cleanup_dispatch() {
    ORIGINAL_DISPATCH.with(|orig| {
        *orig.borrow_mut() = None;
    });
}
