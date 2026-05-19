//! macOS-only "fn + Shift" hold-to-record hotkey, implemented via CGEventTap.
//!
//! The Carbon-era `RegisterEventHotKey` API (which `tauri-plugin-global-shortcut`
//! uses on macOS) can't register `fn` as a modifier — `fn` is reported via
//! `kCGEventFlagMaskSecondaryFn` on raw CGEvents, which live below the hotkey
//! registration layer. So we install a session-level event tap on a background
//! thread, watch flag-change events, and emit our own `hotkey:down/up` events
//! when fn + Shift transitions in/out.
//!
//! Requires Accessibility permission (which is already required for synthetic
//! paste).

#![cfg(target_os = "macos")]

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use tauri::{AppHandle, Emitter};

// Bit flags on CGEventFlags. From CGEventTypes.h.
const FLAG_SHIFT: u64 = 0x0002_0000;
const FLAG_SECONDARY_FN: u64 = 0x0080_0000;

// CGEventType::FlagsChanged
const EVENT_FLAGS_CHANGED: u32 = 12;
// 1 << EVENT_FLAGS_CHANGED
const MASK_FLAGS_CHANGED: u64 = 1u64 << EVENT_FLAGS_CHANGED;

// CGEventTapLocation::HID
const TAP_HID: u32 = 0;
// CGEventTapPlacement::HeadInsertEventTap
const PLACE_HEAD_INSERT: u32 = 0;
// CGEventTapOptions::ListenOnly — we never modify events, so listen-only is
// both lighter-weight and avoids needing kAXTrustedCheck a second time.
const OPT_LISTEN_ONLY: u32 = 1;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(
            proxy: *mut c_void,
            ty: u32,
            event: *mut c_void,
            user_info: *mut c_void,
        ) -> *mut c_void,
        user_info: *mut c_void,
    ) -> *mut c_void; // CFMachPortRef

    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CGEventGetFlags(event: *mut c_void) -> u64;
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: isize,
    ) -> *mut c_void; // CFRunLoopSourceRef
    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: *mut c_void);
    fn CFRelease(cf: *const c_void);
    static kCFRunLoopCommonModes: *const c_void;
}

struct Callback {
    app: AppHandle,
    was_active: AtomicBool,
}

extern "C" fn tap_callback(
    _proxy: *mut c_void,
    ty: u32,
    event: *mut c_void,
    user_info: *mut c_void,
) -> *mut c_void {
    if ty != EVENT_FLAGS_CHANGED || user_info.is_null() || event.is_null() {
        return event;
    }

    // Safety: we keep the Arc alive for the lifetime of the run loop in `start`.
    let cb: &Callback = unsafe { &*(user_info as *const Callback) };

    let flags = unsafe { CGEventGetFlags(event) };
    let is_active = (flags & FLAG_SHIFT) != 0 && (flags & FLAG_SECONDARY_FN) != 0;
    let was = cb.was_active.load(Ordering::Relaxed);

    if is_active && !was {
        let _ = cb.app.emit("hotkey:down", ());
    } else if !is_active && was {
        let _ = cb.app.emit("hotkey:up", ());
    }
    cb.was_active.store(is_active, Ordering::Relaxed);

    event
}

/// Pointer wrapper that asserts Send for raw Core Foundation refs.
/// Safe because these are reference-counted via CF and we serialize access.
struct SendablePtr(*mut c_void);
unsafe impl Send for SendablePtr {}

pub struct FnHotkey {
    runloop: Mutex<Option<SendablePtr>>,
    thread: Mutex<Option<JoinHandle<()>>>,
    _cb: Arc<Callback>,
}

impl FnHotkey {
    pub fn start(app: AppHandle) -> Self {
        let cb = Arc::new(Callback {
            app,
            was_active: AtomicBool::new(false),
        });
        let cb_for_thread = cb.clone();

        let (tx, rx) = std::sync::mpsc::sync_channel::<Option<SendablePtr>>(1);

        let thread = std::thread::Builder::new()
            .name("fn-hotkey-tap".into())
            .spawn(move || {
                let user_info = Arc::as_ptr(&cb_for_thread) as *mut c_void;

                unsafe {
                    let tap = CGEventTapCreate(
                        TAP_HID,
                        PLACE_HEAD_INSERT,
                        OPT_LISTEN_ONLY,
                        MASK_FLAGS_CHANGED,
                        tap_callback,
                        user_info,
                    );
                    if tap.is_null() {
                        eprintln!("fn hotkey: CGEventTapCreate returned null (missing Accessibility permission?)");
                        let _ = tx.send(None);
                        return;
                    }

                    let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
                    if source.is_null() {
                        eprintln!("fn hotkey: CFMachPortCreateRunLoopSource failed");
                        CFRelease(tap);
                        let _ = tx.send(None);
                        return;
                    }

                    let rl = CFRunLoopGetCurrent();
                    CFRunLoopAddSource(rl, source, kCFRunLoopCommonModes);
                    CGEventTapEnable(tap, true);

                    let _ = tx.send(Some(SendablePtr(rl)));

                    CFRunLoopRun();

                    // Loop stopped — clean up.
                    CGEventTapEnable(tap, false);
                    CFRelease(source);
                    CFRelease(tap);
                }
            })
            .expect("spawn fn-hotkey thread");

        let runloop = rx.recv().ok().flatten();

        FnHotkey {
            runloop: Mutex::new(runloop),
            thread: Mutex::new(Some(thread)),
            _cb: cb,
        }
    }

    pub fn stop(&self) {
        let rl = self.runloop.lock().unwrap().take();
        if let Some(SendablePtr(rl)) = rl {
            unsafe { CFRunLoopStop(rl) };
        }
        if let Some(t) = self.thread.lock().unwrap().take() {
            let _ = t.join();
        }
    }
}

impl Drop for FnHotkey {
    fn drop(&mut self) {
        self.stop();
    }
}
