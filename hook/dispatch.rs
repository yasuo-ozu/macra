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

/// Interception state for one trampoline that is currently on this thread's stack.
struct Frame {
    /// The dispatch closure rustc handed that trampoline.
    dispatch: OriginalDispatch,
    /// Strings intercepted on behalf of that trampoline's expansion.
    captured: CapturedStrings,
}

thread_local! {
    /// One [`Frame`] per trampoline currently executing on this thread, innermost last.
    ///
    /// This is a stack, not a single slot, because expansions nest. The bridge has a
    /// `TokenStream::expand_expr` method (nightly `proc_macro_expand`, present on
    /// every compiler in the 1.86–1.97 window the hook arms for): a proc macro calls
    /// it, the server performs a full expansion of the argument *on the same thread*,
    /// and if that argument is itself a proc-macro invocation the server calls that
    /// macro's `run` — another of our trampolines — while the outer trampoline is
    /// still on the stack. With one slot the inner install overwrote the outer's
    /// dispatch, the inner cleanup cleared it so the outer's next request hit
    /// "original dispatch not set", and the inner captures were folded into the
    /// outer record. Each of those is a panic or a wrong answer inside rustc, and
    /// the trampolines are `extern "C"`, so a panic there is a process abort.
    static FRAMES: RefCell<Vec<Frame>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy)]
struct OriginalDispatch {
    call: unsafe extern "C" fn(*mut u8, Buffer) -> Buffer,
    env: *mut u8,
}

/// Bridge API method group tag for TokenStream methods.
const METHOD_TOKEN_STREAM: u8 = 0x01;

/// TokenStream method indices (from `with_api!` enum order):
/// drop=0, clone=1, is_empty=2, expand_expr=3, from_str=4, to_string=5, ...
const TS_FROM_STR: u8 = 0x04;
const TS_TO_STRING: u8 = 0x05;

/// The RPC tag encoding the running compiler uses.
///
/// `with_api!` used to nest methods under a type, so a request began with a group
/// byte and a method byte. From 1.95 it is one flat `#[repr(u8)] enum ApiTags`
/// encoded as a single byte — and the indices within it shift as methods are added
/// or removed, so cargo-macra passes the numbering rather than the hook assuming it.
/// 1.95 through 1.99 put `ts_to_string` at 10; 1.100 removed a method, moving it
/// to 9. Sending the wrong one invokes a different bridge method and panics rustc.
fn rpc_tags() -> Option<cargo_macra::RpcTags> {
    crate::types::selected_abi().map(|abi| abi.rpc)
}

/// Split a request into `(from_str, to_string)` predicates for the ABI in use.
///
/// The nested encoding prefixes the tag with a group byte, so the argument payload
/// also starts one byte later than in the flat form.
fn classify_request(data: &[u8]) -> (bool, bool, usize) {
    match rpc_tags() {
        Some(cargo_macra::RpcTags::Flat {
            from_str,
            to_string,
        }) => {
            let tag = data.first().copied();
            (tag == Some(from_str), tag == Some(to_string), 1)
        }
        _ => {
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
}

/// Encode a `ts_to_string` request for the ABI in use.
fn ts_to_string_request(handle: u32) -> Vec<u8> {
    let mut request = match rpc_tags() {
        Some(cargo_macra::RpcTags::Flat { to_string, .. }) => vec![to_string],
        _ => vec![METHOD_TOKEN_STREAM, TS_TO_STRING],
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

/// The innermost frame's dispatch and its depth, or `None` outside any trampoline.
///
/// Callers forward through the returned copy *after* this function has returned, so
/// no `RefCell` borrow is alive while the server runs. That matters: a forwarded
/// `expand_expr` can re-enter [`install_dispatch_interceptor`], whose `borrow_mut`
/// would otherwise collide with a still-held `borrow` here and panic with
/// `BorrowMutError` — inside an `extern "C"` frame, hence an abort of rustc.
fn current_dispatch() -> Option<(OriginalDispatch, usize)> {
    FRAMES.with(|frames| {
        let frames = frames.borrow();
        let top = frames.last()?;
        Some((top.dispatch, frames.len() - 1))
    })
}

/// Record an intercepted string against the frame at `depth`.
///
/// The frame is addressed by depth rather than "whatever is on top" so that a
/// nested expansion which ran during the forward — and pushed and popped its own
/// frame — cannot make the outer request's capture land in the wrong record. If
/// that frame is gone (an unbalanced cleanup), the string is dropped rather than
/// attributed to a stranger.
fn record(depth: usize, with: impl FnOnce(&mut CapturedStrings)) {
    FRAMES.with(|frames| {
        if let Some(frame) = frames.borrow_mut().get_mut(depth) {
            with(&mut frame.captured);
        }
    });
}

/// Release a buffer the bridge handed us that we are not passing on.
fn drop_buffer(buffer: Buffer) {
    if let Some(drop_fn) = buffer.drop {
        drop_fn(buffer);
    }
}

/// The wrapped dispatch function that intercepts bridge RPC calls
unsafe extern "C" fn wrapped_dispatch(env: *mut u8, request: Buffer) -> Buffer {
    let request_data = request.as_slice().to_vec();
    let (is_from_str, is_to_string, arg_offset) = classify_request(&request_data);

    // With no frame there is nothing to forward to. This cannot happen while the
    // trampolines keep install/cleanup balanced, but the alternative — `expect` —
    // is a panic in an `extern "C"` fn, i.e. an abort of the compiler. An empty
    // reply instead fails the client's decode inside its own `catch_unwind`, which
    // rustc reports as that proc macro panicking and carries on.
    let Some((dispatch, depth)) = current_dispatch() else {
        eprintln!("[macra-hook] bridge request with no dispatch interceptor installed");
        drop_buffer(request);
        return Buffer::from_vec(Vec::new());
    };

    // Forward to original dispatch. No thread-local borrow is held here: the server
    // may run a nested expansion before returning (see `FRAMES`).
    let response = unsafe { (dispatch.call)(dispatch.env, request) };

    // POST-FORWARD: Check the method tags and capture strings
    {
        if is_from_str {
            // ts_from_str request: [tag(s)] [usize_le len] [UTF-8 string]
            if let Some(s) = extract_string(&request_data, arg_offset) {
                record(depth, |c| c.from_str_calls.push(s));
            }
        } else if is_to_string {
            // ts_to_string: response = [0x00 Ok] [usize_le len] [UTF-8 string]
            let response_data = response.as_slice();
            if response_data.first() == Some(&0x00) {
                if let Some(s) = extract_string(response_data, 1) {
                    record(depth, |c| c.to_string_results.push(s));
                }
            }
        }
    }

    let _ = env;
    response
}

/// Install the dispatch interceptor, replacing the closure in the BridgeConfig.
///
/// Pushes a frame for the calling trampoline; [`cleanup_dispatch`] pops it. Every
/// install must be paired with exactly one cleanup on the same thread, in LIFO
/// order — which is what nested trampolines naturally do.
///
/// Returns a new Closure that wraps the original dispatch.
///
/// # Safety
/// The original closure must be valid for the lifetime of the returned closure.
pub unsafe fn install_dispatch_interceptor<'a>(
    original: &Closure<'a, Buffer, Buffer>,
) -> Closure<'a, Buffer, Buffer> {
    FRAMES.with(|frames| {
        frames.borrow_mut().push(Frame {
            dispatch: OriginalDispatch {
                call: original.call,
                env: original.env,
            },
            // A fresh capture set per frame: the previous frame's strings stay with
            // the previous frame instead of being cleared or inherited.
            captured: CapturedStrings::default(),
        });
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

    let Some((dispatch, _)) = current_dispatch() else {
        drop_buffer(buf);
        return None;
    };
    let response = unsafe { (dispatch.call)(dispatch.env, buf) };

    let resp_data = response.as_slice();
    // Response format: [0x00 Ok marker] [usize_le len] [UTF-8 string]
    let parsed = if resp_data.first() == Some(&0x00) {
        extract_string(resp_data, 1)
    } else {
        None
    };

    // The server hands back a buffer it grew for us; nothing else frees it, and the
    // mirror type has no Drop. Without this every intercepted expansion leaked a
    // buffer holding the whole token-stream text, inside rustc.
    drop_buffer(response);

    parsed
}

/// Take the captured strings from the current (innermost) invocation, resetting them.
pub fn take_captured() -> CapturedStrings {
    FRAMES.with(|frames| {
        frames
            .borrow_mut()
            .last_mut()
            .map(|frame| std::mem::take(&mut frame.captured))
            .unwrap_or_default()
    })
}

/// Pop the innermost trampoline's frame, restoring the enclosing one (if any).
///
/// An unbalanced call on an empty stack is a no-op: `Vec::pop` returns `None`
/// rather than underflowing, so a cleanup without a matching install can neither
/// panic nor disturb another frame.
pub fn cleanup_dispatch() {
    FRAMES.with(|frames| {
        frames.borrow_mut().pop();
    });
}

#[cfg(test)]
mod tests {
    //! These exercise the frame stack with fake dispatch closures. They rely on
    //! `MACRA_ABI` being unset in the test process so that `classify_request` uses
    //! the nested (group byte + method byte) tag encoding.

    use super::*;

    /// Encode a `ts_from_str` request carrying `s`, in the nested tag encoding.
    fn from_str_request(s: &str) -> Buffer {
        let mut v = vec![METHOD_TOKEN_STREAM, TS_FROM_STR];
        v.extend_from_slice(&s.len().to_le_bytes());
        v.extend_from_slice(s.as_bytes());
        Buffer::from_vec(v)
    }

    /// Encode an `Ok(String)` reply.
    fn ok_string_reply(s: &str) -> Buffer {
        let mut v = vec![0x00];
        v.extend_from_slice(&s.len().to_le_bytes());
        v.extend_from_slice(s.as_bytes());
        Buffer::from_vec(v)
    }

    fn reply_string(b: Buffer) -> Option<String> {
        let s = if b.as_slice().first() == Some(&0x00) {
            extract_string(b.as_slice(), 1)
        } else {
            None
        };
        drop_buffer(b);
        s
    }

    /// A stand-in server: answers every request with `Ok("dispatch-<env>")`, so a
    /// caller can tell which closure its request reached.
    unsafe extern "C" fn tagged_server(env: *mut u8, request: Buffer) -> Buffer {
        drop_buffer(request);
        ok_string_reply(&format!("dispatch-{}", env as usize))
    }

    /// A stand-in for a server handling `expand_expr`: while "expanding" it runs a
    /// whole nested trampoline lifecycle — install, a request through the wrapped
    /// dispatch, take, cleanup — exactly as a nested proc macro's `run` would.
    unsafe extern "C" fn reentrant_server(env: *mut u8, request: Buffer) -> Buffer {
        drop_buffer(request);
        let inner_original = closure(tagged_server, 2);
        let wrapped = unsafe { install_dispatch_interceptor(&inner_original) };
        let reply = unsafe { (wrapped.call)(wrapped.env, from_str_request("inner")) };
        assert_eq!(reply_string(reply).as_deref(), Some("dispatch-2"));
        let inner = take_captured();
        assert_eq!(inner.from_str_calls, ["inner"]);
        cleanup_dispatch();
        ok_string_reply(&format!("expanded-{}", env as usize))
    }

    fn closure(
        call: unsafe extern "C" fn(*mut u8, Buffer) -> Buffer,
        env: usize,
    ) -> Closure<'static, Buffer, Buffer> {
        Closure {
            call,
            env: env as *mut u8,
            _marker: std::marker::PhantomData,
        }
    }

    #[test]
    fn nested_install_forwards_to_innermost_then_restores_outer() {
        let outer = closure(tagged_server, 1);
        let inner = closure(tagged_server, 2);

        let _ = unsafe { install_dispatch_interceptor(&outer) };
        assert_eq!(
            unsafe { call_to_string_on_handle(7) }.as_deref(),
            Some("dispatch-1")
        );

        let _ = unsafe { install_dispatch_interceptor(&inner) };
        assert_eq!(
            unsafe { call_to_string_on_handle(7) }.as_deref(),
            Some("dispatch-2")
        );

        cleanup_dispatch();
        // The outer dispatch is back, not `None`.
        assert_eq!(
            unsafe { call_to_string_on_handle(7) }.as_deref(),
            Some("dispatch-1")
        );
        cleanup_dispatch();
        assert!(unsafe { call_to_string_on_handle(7) }.is_none());
    }

    #[test]
    fn reentrant_forward_keeps_inner_captures_out_of_outer_record() {
        let outer = closure(reentrant_server, 1);
        let wrapped = unsafe { install_dispatch_interceptor(&outer) };

        // Before the fix this forward held a `Ref` on the slot while the server
        // re-entered `install_dispatch_interceptor`, panicking on `borrow_mut`.
        let reply = unsafe { (wrapped.call)(wrapped.env, from_str_request("outer")) };
        assert_eq!(reply_string(reply).as_deref(), Some("expanded-1"));

        // The outer frame survived the nested lifecycle and holds only its own string.
        let captured = take_captured();
        assert_eq!(captured.from_str_calls, ["outer"]);
        assert!(captured.to_string_results.is_empty());

        // And the outer dispatch is still the one installed, not cleared by the
        // inner cleanup.
        assert_eq!(
            unsafe { call_to_string_on_handle(7) }.as_deref(),
            Some("expanded-1")
        );
        cleanup_dispatch();
    }

    #[test]
    fn captures_are_per_frame() {
        let outer = closure(tagged_server, 1);
        let wrapped = unsafe { install_dispatch_interceptor(&outer) };
        drop_buffer(unsafe { (wrapped.call)(wrapped.env, from_str_request("a")) });

        let inner = closure(tagged_server, 2);
        let wrapped_inner = unsafe { install_dispatch_interceptor(&inner) };
        drop_buffer(unsafe { (wrapped_inner.call)(wrapped_inner.env, from_str_request("b")) });

        assert_eq!(take_captured().from_str_calls, ["b"]);
        cleanup_dispatch();
        assert_eq!(take_captured().from_str_calls, ["a"]);
        cleanup_dispatch();
    }

    #[test]
    fn unbalanced_cleanup_is_harmless() {
        cleanup_dispatch();
        cleanup_dispatch();
        assert!(take_captured().from_str_calls.is_empty());
        assert!(unsafe { call_to_string_on_handle(1) }.is_none());

        // The stack still works afterwards.
        let outer = closure(tagged_server, 3);
        let _ = unsafe { install_dispatch_interceptor(&outer) };
        assert_eq!(
            unsafe { call_to_string_on_handle(1) }.as_deref(),
            Some("dispatch-3")
        );
        cleanup_dispatch();
    }

    #[test]
    fn request_without_frame_gets_empty_reply_not_panic() {
        let reply = unsafe { wrapped_dispatch(std::ptr::null_mut(), from_str_request("x")) };
        assert!(reply.as_slice().is_empty());
        drop_buffer(reply);
    }
}
