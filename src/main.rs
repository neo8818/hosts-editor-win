#![windows_subsystem = "windows"]

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use encoding_rs::GBK;

slint::include_modules!();

fn get_hosts_path() -> PathBuf {
    if let Some(system_root) = std::env::var("SystemRoot").ok() {
        PathBuf::from(system_root).join("System32\\drivers\\etc\\hosts")
    } else {
        PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts")
    }
}

fn read_hosts_file() -> Result<String, String> {
    let hosts_path = get_hosts_path();
    match fs::read(&hosts_path) {
        Ok(bytes) => {
            // Try UTF-8 first
            let content = if let Ok(s) = String::from_utf8(bytes.clone()) {
                s
            } else {
                // Fall back to GBK (Chinese Windows)
                let (s, _, _) = GBK.decode(&bytes);
                s.into_owned()
            };
            // Replace tabs with spaces for display
            Ok(content.replace('\t', "    "))
        }
        Err(e) => Err(format!("Cannot read hosts file: {}", e)),
    }
}

fn save_hosts_file(content: &str) -> Result<String, String> {
    let hosts_path = get_hosts_path();
    // Save as UTF-8 (modern standard)
    match fs::write(&hosts_path, content.as_bytes()) {
        Ok(_) => Ok("File saved successfully".to_string()),
        Err(e) => {
            let error_msg = if e.kind() == std::io::ErrorKind::PermissionDenied {
                "Permission denied! Please run as administrator.".to_string()
            } else {
                format!("Failed to save: {}", e)
            };
            Err(error_msg)
        }
    }
}

fn compute_diff_lines(original: &str, modified: &str) -> Vec<DiffLine> {
    use similar::{ChangeTag, TextDiff};
    
    if original == modified {
        return vec![DiffLine {
            text: SharedString::from("No changes detected."),
            line_type: 0,
        }];
    }
    
    let diff = TextDiff::from_lines(original, modified);
    let mut lines = Vec::new();
    
    for op in diff.ops() {
        for change in diff.iter_changes(op) {
            let line_type = match change.tag() {
                ChangeTag::Delete => 1,
                ChangeTag::Insert => 2,
                ChangeTag::Equal => 0,
            };
            
            let prefix = match change.tag() {
                ChangeTag::Delete => "- ",
                ChangeTag::Insert => "+ ",
                ChangeTag::Equal => "  ",
            };
            
            let text = format!("{}{}", prefix, change.value().trim_end());
            lines.push(DiffLine {
                text: SharedString::from(text),
                line_type,
            });
        }
    }
    
    lines
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    
    let original_content = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let original_for_refresh = original_content.clone();
    let original_for_save = original_content.clone();
    let original_for_confirm = original_content.clone();
    
    // Refresh
    let ui_handle = ui.as_weak();
    ui.on_refresh_clicked(move || {
        let ui = ui_handle.upgrade().unwrap();
        match read_hosts_file() {
            Ok(content) => {
                *original_for_refresh.lock().unwrap() = content.clone();
                ui.set_hosts_content(SharedString::from(&content));
                ui.set_status(SharedString::from(""));
            }
            Err(e) => {
                ui.set_status(SharedString::from(&e));
            }
        }
    });
    
    // Save button - show diff dialog
    let ui_handle = ui.as_weak();
    ui.on_save_clicked(move || {
        let ui = ui_handle.upgrade().unwrap();
        let current = ui.get_hosts_content().to_string();
        let original = original_for_save.lock().unwrap();
        
        let diff_lines = compute_diff_lines(&original, &current);
        let model = Rc::new(VecModel::from(diff_lines));
        ui.set_diff_lines(ModelRc::from(model));
        ui.set_show_confirm(true);
    });
    
    // Confirm save
    let ui_handle = ui.as_weak();
    ui.on_confirm_save(move || {
        let ui = ui_handle.upgrade().unwrap();
        let content = ui.get_hosts_content().to_string();
        
        match save_hosts_file(&content) {
            Ok(msg) => {
                *original_for_confirm.lock().unwrap() = content;
                ui.set_status(SharedString::from(&msg));
            }
            Err(e) => {
                ui.set_status(SharedString::from(&e));
            }
        }
        ui.set_show_confirm(false);
    });
    
    // Cancel save
    let ui_handle = ui.as_weak();
    ui.on_cancel_save(move || {
        let ui = ui_handle.upgrade().unwrap();
        ui.set_show_confirm(false);
    });
    
    // Initial load
    ui.set_hosts_path(SharedString::from(format!("Path: {}", get_hosts_path().display())));
    
    match read_hosts_file() {
        Ok(content) => {
            *original_content.lock().unwrap() = content.clone();
            ui.set_hosts_content(SharedString::from(&content));
        }
        Err(e) => {
            ui.set_status(SharedString::from(&e));
        }
    }
    
    ui.run()
}
