//! C ABI of the router, in the shared Aphrody C ABI (`aphrody_*` symbols, JSON strings freed with `aphrody_free`).
//! See include/ai_router.h.

use crate::{Config, Router};
use ay_open_c_abi::{c_str, into_c_string, json_err};
use std::ffi::c_char;

pub use ay_open_c_abi::{aphrody_abi_version, aphrody_free};

/// Opens a router from a JSON configuration (NULL or "" for defaults). Returns NULL on error; the error text is
/// then available from `aphrody_router_last_error`.
///
/// # Safety
/// `config_json` is NULL or a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aphrody_router_open(config_json: *const c_char) -> *mut Router {
    let text = if config_json.is_null() { "" } else { unsafe { c_str(config_json) } };
    let config: Config = if text.trim().is_empty() {
        Config::default()
    } else {
        match serde_json::from_str(text) {
            Ok(c) => c,
            Err(e) => return fail(format!("invalid configuration: {e}")),
        }
    };
    match Router::open(&config) {
        Ok(r) => Box::into_raw(Box::new(r)),
        Err(e) => fail(e),
    }
}

thread_local! {
    static LAST_ERROR: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn fail(e: String) -> *mut Router {
    LAST_ERROR.with(|l| *l.borrow_mut() = e);
    std::ptr::null_mut()
}

/// The last error of `aphrody_router_open` on this thread, as a JSON string to free with `aphrody_free`.
#[unsafe(no_mangle)]
pub extern "C" fn aphrody_router_last_error() -> *mut c_char {
    LAST_ERROR.with(|l| json_err(l.borrow().clone()))
}

/// Runs an OpenAI chat-completions request (JSON) and returns the answer document (JSON, free with `aphrody_free`).
///
/// # Safety
/// `router` comes from `aphrody_router_open`; `request_json` is a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aphrody_router_chat(router: *const Router, request_json: *const c_char) -> *mut c_char {
    if router.is_null() || request_json.is_null() {
        return json_err("null argument");
    }
    let router = unsafe { &*router };
    match serde_json::from_str(unsafe { c_str(request_json) }) {
        Ok(req) => into_c_string(router.chat(&req).to_string()),
        Err(e) => json_err(format!("invalid request: {e}")),
    }
}

/// Usable offers for a policy or model (`auto`, `auto:local`, `auto:free`, `auto:cheap`, `<provider>/<model>`), as a
/// JSON array (free with `aphrody_free`).
///
/// # Safety
/// `router` comes from `aphrody_router_open`; `model` is NULL or a NUL-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aphrody_router_models(router: *const Router, model: *const c_char) -> *mut c_char {
    if router.is_null() {
        return json_err("null router");
    }
    let router = unsafe { &*router };
    let model = if model.is_null() { "auto" } else { unsafe { c_str(model) } };
    into_c_string(serde_json::to_string(&router.models(model)).unwrap_or_else(|_| "[]".to_owned()))
}

/// Releases a router.
///
/// # Safety
/// `router` comes from `aphrody_router_open` and is not used afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aphrody_router_close(router: *mut Router) {
    if !router.is_null() {
        drop(unsafe { Box::from_raw(router) });
    }
}

/// Callback receiving one chat.completion.chunk (JSON, valid during the call only) and the caller's pointer.
pub type ChunkCallback = extern "C" fn(chunk_json: *const c_char, user: *mut std::ffi::c_void);

/// Streams a chat: `on_chunk` is called on this thread for each chunk; returns the router summary or an error
/// document (JSON, free with `aphrody_free`).
///
/// # Safety
/// `router` comes from `aphrody_router_open`; `request_json` is a NUL-terminated UTF-8 string; `on_chunk` is a valid
/// function; `user` is passed back unchanged.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aphrody_router_chat_stream(router: *const Router, request_json: *const c_char, on_chunk: ChunkCallback, user: *mut std::ffi::c_void) -> *mut c_char {
    if router.is_null() || request_json.is_null() {
        return json_err("null argument");
    }
    let router = unsafe { &*router };
    let req: serde_json::Value = match serde_json::from_str(unsafe { c_str(request_json) }) {
        Ok(r) => r,
        Err(e) => return json_err(format!("invalid request: {e}")),
    };
    let summary = router.chat_stream(&req, &mut |chunk| {
        if let Ok(s) = std::ffi::CString::new(chunk.to_string()) {
            on_chunk(s.as_ptr(), user);
        }
    });
    into_c_string(summary.to_string())
}
