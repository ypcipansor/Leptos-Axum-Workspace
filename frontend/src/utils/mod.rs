use web_sys::window;

pub fn get_local_storage_item(key: &str) -> Option<String> {
    window()?.local_storage().ok()??.get_item(key).ok()?
}

pub fn set_local_storage_item(key: &str, value: &str) {
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.set_item(key, value);
        }
    }
}

pub fn remove_local_storage_item(key: &str) {
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.local_storage() {
            let _ = storage.remove_item(key);
        }
    }
}
