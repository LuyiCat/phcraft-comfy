//! Persistent WebAssembly plug-ins for sandboxed shells whose picker returns file contents.
//!
//! Desktop keeps its existing per-process `plugin.install` and preference-folder workflow. A
//! shell with `Services::plugin_import_dir` instead owns one `<manifest id>.wasm` per import in
//! its container; the folder is restored after preferences load on every launch.

use serde_json::{Value, json};

use crate::PhotocraftApp;

/// A file picker's contents become the same `plugin.install` payload the engine already accepts.
/// Keep `path` as the display name in the dialog; `data` takes precedence when OK confirms it.
pub(crate) fn fill_picked(app: &mut PhotocraftApp, id: u64, name: String, bytes: Vec<u8>) -> Result<Value, String> {
    let d = app.ui.dialog_mut(id).ok_or("the dialog was closed")?;
    d.fields.insert("path".into(), json!(name));
    d.fields.insert("data".into(), json!(photocraft_engine::paint::tile::b64_encode(&bytes)));
    Ok(Value::Null)
}

/// Save a successfully installed module under its validated manifest id. The engine has already
/// parsed the module and enforced its size and manifest limits before this runs.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn persist_installed(app: &mut PhotocraftApp, params: &Value, installed: &Value) -> Result<(), String> {
    let Some(dir) = app.services.plugin_import_dir.clone() else { return Ok(()) };
    let id = installed.get("id").and_then(Value::as_str).ok_or("the installed plug-in returned no id")?;
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err("the installed plug-in has an unsafe id".into());
    }
    let bytes = if let Some(data) = params.get("data").and_then(Value::as_str) {
        photocraft_engine::paint::tile::b64_decode(data).ok_or("the picked plug-in data is not valid base64")?
    } else if let Some(path) = params.get("path").and_then(Value::as_str) {
        photocraft_format::read_file(std::path::Path::new(path)).map_err(|e| format!("{path}: {e}"))?
    } else {
        return Err("the installed plug-in has no source bytes".into());
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(format!("{id}.wasm"));
    photocraft_format::atomic_write(&path, &bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Restore the container's plug-ins without changing the user's separate preference folder.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn restore(app: &mut PhotocraftApp) {
    let Some(dir) = app.services.plugin_import_dir.clone().filter(|dir| dir.is_dir()) else { return };
    match app.session.execute("plugin.reload", json!({"path": dir.to_string_lossy()})) {
        Ok(report) => {
            if let Some(failed) = report.get("failed").and_then(Value::as_array).and_then(|failed| failed.first()) {
                app.ui.status = failed.get("error").and_then(Value::as_str).unwrap_or("A saved plug-in could not be loaded").to_string();
                app.ui.status_error = true;
            }
        }
        Err(e) => {
            app.ui.status = format!("Saved plug-ins could not be loaded: {e}");
            app.ui.status_error = true;
        }
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    const ID: &str = "org.photocraft.test.ios-persistence";

    /// A minimal valid filter with an id no other registry test owns.
    fn plugin() -> Vec<u8> {
        let manifest = format!(r#"{{"id":"{ID}","name":"Persistence Test","version":"1","kind":"filter","params":{{}}}}"#);
        let source = format!(
            r#"(module
  (memory (export "memory") 1)
  (data (i32.const 16) "{data}")
  (func (export "pc_abi_version") (result i32) (i32.const 1))
  (func (export "pc_manifest") (result i64) (i64.or (i64.shl (i64.const {len}) (i64.const 32)) (i64.const 16)))
  (func (export "pc_alloc") (param $n i32) (result i32) (local $old i32)
    (local.set $old (memory.grow (i32.shr_u (i32.add (local.get $n) (i32.const 65535)) (i32.const 16))))
    (if (result i32) (i32.eq (local.get $old) (i32.const -1)) (then (i32.const 0)) (else (i32.mul (local.get $old) (i32.const 65536)))))
  (func (export "pc_filter") (param $buf i32) (param $len i32) (param $w i32) (param $h i32)
    (param $ch i32) (param $fmt i32) (param $pp i32) (param $pl i32) (result i32) (i32.const 0)))"#,
            data = manifest.replace('"', "\\\""),
            len = manifest.len()
        );
        wat::parse_str(source).unwrap()
    }

    #[test]
    fn picked_plugin_confirms_to_disk_and_restores_on_startup() {
        let dir = tempfile::tempdir().unwrap();
        let services = crate::Services { plugin_import_dir: Some(dir.path().to_path_buf()), ..Default::default() };
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        let opened = crate::menus::invoke(&mut app, &egui::Context::default(), "plugin.install", json!({})).unwrap();
        let dialog = opened["dialog"].as_u64().unwrap();
        fill_picked(&mut app, dialog, "Picked.wasm".into(), plugin()).unwrap();
        let installed = crate::dialogs::confirm(&mut app, dialog).unwrap();
        assert_eq!(installed["id"], json!(ID));
        assert_eq!(std::fs::read(dir.path().join(format!("{ID}.wasm"))).unwrap(), plugin());
        photocraft_plugins::registry::remove(ID);
        drop(app);

        let services = crate::Services { plugin_import_dir: Some(dir.path().to_path_buf()), ..Default::default() };
        let mut restored = PhotocraftApp::new(photocraft_engine::Session::new(), services);
        crate::prefs_ui::load(&mut restored);
        assert!(photocraft_plugins::registry::get(ID).is_some(), "the saved module is registered again");
        photocraft_plugins::registry::remove(ID);
    }
}
