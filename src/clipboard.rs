use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard unavailable: {}", e))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Failed to copy: {}", e))
}
