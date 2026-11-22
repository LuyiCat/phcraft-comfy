//! The Rust half of the system pickers (`ios/PCDocumentPicker.m`): the document picker and
//! the export sheet answer through the same pending-reply machinery — only the presenting
//! call differs. The document picker and its delegate are ObjC (implementing the protocol
//! from Rust needs objc2's unsafe `declare_class!`), and this module is the safe-looking
//! seam — the C callbacks run on the main thread (where egui frames also run, never
//! concurrently), copy their bytes across the ABI, and answer the app's pending
//! `FileDialogReply`. The only unsafe in this crate is the pointer reads those copies require.
//!
//! The real implementation is iOS-only (the pc_* symbols only link there); on other hosts —
//! clippy and tests of this bin — the entry points are no-ops, and a dropped reply counts as
//! Cancel (`crates/ui-egui/src/file_dialog.rs`).

#[cfg(target_os = "ios")]
use photocraft_ui_egui::FileDialogReply;

/// Wire the callbacks once, before any dialog can start (from `main`).
#[cfg(target_os = "ios")]
pub fn install() {
    real::install();
}

/// File ▸ Open and friends: present the system picker; the reply goes out from the
/// delegate's callbacks.
#[cfg(target_os = "ios")]
pub fn open(multiple: bool, reply: FileDialogReply) {
    real::open(multiple, reply);
}

/// File ▸ Save As on iPadOS: present the system export picker for the suggested name, and
/// answer with the real destination path the delegate reports. File ▸ Save does not use this;
/// it keeps writing the app's Documents path through the ordinary `Save` request.
#[cfg(target_os = "ios")]
pub fn save_location(suggested: &str, reply: FileDialogReply) {
    real::save_location(suggested, reply);
}

/// Present the export sheet for a file (the bridge for a copy-out command; a Save no longer
/// stages a tmp file, it writes Documents directly, see `main.rs`).
#[cfg(target_os = "ios")]
#[allow(dead_code)]
pub fn export(path: &std::path::Path) {
    real::export(path);
}

#[cfg(not(target_os = "ios"))]
pub use stubs::{install, open, save_location};

#[cfg(target_os = "ios")]
mod real {
    use std::ffi::{CString, c_char, c_int, c_uchar};
    use std::path::PathBuf;
    use std::sync::{Mutex, PoisonError};

    use photocraft_ui_egui::{FileDialogAnswer, FileDialogReply};

    unsafe extern "C" {
        // `safe`: our own C (PCDocumentPicker.m) — each dispatches async on the main queue and
        // keeps no reference to its arguments past the call.
        safe fn pc_set_callbacks(open: extern "C" fn(*const c_char, *const c_uchar, usize, c_int), cancel: extern "C" fn());
        safe fn pc_pick_open(allow_multiple: c_int);
        safe fn pc_pick_export(path: *const c_char);
    }

    /// The dialog the app started and is waiting on: the answer channel plus the files the
    /// delegate has delivered so far (multi-select delivers one callback per file, `last` flags
    /// the end). Save As also remembers the empty staging file the system picker exports and
    /// the destination path callback's name field.
    struct Pending {
        reply: Option<FileDialogReply>,
        files: Vec<(String, Vec<u8>)>,
        last_name: String,
        staging: Option<PathBuf>,
    }

    static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

    pub fn install() {
        pc_set_callbacks(on_open, on_cancel);
    }

    pub fn open(multiple: bool, reply: FileDialogReply) {
        *PENDING.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(Pending { reply: Some(reply), files: Vec::new(), last_name: String::new(), staging: None });
        pc_pick_open(c_int::from(multiple));
    }

    pub fn save_location(suggested: &str, reply: FileDialogReply) {
        let name = std::path::Path::new(suggested).file_name().map(|n| n.to_string_lossy().into_owned()).filter(|n| !n.is_empty());
        let Some(name) = name else {
            reply.send(None);
            return;
        };
        // UIKit's export initializer requires an existing file. Its contents are irrelevant:
        // the system copies this placeholder to the chosen destination, then the app's normal
        // save flow overwrites that destination with the real bytes.
        let dir = std::env::temp_dir().join("photocomfy-save-as");
        if std::fs::create_dir_all(&dir).is_err() {
            reply.send(None);
            return;
        }
        let staging = dir.join(name);
        if std::fs::File::create(&staging).is_err() {
            reply.send(None);
            return;
        }
        let Ok(path) = CString::new(staging.to_string_lossy().as_bytes()) else {
            reply.send(None);
            return;
        };
        *PENDING.lock().unwrap_or_else(PoisonError::into_inner) =
            Some(Pending { reply: Some(reply), files: Vec::new(), last_name: String::new(), staging: Some(staging) });
        pc_pick_export(path.as_ptr());
    }

    /// Present the export sheet for a file the `write` service already staged (see `main.rs`).
    pub fn export(path: &std::path::Path) {
        let Ok(path) = CString::new(path.to_string_lossy().as_bytes()) else {
            return;
        };
        pc_pick_export(path.as_ptr());
    }

    extern "C" fn on_open(name: *const c_char, bytes: *const c_uchar, len: usize, last: c_int) {
        // SAFETY: the pointers are the callback arguments PCDocumentPicker.m passes for the
        // lifetime of the call (the NSData and NSString outlive it), and `len` is that
        // NSData's length — both are copied below before returning.
        let name = if name.is_null() { "file".to_string() } else { unsafe { std::ffi::CStr::from_ptr(name) }.to_string_lossy().into_owned() };
        let data = if bytes.is_null() || len == 0 { Vec::new() } else { unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec() };
        let mut guard = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(pending) = guard.as_mut() else { return };
        pending.last_name = name.clone();
        if !data.is_empty() {
            pending.files.push((name, data));
        }
        if last != 0 {
            let Some(mut done) = guard.take() else { return };
            if let Some(staging) = done.staging.take() {
                let _ = std::fs::remove_file(staging);
            }
            if let Some(reply) = done.reply.take() {
                let answer = match done.files.len() {
                    // An export picker reports its destination as a name with no bytes; an open
                    // picker reports an unreadable selection as an empty name, which stays Cancel.
                    0 if !done.last_name.is_empty() => Some(FileDialogAnswer::SaveTo(done.last_name.clone())),
                    0 => None,
                    1 => Some(FileDialogAnswer::Contents(done.files[0].0.clone(), std::mem::take(&mut done.files[0].1))),
                    _ => Some(FileDialogAnswer::ContentsMany(std::mem::take(&mut done.files))),
                };
                reply.send(answer);
            }
        }
    }

    extern "C" fn on_cancel() {
        let mut guard = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(pending) = guard.as_mut() {
            if let Some(staging) = pending.staging.take() {
                let _ = std::fs::remove_file(staging);
            }
            if let Some(reply) = pending.reply.take() {
                reply.send(None);
            }
        }
        *guard = None;
    }
}

#[cfg(not(target_os = "ios"))]
mod stubs {
    use photocraft_ui_egui::FileDialogReply;

    pub fn install() {}

    /// No picker off-device: the reply drops unanswered, which the app reads as Cancel.
    pub fn open(_multiple: bool, _reply: FileDialogReply) {}

    /// The Save As location picker is likewise iOS-only in the shell's services.
    pub fn save_location(_suggested: &str, _reply: FileDialogReply) {}

    #[allow(dead_code)]
    pub fn export(_path: &std::path::Path) {}
}
