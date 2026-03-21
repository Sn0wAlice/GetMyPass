use arboard::Clipboard;

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard unavailable: {}", e))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Failed to copy: {}", e))
}

pub fn clear_clipboard() -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard unavailable: {}", e))?;
    clipboard
        .set_text(String::new())
        .map_err(|e| format!("Failed to clear clipboard: {}", e))
}
