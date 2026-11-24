use winit::platform::android::activity::{AndroidApp, AndroidAppWaker};

static EVENTS: std::sync::Mutex<Vec<egui::Event>> = std::sync::Mutex::new(Vec::new());
static WAKER: std::sync::OnceLock<AndroidAppWaker> = std::sync::OnceLock::new();

pub fn init(app: &AndroidApp) {
    let _ = WAKER.set(app.create_waker());
}

fn push(event: egui::Event) {
    EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(event);
    if let Some(waker) = WAKER.get() {
        waker.wake();
    }
}

pub fn drain(raw: &mut egui::RawInput) {
    raw.events.extend(EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).drain(..));
}

fn utf16_offset_to_chars(text: &str, offset: usize) -> usize {
    String::from_utf16_lossy(&text.encode_utf16().take(offset).collect::<Vec<_>>()).chars().count()
}

#[unsafe(no_mangle)]
extern "C" fn Java_com_photocomfy_app_PhotoComfyActivity_nativeIme(
    env: *mut jni::sys::JNIEnv,
    _class: jni::sys::jclass,
    text: jni::sys::jstring,
    kind: jni::sys::jint,
    selected_start: jni::sys::jint,
    selected_end: jni::sys::jint,
) {
    if env.is_null() || text.is_null() {
        return;
    }
    let interface = unsafe { &(**env).v1_1 };
    let chars = unsafe { (interface.GetStringUTFChars)(env, text, std::ptr::null_mut()) };
    if chars.is_null() {
        return;
    }
    let value = unsafe { std::ffi::CStr::from_ptr(chars) }.to_string_lossy().into_owned();
    unsafe { (interface.ReleaseStringUTFChars)(env, text, chars) };
    let text = value;

    match kind {
        0 if !text.is_empty() => push(egui::Event::Text(text)),
        1 => {
            let count = text.encode_utf16().count();
            let start = selected_start.max(0) as usize;
            let end = selected_end.max(0) as usize;
            let active_range_chars = (start <= end && end <= count).then_some(utf16_offset_to_chars(&text, start)..utf16_offset_to_chars(&text, end));
            push(egui::Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars }));
        }
        2 => {
            push(egui::Event::Ime(egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None }));
            push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
        }
        3 => {
            for pressed in [true, false] {
                push(egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed, repeat: false, modifiers: egui::Modifiers::NONE });
            }
        }
        _ => {}
    }
}

#[derive(Default)]
pub struct ImeBridge {
    active: bool,
}

impl egui::Plugin for ImeBridge {
    fn debug_name(&self) -> &'static str {
        "photocomfy-android-ime"
    }

    fn output_hook(&mut self, _ctx: &egui::Context, output: &mut egui::FullOutput) {
        let active = output.platform_output.ime.is_some();
        if active != self.active {
            self.active = active;
            if let Err(error) = crate::platform::set_text_input_active(active) {
                log::warn!("Android text input: {error}");
            }
        }
    }
}
