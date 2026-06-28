//! C ABI for `rustiniparser`.
//!
//! This module exposes a stable C-compatible interface so the library can be
//! linked from C and C++ via the generated `.a` (staticlib) and `.so`/`.dll`/
//! `.dylib` (cdylib) artifacts. The matching header is
//! `include/rustiniparser.h`.
//!
//! ## Memory & ownership rules
//!
//! - A document is an opaque `IniDocument*` created by [`ini_new`] or
//!   [`ini_parse`] and **must** be released exactly once with [`ini_free`].
//! - String arguments are borrowed, NUL-terminated C strings; they are not
//!   retained past the call.
//! - String getters copy into a caller-provided buffer and never return a
//!   pointer into Rust-owned memory.
//! - Section name `""` (empty string) addresses the global section.

use crate::{Document, Error};
use std::ffi::{c_char, c_double, c_int, c_longlong, CStr};
use std::ptr;

/// Opaque handle to an INI document. Allocated by the library, freed by
/// [`ini_free`].
pub struct IniDocument {
    inner: Document,
}

/// Status codes returned by the mutating functions. `0` is success.
pub const INI_OK: c_int = 0;
pub const INI_ERR_SYNTAX: c_int = 1;
pub const INI_ERR_TOO_LONG: c_int = 2;
pub const INI_ERR_FULL: c_int = 3;
pub const INI_ERR_NOT_FOUND: c_int = 4;
pub const INI_ERR_NULL: c_int = 5;
pub const INI_ERR_UTF8: c_int = 6;

fn err_code(e: Error) -> c_int {
    match e {
        Error::Syntax => INI_ERR_SYNTAX,
        Error::TooLong => INI_ERR_TOO_LONG,
        Error::Full => INI_ERR_FULL,
        Error::NotFound => INI_ERR_NOT_FOUND,
    }
}

/// Borrow a C string as `&str`. Returns `None` for NULL or invalid UTF-8.
///
/// # Safety
/// `p` must be NULL or a valid pointer to a NUL-terminated C string that stays
/// alive for the duration of the call.
unsafe fn as_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

// --------------------------------------------------------------------------
// Lifecycle
// --------------------------------------------------------------------------

/// Create a new, empty document. Never returns NULL except on allocation
/// failure. Free it with [`ini_free`].
#[no_mangle]
pub extern "C" fn ini_new() -> *mut IniDocument {
    Box::into_raw(Box::new(IniDocument {
        inner: Document::new(),
    }))
}

/// Parse `text` (a NUL-terminated C string) into a new document.
///
/// Returns NULL if `text` is NULL, not valid UTF-8, or contains a syntax
/// error. On success, free the result with [`ini_free`].
///
/// # Safety
/// `text` must be NULL or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ini_parse(text: *const c_char) -> *mut IniDocument {
    let Some(s) = as_str(text) else {
        return ptr::null_mut();
    };
    match Document::parse(s) {
        Ok(doc) => Box::into_raw(Box::new(IniDocument { inner: doc })),
        Err(_) => ptr::null_mut(),
    }
}

/// Merge additional INI `text` into an existing document. Returns a status
/// code.
///
/// # Safety
/// `doc` must be a live handle from this library (or NULL); `text` must be
/// NULL or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ini_load(doc: *mut IniDocument, text: *const c_char) -> c_int {
    let Some(doc) = doc.as_mut() else {
        return INI_ERR_NULL;
    };
    let Some(s) = as_str(text) else {
        return INI_ERR_UTF8;
    };
    match doc.inner.load(s) {
        Ok(()) => INI_OK,
        Err(e) => err_code(e),
    }
}

/// Release a document. Passing NULL is a no-op. Each handle must be freed at
/// most once.
///
/// # Safety
/// `doc` must be NULL or a handle previously returned by this library and not
/// yet freed.
#[no_mangle]
pub unsafe extern "C" fn ini_free(doc: *mut IniDocument) {
    if !doc.is_null() {
        drop(Box::from_raw(doc));
    }
}

/// Number of stored entries, or 0 if `doc` is NULL.
///
/// # Safety
/// `doc` must be NULL or a live handle.
#[no_mangle]
pub unsafe extern "C" fn ini_len(doc: *const IniDocument) -> usize {
    match doc.as_ref() {
        Some(d) => d.inner.len(),
        None => 0,
    }
}

// --------------------------------------------------------------------------
// Getters
// --------------------------------------------------------------------------

/// Whether `section`/`key` exists. Returns 1 (true) or 0 (false).
///
/// # Safety
/// `doc` must be NULL or a live handle; `section`/`key` must be valid C
/// strings.
#[no_mangle]
pub unsafe extern "C" fn ini_has(
    doc: *const IniDocument,
    section: *const c_char,
    key: *const c_char,
) -> c_int {
    let (Some(d), Some(s), Some(k)) = (doc.as_ref(), as_str(section), as_str(key)) else {
        return 0;
    };
    d.inner.has(s, k) as c_int
}

/// Copy the string value for `section`/`key` into `out` (capacity `out_len`,
/// including the NUL terminator). If the key is absent, `fallback` is used.
///
/// The result is always NUL-terminated when `out_len > 0`. Returns the length
/// (excluding the NUL) of the full value; if that is `>= out_len` the value
/// was truncated. Pass `out = NULL`/`out_len = 0` to query the needed length.
///
/// # Safety
/// `doc` must be NULL or a live handle; `section`/`key`/`fallback` must be
/// valid C strings; `out` must point to at least `out_len` writable bytes (or
/// be NULL when `out_len` is 0).
#[no_mangle]
pub unsafe extern "C" fn ini_get_string(
    doc: *const IniDocument,
    section: *const c_char,
    key: *const c_char,
    fallback: *const c_char,
    out: *mut c_char,
    out_len: usize,
) -> usize {
    let fb = as_str(fallback).unwrap_or("");
    let value = match (doc.as_ref(), as_str(section), as_str(key)) {
        (Some(d), Some(s), Some(k)) => d.inner.get_string(s, k, fb),
        _ => fb,
    };

    let bytes = value.as_bytes();
    if !out.is_null() && out_len > 0 {
        let copy = bytes.len().min(out_len - 1);
        ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, copy);
        *out.add(copy) = 0;
    }
    bytes.len()
}

/// Get the value as a 64-bit integer (decimal or `0x` hex), or `fallback`.
///
/// # Safety
/// `doc` must be NULL or a live handle; `section`/`key` must be valid C
/// strings.
#[no_mangle]
pub unsafe extern "C" fn ini_get_int(
    doc: *const IniDocument,
    section: *const c_char,
    key: *const c_char,
    fallback: c_longlong,
) -> c_longlong {
    match (doc.as_ref(), as_str(section), as_str(key)) {
        (Some(d), Some(s), Some(k)) => d.inner.get_int(s, k, fallback as i64) as c_longlong,
        _ => fallback,
    }
}

/// Get the value as a double, or `fallback`.
///
/// # Safety
/// See [`ini_get_int`].
#[no_mangle]
pub unsafe extern "C" fn ini_get_double(
    doc: *const IniDocument,
    section: *const c_char,
    key: *const c_char,
    fallback: c_double,
) -> c_double {
    match (doc.as_ref(), as_str(section), as_str(key)) {
        (Some(d), Some(s), Some(k)) => d.inner.get_double(s, k, fallback),
        _ => fallback,
    }
}

/// Get the value as a boolean (1/0), or `fallback`.
///
/// # Safety
/// See [`ini_get_int`].
#[no_mangle]
pub unsafe extern "C" fn ini_get_bool(
    doc: *const IniDocument,
    section: *const c_char,
    key: *const c_char,
    fallback: c_int,
) -> c_int {
    match (doc.as_ref(), as_str(section), as_str(key)) {
        (Some(d), Some(s), Some(k)) => d.inner.get_bool(s, k, fallback != 0) as c_int,
        _ => fallback,
    }
}

// --------------------------------------------------------------------------
// Setters
// --------------------------------------------------------------------------

/// Set `section`/`key` to a string value (adds or overwrites). Status code.
///
/// # Safety
/// `doc` must be a live handle; `section`/`key`/`value` must be valid C
/// strings.
#[no_mangle]
pub unsafe extern "C" fn ini_set(
    doc: *mut IniDocument,
    section: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    let Some(d) = doc.as_mut() else {
        return INI_ERR_NULL;
    };
    let (Some(s), Some(k), Some(v)) = (as_str(section), as_str(key), as_str(value)) else {
        return INI_ERR_UTF8;
    };
    match d.inner.set(s, k, v) {
        Ok(()) => INI_OK,
        Err(e) => err_code(e),
    }
}

/// Set `section`/`key` to an integer value. Status code.
///
/// # Safety
/// `doc` must be a live handle; `section`/`key` must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn ini_set_int(
    doc: *mut IniDocument,
    section: *const c_char,
    key: *const c_char,
    value: c_longlong,
) -> c_int {
    let Some(d) = doc.as_mut() else {
        return INI_ERR_NULL;
    };
    let (Some(s), Some(k)) = (as_str(section), as_str(key)) else {
        return INI_ERR_UTF8;
    };
    match d.inner.set_int(s, k, value as i64) {
        Ok(()) => INI_OK,
        Err(e) => err_code(e),
    }
}

/// Set `section`/`key` to a boolean value (`value != 0`). Status code.
///
/// # Safety
/// `doc` must be a live handle; `section`/`key` must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn ini_set_bool(
    doc: *mut IniDocument,
    section: *const c_char,
    key: *const c_char,
    value: c_int,
) -> c_int {
    let Some(d) = doc.as_mut() else {
        return INI_ERR_NULL;
    };
    let (Some(s), Some(k)) = (as_str(section), as_str(key)) else {
        return INI_ERR_UTF8;
    };
    match d.inner.set_bool(s, k, value != 0) {
        Ok(()) => INI_OK,
        Err(e) => err_code(e),
    }
}

/// Remove `section`/`key`. Returns `INI_OK` or `INI_ERR_NOT_FOUND`.
///
/// # Safety
/// `doc` must be a live handle; `section`/`key` must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn ini_remove(
    doc: *mut IniDocument,
    section: *const c_char,
    key: *const c_char,
) -> c_int {
    let Some(d) = doc.as_mut() else {
        return INI_ERR_NULL;
    };
    let (Some(s), Some(k)) = (as_str(section), as_str(key)) else {
        return INI_ERR_UTF8;
    };
    match d.inner.remove(s, k) {
        Ok(()) => INI_OK,
        Err(e) => err_code(e),
    }
}
