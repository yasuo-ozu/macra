//! ABI-compatible mirrors of rustc's proc_macro bridge types.
//!
//! These match the nightly-2025-04-08 layout. All types are `#[repr(C)]`.

use std::marker::PhantomData;
use std::sync::atomic::AtomicU32;

/// Mirror of `proc_macro::bridge::buffer::Buffer`
#[repr(C)]
pub struct Buffer {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
    pub reserve: Option<extern "C" fn(Buffer, usize) -> Buffer>,
    pub drop: Option<extern "C" fn(Buffer)>,
}

impl Buffer {
    pub fn as_slice(&self) -> &[u8] {
        if self.data.is_null() || self.len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(self.data, self.len) }
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        if self.data.is_null() || self.len == 0 {
            &mut []
        } else {
            unsafe { std::slice::from_raw_parts_mut(self.data, self.len) }
        }
    }

    /// Create a Buffer from a byte slice with proper reserve/drop functions.
    /// The bridge's Buffer::Drop unconditionally calls the drop function pointer,
    /// so we must always provide valid functions.
    pub fn from_vec(data: Vec<u8>) -> Self {
        let mut v = std::mem::ManuallyDrop::new(data);
        Buffer {
            data: v.as_mut_ptr(),
            len: v.len(),
            capacity: v.capacity(),
            reserve: Some(buffer_reserve),
            drop: Some(buffer_drop),
        }
    }
}

/// Drop function for Vec-backed Buffers. Reconstructs and drops the Vec.
extern "C" fn buffer_drop(b: Buffer) {
    if !b.data.is_null() && b.capacity > 0 {
        unsafe {
            drop(Vec::from_raw_parts(b.data, b.len, b.capacity));
        }
    }
}

/// Reserve function for Vec-backed Buffers.
extern "C" fn buffer_reserve(b: Buffer, additional: usize) -> Buffer {
    let mut v = if !b.data.is_null() && b.capacity > 0 {
        unsafe { Vec::from_raw_parts(b.data, b.len, b.capacity) }
    } else {
        Vec::new()
    };
    v.reserve(additional);
    Buffer::from_vec(v)
}

/// Mirror of `proc_macro::bridge::closure::Closure<'a, A, R>`
#[repr(C)]
pub struct Closure<'a, A, R> {
    pub call: unsafe extern "C" fn(*mut u8, A) -> R,
    pub env: *mut u8,
    pub _marker: PhantomData<&'a mut ()>,
}

/// Mirror of `proc_macro::bridge::HandleCounters`
#[repr(C)]
pub struct HandleCounters {
    pub free_functions: AtomicU32,
    pub token_stream: AtomicU32,
    pub source_file: AtomicU32,
    pub span: AtomicU32,
}

/// Mirror of `proc_macro::bridge::BridgeConfig<'a>`
#[repr(C)]
pub struct BridgeConfig<'a> {
    pub input: Buffer,
    pub dispatch: Closure<'a, Buffer, Buffer>,
    pub force_show_panics: bool,
    pub _marker: PhantomData<&'a ()>,
}

/// Mirror of `proc_macro::bridge::client::Client<I, O>`
///
/// The `run` function pointer has the signature:
/// `extern "C" fn(BridgeConfig<'_>) -> Buffer`
#[repr(C)]
pub struct Client<I, O> {
    pub handle_counters: &'static HandleCounters,
    pub run: extern "C" fn(BridgeConfig<'_>) -> Buffer,
    pub _marker: PhantomData<fn(I) -> O>,
}

impl<I, O> Clone for Client<I, O> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<I, O> Copy for Client<I, O> {}

/// Opaque types for Client generic parameters
#[repr(C)]
pub struct TokenStream {
    _private: [u8; 0],
}

/// Mirror of `proc_macro::bridge::client::ProcMacro`
///
/// Three variants matching the nightly layout.
#[repr(C)]
pub enum ProcMacro {
    CustomDerive {
        trait_name: &'static str,
        attributes: &'static [&'static str],
        client: Client<TokenStream, TokenStream>,
    },
    Attr {
        name: &'static str,
        client: Client<(TokenStream, TokenStream), TokenStream>,
    },
    Bang {
        name: &'static str,
        client: Client<TokenStream, TokenStream>,
    },
}

impl ProcMacro {
    /// Get the name of this proc macro
    pub fn name(&self) -> &'static str {
        match self {
            ProcMacro::CustomDerive { trait_name, .. } => trait_name,
            ProcMacro::Attr { name, .. } => name,
            ProcMacro::Bang { name, .. } => name,
        }
    }

    /// Get the kind string
    pub fn kind(&self) -> &'static str {
        match self {
            ProcMacro::CustomDerive { .. } => "CustomDerive",
            ProcMacro::Attr { .. } => "Attr",
            ProcMacro::Bang { .. } => "Bang",
        }
    }
}
