//! Native HTTP transport (see `photocraft_engine::comfy_cmds`): blocking ureq calls, one request
//! per call. It speaks both the local ComfyUI protocol and a configurable OpenAI-compatible Image
//! API; the engine calls it from worker threads, so a slow server never blocks the UI.

use std::io::Read;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use photocraft_engine::comfy_cmds::{ComfyCall, ComfyFn, ComfyReply, ComfyRequest, ImageApiImage, image_api_base_url, image_api_images};
use serde_json::{Value, json};

/// JSON calls connect and answer in seconds; a big image download may legitimately take minutes.
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const IMAGE_API_REPLY_MAX: u64 = 1024 * 1024 * 1024;
const IMAGE_API_URL_TIMEOUT: Duration = Duration::from_secs(120);
/// Never read more than this from one reply (a broken server must not exhaust memory).
const REPLY_MAX: u64 = 2 * 1024 * 1024 * 1024;
/// One readiness probe while a launched ComfyUI is still loading.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);
/// How long a started ComfyUI gets to come up (model imports can make a cold start slow).
const START_TIMEOUT: Duration = Duration::from_secs(180);
const START_POLL: Duration = Duration::from_millis(500);
/// The tail of the server's console log an error shows, so a failing launch says why.
const LOG_TAIL: u64 = 400;

/// The ComfyUI this shell started, so a second Start waits instead of spawning a twin, and the
/// handle reaps the child when it exits. Poisoning just means another start raced us: the child
/// it holds is as good as ours.
static STARTED: Mutex<Option<std::process::Child>> = Mutex::new(None);

pub fn transport() -> ComfyFn {
    std::sync::Arc::new(|req: &ComfyRequest| call(&req.url, &req.call))
}

fn call(url: &str, call: &ComfyCall) -> Result<ComfyReply, String> {
    match call {
        ComfyCall::SystemStats => Ok(ComfyReply::Json(get_json(url, "/system_stats")?)),
        // The whole document at once: the per-node endpoint returns `{}` on current servers.
        ComfyCall::ObjectInfo => Ok(ComfyReply::Json(get_json(url, "/object_info")?)),
        ComfyCall::Queue { workflow, client_id } => {
            let body = serde_json::json!({"prompt": workflow, "client_id": client_id});
            Ok(ComfyReply::Json(post_json(url, "/prompt", &body)?))
        }
        // Fire and forget: the answer, if any, is nothing we act on.
        ComfyCall::Interrupt => {
            let _ = post_json(url, "/interrupt", &serde_json::json!({}));
            Ok(ComfyReply::Json(Value::Null))
        }
        ComfyCall::History { prompt_id } => {
            let path = format!("/history/{}", path_segment(prompt_id)?);
            Ok(ComfyReply::Json(get_json(url, &path)?))
        }
        ComfyCall::Download { filename, subfolder, kind } => {
            Ok(ComfyReply::Bytes(get_bytes(url, "/view", &[("filename", filename), ("subfolder", subfolder), ("type", kind)])?))
        }
        ComfyCall::Upload { name, bytes } => Ok(ComfyReply::Json(upload(url, name, bytes)?)),
        // Auto-start: probe, launch, wait. Loopback only — the engine gates it too, but a
        // transport must never run a launch command pointed at somebody else's machine.
        ComfyCall::Start { command } => start(url, command),
        ComfyCall::ApiImage { base, key, model, prompt, image, timeout_s } => {
            let timeout = Duration::from_secs((*timeout_s).clamp(10, 3600)).max(CALL_TIMEOUT);
            let base = image_api_base_url(base)?;
            let reply = match image {
                Some(png) => post_edits(&base, key, model, prompt, png, timeout)?,
                None => post_generations(&base, key, model, prompt, timeout)?,
            };
            match image_api_images(&reply)?.first() {
                Some(ImageApiImage::Bytes(bytes)) => Ok(ComfyReply::Bytes(bytes.clone())),
                Some(ImageApiImage::Url(url)) => Ok(ComfyReply::Bytes(get_url(url, key)?)),
                None => Err("the Image API replied without an image".into()),
            }
        }
        ComfyCall::ApiCheck { base, key } => {
            let base = image_api_base_url(base)?;
            Ok(ComfyReply::Json(api_get_json(&base, "/models", key, CALL_TIMEOUT, "the model list")?))
        }
    }
}

/// A path segment must be server-generated (a prompt id); anything else is refused, not encoded.
fn path_segment(s: &str) -> Result<&str, String> {
    if !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        Ok(s)
    } else {
        Err(format!("ComfyUI sent an unusable prompt id {s:?}"))
    }
}

fn get_json(url: &str, path: &str) -> Result<Value, String> {
    let resp = ureq::get(&format!("{url}{path}")).config().timeout_global(Some(CALL_TIMEOUT)).build().call();
    read_json(resp)
}

fn post_json(url: &str, path: &str, body: &Value) -> Result<Value, String> {
    let resp = ureq::post(&format!("{url}{path}")).config().timeout_global(Some(CALL_TIMEOUT)).build().send_json(body);
    read_json(resp)
}

/// A non-2xx still carries the server's reason (a rejected workflow's node errors) in its body.
fn read_json(resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<Value, String> {
    let resp = resp.map_err(|e| format!("{e}"))?;
    let status = resp.status();
    let body = read_body(resp)?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", preview(&body)));
    }
    serde_json::from_slice(&body).map_err(|e| format!("ComfyUI's reply is not JSON: {e}"))
}

fn get_bytes(url: &str, path: &str, queries: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    let mut request = ureq::get(&format!("{url}{path}")).config().timeout_global(Some(DOWNLOAD_TIMEOUT)).build();
    for (k, v) in queries {
        request = request.query(k, v);
    }
    let resp = request.call().map_err(|e| format!("{e}"))?;
    let status = resp.status();
    let body = read_body(resp)?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {}", preview(&body)));
    }
    Ok(body)
}

fn upload(url: &str, name: &str, bytes: &[u8]) -> Result<Value, String> {
    use ureq::unversioned::multipart::{Form, Part};
    let part = Part::bytes(bytes).file_name(name).mime_str("image/png").map_err(|e| format!("internal multipart error: {e}"))?;
    let form = Form::new().text("overwrite", "true").part("image", part);
    let resp = ureq::post(&format!("{url}/upload/image")).config().timeout_global(Some(DOWNLOAD_TIMEOUT)).build().send(form);
    read_json(resp)
}

/// `POST /images/edits`: the source image, model and prompt as multipart form data.
fn post_edits(base: &str, key: &str, model: &str, prompt: &str, png: &[u8], timeout: Duration) -> Result<Value, String> {
    use ureq::unversioned::multipart::{Form, Part};
    let part = Part::bytes(png).file_name("photocraft.png").mime_str("image/png").map_err(|e| format!("internal multipart error: {e}"))?;
    let form = Form::new().text("model", model).text("prompt", prompt).part("image", part);
    let request = ureq::post(&format!("{base}/images/edits")).config().timeout_global(Some(timeout)).build();
    let request = if key.is_empty() { request } else { request.header("Authorization", format!("Bearer {key}")) };
    read_json(request.send(form)).map_err(|e| format!("the image edit: {e}"))
}

/// `POST /images/generations`: text-only generation for a document-free call.
fn post_generations(base: &str, key: &str, model: &str, prompt: &str, timeout: Duration) -> Result<Value, String> {
    let request = ureq::post(&format!("{base}/images/generations")).config().timeout_global(Some(timeout)).build();
    let request = if key.is_empty() { request } else { request.header("Authorization", format!("Bearer {key}")) };
    read_json(request.send_json(json!({"model": model, "prompt": prompt}))).map_err(|e| format!("the image generation: {e}"))
}

fn api_get_json(base: &str, path: &str, key: &str, timeout: Duration, what: &str) -> Result<Value, String> {
    let request = ureq::get(&format!("{base}{path}")).config().timeout_global(Some(timeout)).build();
    let request = if key.is_empty() { request } else { request.header("Authorization", format!("Bearer {key}")) };
    read_json(request.call()).map_err(|e| format!("{what}: {e}"))
}

/// Fetch an image URL returned by a custom OpenAI-compatible relay.
fn get_url(url: &str, key: &str) -> Result<Vec<u8>, String> {
    let request = ureq::get(url).config().timeout_global(Some(IMAGE_API_URL_TIMEOUT)).build();
    let request = if key.is_empty() { request } else { request.header("Authorization", format!("Bearer {key}")) };
    let resp = request.call().map_err(|e| format!("fetching the Image API image: {e}"))?;
    let status = resp.status();
    let body = read_body_with_limit(resp, IMAGE_API_REPLY_MAX)?;
    if !status.is_success() {
        return Err(format!("fetching the Image API image: HTTP {status}: {}", preview(&body)));
    }
    if body.is_empty() {
        return Err("the Image API returned an empty image".into());
    }
    Ok(body)
}

fn read_body(resp: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>, String> {
    read_body_with_limit(resp, REPLY_MAX)
}

fn read_body_with_limit(resp: ureq::http::Response<ureq::Body>, limit: u64) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    resp.into_body().into_reader().take(limit).read_to_end(&mut body).map_err(|e| format!("reading the reply: {e}"))?;
    Ok(body)
}

// ------------------------------------------------------------------ auto-start

/// The host of `url` (an `http(s)://…` address): between the scheme and the first `/`, minus
/// the port. IPv6 brackets come off, so `http://[::1]:8188` reads as `::1`.
fn host_of(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let host = rest.split('/').next().unwrap_or(rest);
    // An IPv6 host is the part in brackets; anything else ends at its port.
    if let Some(inner) = host.strip_prefix('[') {
        return inner.split(']').next().unwrap_or(inner);
    }
    host.split(':').next().unwrap_or(host)
}

/// True when the host names this machine: only such a server may be started from here.
fn is_loopback(url: &str) -> bool {
    matches!(host_of(url), "127.0.0.1" | "localhost" | "::1")
}

/// Is the server at `url` answering `/system_stats`? A short, separate probe: a connection
/// refused or a loading server's silence is a normal answer here, not an error to report.
fn probe(url: &str) -> Option<Value> {
    let resp = ureq::get(&format!("{url}/system_stats")).config().timeout_global(Some(PROBE_TIMEOUT)).build().call();
    read_json(resp).ok()
}

/// The file a started ComfyUI's console output goes to, truncated per start: a failing launch
/// says why from there, and a user can read it after a crash. Under the temp dir, not the
/// config dir, so it never mixes into synced settings.
fn server_log() -> std::path::PathBuf {
    std::env::temp_dir().join("photocraft-comfyui.log")
}

/// The shell wrapper a launch command runs through: the user writes one line, arguments and
/// `cd … &&` included, so it needs a shell to interpret it.
fn shell_command(command: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    let mut cmd = std::process::Command::new("cmd");
    #[cfg(not(target_os = "windows"))]
    let mut cmd = std::process::Command::new("sh");
    #[cfg(target_os = "windows")]
    cmd.arg("/C").arg(command);
    #[cfg(not(target_os = "windows"))]
    cmd.arg("-c").arg(command);
    cmd
}

/// Run the user's launch command and wait until this machine's ComfyUI answers. An empty
/// command asks for a ComfyUI found on this machine ([`crate::comfy_detect`]: a ComfyUI Desktop install,
/// or a checkout with a virtualenv). The server stays running after the app exits (it is the
/// user's server, not a child we own); the Child handle is kept only to notice an early exit and
/// reap it.
fn start(url: &str, command: &str) -> Result<ComfyReply, String> {
    start_in(url, command, &crate::comfy_detect::search_roots())
}

/// [`start`] against explicit search roots, so a test never starts this machine's real install.
fn start_in(url: &str, command: &str, roots: &[std::path::PathBuf]) -> Result<ComfyReply, String> {
    if !is_loopback(url) {
        return Err(format!("{url} is not on this machine; only a local server (like http://127.0.0.1:8188) can be started"));
    }
    let command = match command.trim() {
        "" => crate::comfy_detect::find(roots).map(|install| install.command(crate::comfy_detect::url_port(url))).ok_or_else(|| {
            "no local ComfyUI was found to start; set the ComfyUI launch command in Preferences, Integrations (for example `cd /opt/ComfyUI && python main.py --port 8188`)".to_string()
        })?,
        command => command.to_string(),
    };
    // Already up (or another window's start finished first): no spawn, just confirm.
    if let Some(stats) = probe(url) {
        return Ok(ComfyReply::Json(stats));
    }
    let mut guard = STARTED.lock().unwrap_or_else(PoisonError::into_inner);
    // Reap a previous child that has exited since, so the slot is meaningful.
    if let Some(child) = guard.as_mut()
        && child.try_wait().ok().flatten().is_some()
    {
        *guard = None;
    }
    if guard.is_none() {
        let mut log = std::fs::File::create(server_log());
        let mut cmd = shell_command(&command);
        if let Ok(f) = &mut log {
            use std::io::Write as _;
            let _ = writeln!(f, "$ {command}");
            cmd.stdout(f.try_clone().map_err(|e| format!("duplicating the ComfyUI log handle: {e}"))?)
                .stderr(f.try_clone().map_err(|e| format!("duplicating the ComfyUI log handle: {e}"))?);
        }
        match cmd.spawn() {
            Ok(child) => *guard = Some(child),
            // The log stays meaningful even without handles: the shell itself wrote nothing.
            Err(e) => return Err(format!("could not run `{command}`: {e}")),
        }
    }
    let deadline = Instant::now() + START_TIMEOUT;
    loop {
        if let Some(stats) = probe(url) {
            return Ok(ComfyReply::Json(stats));
        }
        // A quick exit means the command itself failed (bad path, missing python, port clash):
        // say so with the console tail instead of waiting out the whole timeout.
        if let Some(child) = guard.as_mut() {
            let status = child.try_wait().map_err(|e| format!("checking the started ComfyUI: {e}"))?;
            if let Some(status) = status {
                *guard = None;
                return Err(format!("`{command}` exited with {status}. Last output: {}", log_tail()));
            }
        }
        if Instant::now() >= deadline {
            return Err(format!("ComfyUI did not answer at {url} within {} s. Last output: {}", START_TIMEOUT.as_secs(), log_tail()));
        }
        std::thread::sleep(START_POLL);
    }
}

/// The end of the started server's console log, trimmed to a line the status bar can show.
fn log_tail() -> String {
    let mut text = std::fs::read(server_log()).map(|b| String::from_utf8_lossy(&b).trim().to_string()).unwrap_or_default();
    if text.chars().count() > LOG_TAIL as usize {
        let skip = text.char_indices().nth_back(LOG_TAIL as usize - 1).map_or(0, |(i, _)| i);
        // Start at a character boundary at or after `skip` (from_utf8_lossy output is valid UTF-8).
        text = text[skip..].trim_start().to_string();
    }
    text
}

/// The first characters of an error body, enough to act on without flooding the status bar.
fn preview(body: &[u8]) -> String {
    let text = String::from_utf8_lossy(body);
    let cut = text.char_indices().take_while(|(i, _)| *i < 400).last().map_or(0, |(i, c)| i + c.len_utf8());
    text.get(..cut).unwrap_or_default().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn host_of_handles_ports_paths_and_ipv6() {
        assert_eq!(host_of("http://127.0.0.1:8188"), "127.0.0.1");
        assert_eq!(host_of("http://localhost:8188/system_stats"), "localhost");
        assert_eq!(host_of("https://comfy.example.com"), "comfy.example.com");
        assert_eq!(host_of("http://[::1]:8188"), "::1");
        // Not a URL at all: the whole string is the host, as unreadable rather than wrong.
        assert_eq!(host_of("localhost"), "localhost");
    }

    #[test]
    fn only_this_machine_is_startable() {
        for url in ["http://127.0.0.1:8188", "http://localhost", "http://localhost:8188/", "http://[::1]:8188"] {
            assert!(is_loopback(url), "{url}");
        }
        for url in ["http://192.168.1.5:8188", "https://comfy.example.com", "http://[2001:db8::2]:8188", "http://0.0.0.0:8188", "file:///tmp"] {
            assert!(!is_loopback(url), "{url}");
        }
    }

    #[test]
    fn start_refuses_what_it_must_not_run() {
        // A remote server is refused before anything is spawned.
        let e = start_in("https://comfy.example.com", "python main.py", &[]).err().unwrap_or_default();
        assert!(e.contains("not on this machine"), "{e}");
        // An empty command means "start a ComfyUI found here": with none, the error says where
        // to set one, and nothing was spawned.
        let e = start_in("http://127.0.0.1:8188", "   ", &[]).err().unwrap_or_default();
        assert!(e.contains("no local ComfyUI") && e.contains("Integrations"), "{e}");
        // The shell wrapper carries the command as one argument, quotes and all.
        let cmd = shell_command("cd /opt/ComfyUI && python main.py");
        assert!(format!("{cmd:?}").contains("cd /opt/ComfyUI && python main.py"));
    }

    #[test]
    fn image_api_edit_round_trips_with_custom_model_and_bearer_key() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let image = vec![0x89, b'P', b'N', b'G', 1, 2, 3];
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image);
        let reply = format!(r#"{{"data":[{{"b64_json":"{encoded}"}}]}}"#);
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let mut request = [0u8; 16 * 1024];
            let length = stream.read(&mut request).expect("read test request");
            let request = String::from_utf8_lossy(&request[..length]).to_ascii_lowercase();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
            stream.write_all(response.as_bytes()).expect("write test response");
            request
        });
        let transport = transport();
        let result = transport(&ComfyRequest {
            url: format!("http://{addr}"),
            call: ComfyCall::ApiImage {
                base: format!("http://{addr}"),
                key: "sk-custom".into(),
                model: "my-image-model".into(),
                prompt: "a violet cat".into(),
                image: Some(vec![1, 2, 3]),
                timeout_s: 30,
            },
        });
        let request = server.join().expect("test server thread");
        let ComfyReply::Bytes(bytes) = result.expect("image API response") else { panic!("expected image bytes") };
        assert_eq!(bytes, image);
        assert!(request.starts_with("post /v1/images/edits"), "{request}");
        assert!(request.contains("authorization: bearer sk-custom"), "{request}");
        assert!(request.contains("multipart/form-data"), "{request}");
    }

    #[test]
    fn image_api_check_reads_models_from_a_custom_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let reply = r#"{"data":[{"id":"custom-model"}]}"#;
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let mut request = [0u8; 4096];
            let length = stream.read(&mut request).expect("read test request");
            let request = String::from_utf8_lossy(&request[..length]).to_string();
            let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}", reply.len());
            stream.write_all(response.as_bytes()).expect("write test response");
            request
        });
        let transport = transport();
        let result = transport(&ComfyRequest { url: String::new(), call: ComfyCall::ApiCheck { base: format!("http://{addr}"), key: "sk-custom".into() } });
        let request = server.join().expect("test server thread");
        let ComfyReply::Json(value) = result.expect("model list response") else { panic!("expected JSON model list") };
        assert_eq!(value["data"][0]["id"], "custom-model");
        assert!(request.starts_with("GET /v1/models"), "{request}");
        assert!(request.contains("Authorization: Bearer sk-custom") || request.contains("authorization: Bearer sk-custom"), "{request}");
    }
}
