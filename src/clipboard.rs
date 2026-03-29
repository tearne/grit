use std::io::Write;
use std::path::Path;

use base64::{engine::general_purpose::STANDARD, Engine};

/// Writes an `:open <path>:<line>` command to the clipboard via OSC 52.
///
/// OSC 52 instructs the terminal emulator to set the clipboard. The payload
/// must be standard base64-encoded. Most modern terminal emulators support it.
pub(crate) fn copy_open_command(path: &Path, line: u32) {
    let command = format!(":open {}:{}", path.display(), line);
    let encoded = STANDARD.encode(command.as_bytes());
    let sequence = format!("\x1b]52;c;{encoded}\x07");

    // Write directly to /dev/tty so the sequence reaches the terminal
    // regardless of whether stdout is redirected.
    if let Ok(mut tty) = std::fs::OpenOptions::new().write(true).open("/dev/tty") {
        let _ = tty.write_all(sequence.as_bytes());
    }
}
