//! Putting text on the system clipboard. winit has no clipboard, so this
//! runs `xclip`, the standard X11 command line tool, and hands it the text.

use std::io::Write;
use std::process::{Command, Stdio};

/// Copies `text` to the clipboard.
///
/// # Errors
///
/// A message in the musician's terms when `xclip` is missing or fails.
pub fn copy(text: &str) -> Result<(), String> {
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                String::from("Cannot copy: xclip is not installed.")
            } else {
                format!("Cannot copy: xclip did not start ({error}).")
            }
        })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|error| format!("Cannot copy: xclip did not take the text ({error})."))?;
        // Dropping stdin closes it, which tells xclip the text is complete.
    }
    let status = child
        .wait()
        .map_err(|error| format!("Cannot copy: xclip did not finish ({error})."))?;
    if status.success() {
        Ok(())
    } else {
        Err(String::from("Cannot copy: xclip failed."))
    }
}
