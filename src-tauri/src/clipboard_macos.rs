//! macOS pasteboard: change detection, paste target, skipping app-owned copies.

use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSPasteboard, NSRunningApplication, NSWorkspace};
#[cfg(target_os = "macos")]
use objc2_foundation::{ns_string, NSString};

static IGNORE_CAPTURE_AT: AtomicI64 = AtomicI64::new(-1);
static PASTE_TARGET_PID: AtomicI32 = AtomicI32::new(0);

pub fn change_count() -> i64 {
    #[cfg(target_os = "macos")]
    {
        let pasteboard = NSPasteboard::generalPasteboard();
        pasteboard.changeCount() as i64
    }
    #[cfg(not(target_os = "macos"))]
    {
        0
    }
}

pub fn mark_own_clipboard_write(change_count: i64) {
    IGNORE_CAPTURE_AT.store(change_count, Ordering::SeqCst);
}

pub fn should_ignore_capture(change_count: i64) -> bool {
    change_count == IGNORE_CAPTURE_AT.load(Ordering::SeqCst)
}

pub fn is_concealed() -> bool {
    #[cfg(target_os = "macos")]
    {
        let pasteboard = NSPasteboard::generalPasteboard();
        let ty = ns_string!("org.nspasteboard.ConcealedType");
        pasteboard.dataForType(ty).is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Remember frontmost app before the panel becomes key (call before `show_and_make_key`).
pub fn remember_paste_target() {
    if let Some(pid) = frontmost_pid_excluding_self() {
        PASTE_TARGET_PID.store(pid, Ordering::SeqCst);
    }
}

/// Reactivate the app that had focus before Copyosity (call after `hide_panel`, before Cmd+V).
pub fn restore_paste_target() {
    let pid = PASTE_TARGET_PID.load(Ordering::SeqCst);
    if pid <= 0 {
        return;
    }
    if activate_pid(pid) {
        std::thread::sleep(std::time::Duration::from_millis(90));
    }
    if frontmost_pid() != Some(pid) && activate_pid(pid) {
        std::thread::sleep(std::time::Duration::from_millis(120));
    }
}

#[cfg(target_os = "macos")]
fn frontmost_pid_excluding_self() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid == std::process::id() as i32 {
        return None;
    }
    Some(pid)
}

#[cfg(not(target_os = "macos"))]
fn frontmost_pid_excluding_self() -> Option<i32> {
    None
}

#[cfg(target_os = "macos")]
fn frontmost_pid() -> Option<i32> {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    Some(app.processIdentifier())
}

#[cfg(not(target_os = "macos"))]
fn frontmost_pid() -> Option<i32> {
    None
}

/// Post synthetic Cmd+V (requires Accessibility).
pub fn simulate_cmd_v() {
    unsafe {
        type CGEventRef = *mut std::ffi::c_void;

        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGEventCreateKeyboardEvent(
                source: *mut std::ffi::c_void,
                keycode: u16,
                key_down: bool,
            ) -> CGEventRef;
            fn CGEventSetFlags(event: CGEventRef, flags: u64);
            fn CGEventPost(tap: u32, event: CGEventRef);
            fn CFRelease(cf: *mut std::ffi::c_void);
        }

        const K_CG_EVENT_FLAG_COMMAND: u64 = 0x00100000;
        const K_CG_HID_EVENT_TAP: u32 = 0;
        const K_V_KEYCODE: u16 = 9;

        let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), K_V_KEYCODE, true);
        let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), K_V_KEYCODE, false);

        if !event_down.is_null() && !event_up.is_null() {
            CGEventSetFlags(event_down, K_CG_EVENT_FLAG_COMMAND);
            CGEventSetFlags(event_up, K_CG_EVENT_FLAG_COMMAND);
            CGEventPost(K_CG_HID_EVENT_TAP, event_down);
            CGEventPost(K_CG_HID_EVENT_TAP, event_up);
            CFRelease(event_down);
            CFRelease(event_up);
        }
    }
}

#[cfg(target_os = "macos")]
fn activate_pid(pid: i32) -> bool {
    use objc2_app_kit::NSApplicationActivationOptions;
    let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) else {
        return false;
    };
    app.activateWithOptions(NSApplicationActivationOptions::empty())
}

#[cfg(not(target_os = "macos"))]
fn activate_pid(_pid: i32) -> bool {
    false
}

/// Live accessibility check via a real AX API call (not `AXIsProcessTrusted` cache).
#[cfg(target_os = "macos")]
fn accessibility_live_check() -> bool {
    const K_AX_ERROR_SUCCESS: i32 = 0;
    const K_AX_ERROR_NO_VALUE: i32 = -25205;

    unsafe {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXUIElementCreateSystemWide() -> *mut std::ffi::c_void;
            fn AXUIElementCopyAttributeValue(
                element: *mut std::ffi::c_void,
                attribute: *const std::ffi::c_void,
                value: *mut *mut std::ffi::c_void,
            ) -> i32;
            fn CFRelease(cf: *mut std::ffi::c_void);
        }

        let system = AXUIElementCreateSystemWide();
        if system.is_null() {
            return false;
        }

        let attr = NSString::from_str("AXFocusedApplication");
        let mut value: *mut std::ffi::c_void = std::ptr::null_mut();
        let err = AXUIElementCopyAttributeValue(
            system,
            objc2::rc::Retained::as_ptr(&attr).cast(),
            &mut value,
        );
        if !value.is_null() {
            CFRelease(value);
        }
        CFRelease(system);

        err == K_AX_ERROR_SUCCESS || err == K_AX_ERROR_NO_VALUE
    }
}

#[cfg(target_os = "macos")]
fn accessibility_show_prompt() {
    unsafe {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrustedWithOptions(options: *const std::ffi::c_void) -> bool;
        }

        use objc2_foundation::{NSDictionary, NSNumber, ns_string};

        let key = ns_string!("AXTrustedCheckOptionPrompt");
        let yes = NSNumber::new_bool(true);
        let dict = NSDictionary::from_slices(&[key], &[&*yes]);
        let _ = AXIsProcessTrustedWithOptions(objc2::rc::Retained::as_ptr(&dict).cast());
    }
}

/// Accessibility trust check. `prompt: true` always asks macOS to show its trust dialog.
#[cfg(target_os = "macos")]
pub fn accessibility_trusted(prompt: bool) -> bool {
    if prompt {
        accessibility_show_prompt();
    }
    accessibility_live_check()
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_trusted(_prompt: bool) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_clipboard_write_is_ignored_once() {
        mark_own_clipboard_write(42);
        assert!(should_ignore_capture(42));
        assert!(!should_ignore_capture(43));

        mark_own_clipboard_write(99);
        assert!(should_ignore_capture(99));
        assert!(!should_ignore_capture(42));
    }
}
