use arboard::{Clipboard, ImageData};
use std::borrow::Cow;

#[cfg(target_os = "macos")]
use arboard::SetExtApple;

/// How a clipboard write should be treated for history and pasteboard semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardWriteMode {
    /// Copy to clipboard without recording in history (`exclude_from_history` on macOS).
    Copy,
    /// Prepare pasteboard for pasting into another app (standard write + mark own).
    Paste,
}

/// Write text to the system clipboard.
pub fn write_text<'a>(
    clipboard: &mut Clipboard,
    text: impl Into<Cow<'a, str>>,
    mode: ClipboardWriteMode,
) -> Result<(), String> {
    let text = text.into();
    #[cfg(target_os = "macos")]
    {
        match mode {
            ClipboardWriteMode::Copy => {
                clipboard
                    .set()
                    .exclude_from_history()
                    .text(text)
                    .map_err(|e| e.to_string())?;
            }
            ClipboardWriteMode::Paste => {
                clipboard.set_text(text).map_err(|e| e.to_string())?;
            }
        }
        let count = crate::clipboard_macos::change_count();
        crate::clipboard_macos::mark_own_clipboard_write(count);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = mode;
        clipboard.set_text(text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write image pixels to the system clipboard.
pub fn write_image(
    clipboard: &mut Clipboard,
    image: ImageData<'static>,
    mode: ClipboardWriteMode,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        match mode {
            ClipboardWriteMode::Copy => {
                clipboard
                    .set()
                    .exclude_from_history()
                    .image(image)
                    .map_err(|e| e.to_string())?;
            }
            ClipboardWriteMode::Paste => {
                clipboard.set_image(image).map_err(|e| e.to_string())?;
            }
        }
        let count = crate::clipboard_macos::change_count();
        crate::clipboard_macos::mark_own_clipboard_write(count);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = mode;
        clipboard.set_image(image).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Write text to the system clipboard without recording it in Copyosity history.
pub fn set_text<'a>(
    clipboard: &mut Clipboard,
    text: impl Into<Cow<'a, str>>,
) -> Result<(), String> {
    write_text(clipboard, text, ClipboardWriteMode::Copy)
}

/// Write image pixels to the system clipboard without recording it in Copyosity history.
pub fn set_image(clipboard: &mut Clipboard, image: ImageData<'static>) -> Result<(), String> {
    write_image(clipboard, image, ClipboardWriteMode::Copy)
}
