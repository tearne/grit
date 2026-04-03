use std::io::Write;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};

/// Copies `path:line` to the clipboard via OSC 52 and returns a confirmation
/// string for display.
pub(crate) fn copy(path: &Path, line: u32) -> String {
    let payload = format!(" {}:{}", path.display(), line);
    let encoded = STANDARD.encode(payload.as_bytes());
    let sequence = format!("\x1b]52;c;{encoded}\x07");
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        if tty.write_all(sequence.as_bytes()).is_ok() {
            return format!("Copied: {payload}");
        }
    }
    "OSC 52: failed to write to /dev/tty".to_string()
}
