//! C ABI. Every call returns an opaque `AvResult*` that the caller frees with `av_result_free`;
//! no call panics across the boundary and no error is reported any other way than through the
//! result. Strings are NUL-terminated UTF-8 owned by the result they were read from.

use crate::caps;
use crate::convert::{ConvertOptions, convert};
use crate::error::AvError;
use crate::probe::probe;
use crate::service::{
    ConvertSpec, Sandbox, TranscodeSpec, convert_file, probe_file, transcode_file,
};
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// The outcome of one call: a status, an error string, a JSON document, an output buffer and an
/// optional list of named output files.
pub struct AvResult {
    status: i32,
    error: Option<CString>,
    json: Option<CString>,
    data: Vec<u8>,
    paths: Vec<CString>,
    files: Vec<Vec<u8>>,
}

impl AvResult {
    fn ok() -> AvResult {
        AvResult {
            status: 0,
            error: None,
            json: None,
            data: Vec::new(),
            paths: Vec::new(),
            files: Vec::new(),
        }
    }

    fn failure(code: i32, message: &str) -> AvResult {
        let clean: String = message.chars().filter(|c| *c != '\0').collect();
        AvResult {
            status: code,
            error: CString::new(clean).ok(),
            ..AvResult::ok()
        }
    }

    fn from_error(error: &AvError) -> AvResult {
        AvResult::failure(error.code(), &error.to_string())
    }

    fn with_json(mut self, json: &impl serde::Serialize) -> AvResult {
        match serde_json::to_string(json)
            .ok()
            .and_then(|text| CString::new(text).ok())
        {
            Some(text) => {
                self.json = Some(text);
                self
            }
            None => AvResult::failure(2, "could not serialise the result"),
        }
    }
}

fn into_raw(result: AvResult) -> *mut AvResult {
    Box::into_raw(Box::new(result))
}

fn guarded(call: impl FnOnce() -> AvResult) -> *mut AvResult {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(result) => into_raw(result),
        Err(_) => into_raw(AvResult::failure(
            99,
            "internal error: the operation panicked and was contained",
        )),
    }
}

unsafe fn text<'a>(pointer: *const c_char, what: &str) -> Result<Option<&'a str>, AvError> {
    if pointer.is_null() {
        return Ok(None);
    }
    // SAFETY: the caller promises a NUL-terminated string that outlives the call.
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map(Some)
        .map_err(|_| AvError::invalid(format!("{what} is not valid UTF-8")))
}

unsafe fn bytes<'a>(data: *const u8, length: usize) -> Result<&'a [u8], AvError> {
    if data.is_null() || length == 0 {
        return Err(AvError::invalid("input buffer is empty or null"));
    }
    // SAFETY: the caller promises `length` readable bytes at `data` for the duration of the call.
    Ok(unsafe { std::slice::from_raw_parts(data, length) })
}

fn parse_spec<T: serde::de::DeserializeOwned + Default>(
    json: Option<&str>,
    what: &str,
) -> Result<T, AvError> {
    match json {
        None | Some("") => Ok(T::default()),
        Some(source) => {
            serde_json::from_str(source).map_err(|e| AvError::invalid(format!("{what}: {e}")))
        }
    }
}

/// Library version string; static, never freed.
#[unsafe(no_mangle)]
pub extern "C" fn av_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// JSON capability matrix (codecs and containers).
#[unsafe(no_mangle)]
pub extern "C" fn av_capabilities() -> *mut AvResult {
    guarded(|| {
        AvResult::ok().with_json(
            &serde_json::json!({ "codecs": caps::codec_caps(), "formats": caps::format_caps() }),
        )
    })
}

/// Probe an in-memory file. `hint` is an optional file name. The result carries JSON.
///
/// # Safety
/// `data` must point to `length` readable bytes; `hint` must be NULL or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_probe(
    data: *const u8,
    length: usize,
    hint: *const c_char,
) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let input = unsafe { bytes(data, length)? };
            let hint = unsafe { text(hint, "hint")? };
            Ok(AvResult::ok().with_json(&probe(input, hint)?))
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// Probe a file on disk.
///
/// # Safety
/// `path` must be NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_probe_path(path: *const c_char) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let path =
                unsafe { text(path, "path")? }.ok_or_else(|| AvError::invalid("path is null"))?;
            Ok(AvResult::ok().with_json(&probe_file(&Sandbox::open(), path)?))
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// Transcode an in-memory file. `output_format` is a container name (`mp4`, `mkv`, ...);
/// `options_json` is a `TranscodeSpec` object (or NULL). The output bytes are in the result.
///
/// # Safety
/// Pointers must be valid as for `av_probe`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_transcode(
    data: *const u8,
    length: usize,
    input_hint: *const c_char,
    output_format: *const c_char,
    options_json: *const c_char,
) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let input = unsafe { bytes(data, length)? };
            let hint = unsafe { text(input_hint, "input_hint")? };
            let format = unsafe { text(output_format, "output_format")? };
            let mut spec: TranscodeSpec = parse_spec(
                unsafe { text(options_json, "options_json")? },
                "options_json",
            )?;
            if let Some(format) = format.filter(|f| !f.is_empty()) {
                spec.format = Some(format.to_owned());
            }
            let (format, options) = spec.resolve(None, hint)?;
            let transcoded = crate::transcode::transcode(input, format, &options)?;
            let mut result = AvResult::ok().with_json(&transcoded.report);
            result.data = transcoded.bytes;
            Ok(result)
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// Transcode `input` to `output` on disk (format from the output extension unless `options_json`
/// names one). Set `overwrite` non-zero to replace an existing output.
///
/// # Safety
/// Strings must be NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_transcode_path(
    input: *const c_char,
    output: *const c_char,
    options_json: *const c_char,
    overwrite: i32,
) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let input = unsafe { text(input, "input")? }
                .ok_or_else(|| AvError::invalid("input is null"))?;
            let output = unsafe { text(output, "output")? }
                .ok_or_else(|| AvError::invalid("output is null"))?;
            let spec: TranscodeSpec = parse_spec(
                unsafe { text(options_json, "options_json")? },
                "options_json",
            )?;
            Ok(AvResult::ok().with_json(&transcode_file(
                &Sandbox::open(),
                input,
                output,
                &spec,
                overwrite != 0,
            )?))
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// Convert an in-memory video to SVG, Markdown, HTML, JSX or GPUI documents. `options_json` is a
/// `ConvertSpec` (or NULL). The documents are the result's files.
///
/// # Safety
/// Pointers must be valid as for `av_probe`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_convert(
    data: *const u8,
    length: usize,
    hint: *const c_char,
    options_json: *const c_char,
) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let input = unsafe { bytes(data, length)? };
            let hint = unsafe { text(hint, "hint")? };
            let spec: ConvertSpec = parse_spec(
                unsafe { text(options_json, "options_json")? },
                "options_json",
            )?;
            let options: ConvertOptions = spec.resolve(hint.unwrap_or("video"))?;
            let converted = convert(input, hint, &options)?;
            let mut result = AvResult::ok().with_json(&serde_json::json!({ "frames": converted.frames, "decoded": converted.decoded, "files": converted.files.iter().map(|f| &f.path).collect::<Vec<_>>() }));
            for file in converted.files {
                result.paths.push(
                    CString::new(file.path)
                        .map_err(|_| AvError::invalid("file name contains NUL"))?,
                );
                result.files.push(file.bytes);
            }
            Ok(result)
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// Convert `input` and write the documents under the directory `output_dir`.
///
/// # Safety
/// Strings must be NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_convert_path(
    input: *const c_char,
    output_dir: *const c_char,
    options_json: *const c_char,
    overwrite: i32,
) -> *mut AvResult {
    guarded(|| {
        let outcome = (|| -> Result<AvResult, AvError> {
            let input = unsafe { text(input, "input")? }
                .ok_or_else(|| AvError::invalid("input is null"))?;
            let directory = unsafe { text(output_dir, "output_dir")? }
                .ok_or_else(|| AvError::invalid("output_dir is null"))?;
            let spec: ConvertSpec = parse_spec(
                unsafe { text(options_json, "options_json")? },
                "options_json",
            )?;
            Ok(AvResult::ok().with_json(&convert_file(
                &Sandbox::open(),
                input,
                directory,
                &spec,
                overwrite != 0,
            )?))
        })();
        outcome.unwrap_or_else(|e| AvResult::from_error(&e))
    })
}

/// 0 on success, otherwise 1 unsupported, 2 malformed input, 3 invalid argument, 4 I/O, 5 denied, 99 internal.
///
/// # Safety
/// `result` must come from this library and not have been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_status(result: *const AvResult) -> i32 {
    unsafe { result.as_ref() }.map_or(3, |r| r.status)
}

/// The error message, or NULL when the call succeeded.
///
/// # Safety
/// As for `av_result_status`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_error(result: *const AvResult) -> *const c_char {
    unsafe { result.as_ref() }
        .and_then(|r| r.error.as_ref())
        .map_or(std::ptr::null(), |e| e.as_ptr())
}

/// The JSON document (probe info, transcode report, convert summary, capabilities), or NULL.
///
/// # Safety
/// As for `av_result_status`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_json(result: *const AvResult) -> *const c_char {
    unsafe { result.as_ref() }
        .and_then(|r| r.json.as_ref())
        .map_or(std::ptr::null(), |j| j.as_ptr())
}

/// The output bytes of `av_transcode`; the length is stored in `length`.
///
/// # Safety
/// As for `av_result_status`; `length` must be writable or NULL.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_data(result: *const AvResult, length: *mut usize) -> *const u8 {
    match unsafe { result.as_ref() } {
        Some(r) => {
            if !length.is_null() {
                unsafe { *length = r.data.len() };
            }
            r.data.as_ptr()
        }
        None => {
            if !length.is_null() {
                unsafe { *length = 0 };
            }
            std::ptr::null()
        }
    }
}

/// Number of named output files (`av_convert`).
///
/// # Safety
/// As for `av_result_status`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_file_count(result: *const AvResult) -> usize {
    unsafe { result.as_ref() }.map_or(0, |r| r.files.len())
}

/// Relative path of output file `index`, or NULL when out of range.
///
/// # Safety
/// As for `av_result_status`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_file_path(
    result: *const AvResult,
    index: usize,
) -> *const c_char {
    unsafe { result.as_ref() }
        .and_then(|r| r.paths.get(index))
        .map_or(std::ptr::null(), |p| p.as_ptr())
}

/// Bytes of output file `index`; the length is stored in `length`.
///
/// # Safety
/// As for `av_result_data`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_file_data(
    result: *const AvResult,
    index: usize,
    length: *mut usize,
) -> *const u8 {
    match unsafe { result.as_ref() }.and_then(|r| r.files.get(index)) {
        Some(file) => {
            if !length.is_null() {
                unsafe { *length = file.len() };
            }
            file.as_ptr()
        }
        None => {
            if !length.is_null() {
                unsafe { *length = 0 };
            }
            std::ptr::null()
        }
    }
}

/// Release a result and everything read from it. NULL is accepted.
///
/// # Safety
/// `result` must come from this library and be freed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn av_result_free(result: *mut AvResult) {
    if !result.is_null() {
        drop(unsafe { Box::from_raw(result) });
    }
}
