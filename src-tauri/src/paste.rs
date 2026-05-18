use std::time::Duration;

use arboard::Clipboard;

use crate::error::TranscribeError;

const COPY_TO_KEYSTROKE: Duration = Duration::from_millis(80);
const KEYSTROKE_TO_RESTORE: Duration = Duration::from_millis(200);

pub async fn paste_text(text: String) -> Result<(), TranscribeError> {
    if text.is_empty() {
        return Ok(());
    }

    tokio::task::spawn_blocking(move || paste_blocking(&text))
        .await
        .map_err(|e| TranscribeError::Other(format!("join: {e}")))?
}

fn paste_blocking(text: &str) -> Result<(), TranscribeError> {
    let mut clipboard = Clipboard::new()
        .map_err(|e| TranscribeError::Other(format!("clipboard init: {e}")))?;

    // Save current text clipboard so we can restore it. If the clipboard holds
    // a non-text payload (image, files), get_text errors and we skip restore.
    let saved = clipboard.get_text().ok();

    clipboard
        .set_text(text.to_string())
        .map_err(|e| TranscribeError::Other(format!("clipboard set: {e}")))?;

    std::thread::sleep(COPY_TO_KEYSTROKE);

    send_paste_keystroke()?;

    std::thread::sleep(KEYSTROKE_TO_RESTORE);

    if let Some(prev) = saved {
        let _ = clipboard.set_text(prev);
    }

    Ok(())
}

#[cfg(target_os = "macos")]
fn send_paste_keystroke() -> Result<(), TranscribeError> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    // 'v' keycode on US layout; the Cmd modifier flag tells the system to
    // interpret this as paste regardless of layout.
    const KEY_V: u16 = 9;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| TranscribeError::Other("CGEventSource::new failed".into()))?;

    let down = CGEvent::new_keyboard_event(source.clone(), KEY_V, true)
        .map_err(|_| TranscribeError::Other("CGEvent keydown failed".into()))?;
    down.set_flags(CGEventFlags::CGEventFlagCommand);
    down.post(CGEventTapLocation::HID);

    let up = CGEvent::new_keyboard_event(source, KEY_V, false)
        .map_err(|_| TranscribeError::Other("CGEvent keyup failed".into()))?;
    up.set_flags(CGEventFlags::CGEventFlagCommand);
    up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_paste_keystroke() -> Result<(), TranscribeError> {
    Err(TranscribeError::Other(
        "paste keystroke not implemented for this platform".into(),
    ))
}
