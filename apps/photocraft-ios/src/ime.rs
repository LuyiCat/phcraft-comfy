use std::os::raw::c_uchar;

static EVENTS: std::sync::Mutex<Vec<egui::Event>> = std::sync::Mutex::new(Vec::new());

fn text(ptr: *const c_uchar, len: usize) -> String {
    if ptr.is_null() || len == 0 {
        return String::new();
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(bytes).into_owned()
}

fn push(event: egui::Event) {
    EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(event);
}

fn utf16_offset_to_chars(text: &str, offset: usize) -> usize {
    String::from_utf16_lossy(&text.encode_utf16().take(offset).collect::<Vec<_>>()).chars().count()
}

pub fn drain(raw: &mut egui::RawInput) {
    raw.events.extend(EVENTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).drain(..));
}

#[unsafe(no_mangle)]
extern "C" fn pc_ime_text(ptr: *const c_uchar, len: usize) {
    let text = text(ptr, len);
    if !text.is_empty() {
        push(egui::Event::Text(text));
    }
}

#[unsafe(no_mangle)]
extern "C" fn pc_ime_preedit(ptr: *const c_uchar, len: usize, selected_start: usize, selected_end: usize) {
    let text = text(ptr, len);
    let active_range_chars = (selected_start <= selected_end && selected_end <= text.encode_utf16().count())
        .then_some(utf16_offset_to_chars(&text, selected_start)..utf16_offset_to_chars(&text, selected_end));
    push(egui::Event::Ime(egui::ImeEvent::Preedit { text, active_range_chars }));
}

#[unsafe(no_mangle)]
extern "C" fn pc_ime_commit(ptr: *const c_uchar, len: usize) {
    push(egui::Event::Ime(egui::ImeEvent::Commit(text(ptr, len))));
}

#[unsafe(no_mangle)]
extern "C" fn pc_ime_backspace() {
    for pressed in [true, false] {
        push(egui::Event::Key { key: egui::Key::Backspace, physical_key: None, pressed, repeat: false, modifiers: egui::Modifiers::NONE });
    }
}
