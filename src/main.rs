use std::process::Command;
use std::sync::{Arc, Mutex};
use eframe::egui;
use rfd::FileDialog;
use winapi::um::fileapi::{GetLogicalDrives, GetDriveTypeA};
use std::ffi::CStr;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AyumiISO for Windows",
        options,
        Box::new(|_cc| Ok(Box::new(AyumiApp::default()))),
    )
}

struct AyumiApp {
    iso_path: String,
    usb_drives: Vec<String>,
    selected_drive: Option<String>,
    burning_progress: Arc<Mutex<f32>>,
    is_burning: Arc<Mutex<bool>>,
    burn_error: Arc<Mutex<Option<String>>>,
}

impl Default for AyumiApp {
    fn default() -> Self {
        Self {
            iso_path: String::new(),
            usb_drives: Self::get_usb_drives(),
            selected_drive: None,
            burning_progress: Arc::new(Mutex::new(0.0)),
            is_burning: Arc::new(Mutex::new(false)),
            burn_error: Arc::new(Mutex::new(None)),
        }
    }
}

impl AyumiApp {
    fn get_usb_drives() -> Vec<String> {
        let mut drives = Vec::new();
        let drive_bits = unsafe { GetLogicalDrives() };
    
        for i in 0..26 {
            if drive_bits & (1 << i) != 0 {
                let drive_letter = format!("{}:\\", (b'A' + i as u8) as char);
                let drive_type = unsafe { GetDriveTypeA(drive_letter.as_ptr() as *const i8) };
    
                // Check if it's a removable drive
                if drive_type == 2 {
                    drives.push(drive_letter);
                }
            }
        }
    
        drives
    }    

    fn copy_iso(&self) -> Result<(), String> {
        if self.iso_path.is_empty() {
            return Err("Please select an ISO file.".to_string());
        }

        if let Some(drive) = &self.selected_drive {
            let drive_path = drive.split(' ').next().unwrap_or("");
            let source = std::path::Path::new(&self.iso_path);
            let destination = std::path::Path::new(drive_path).join(source.file_name().unwrap());

            std::fs::copy(source, destination).map_err(|e| e.to_string())?;
            Ok(())
        } else {
            Err("Please select a USB drive.".to_string())
        }
    }
}

impl eframe::App for AyumiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🌸 AyumiISO for Windows 🌸");

            // ISO Selection
            ui.horizontal(|ui| {
                ui.label("ISO File:");
                ui.text_edit_singleline(&mut self.iso_path);

                if ui.button("Browse").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("ISO Files", &["iso"])
                        .pick_file()
                    {
                        self.iso_path = path.display().to_string();
                    }
                }
            });

            // USB Drive Detection
            ui.separator();
            ui.heading("Available USB Drives:");

            if ui.button("🔄 Refresh USB Drives").clicked() {
                self.usb_drives = Self::get_usb_drives();
            }

            for drive in &self.usb_drives {
                let is_selected = self.selected_drive.as_ref() == Some(drive);
                let response = ui.add(egui::SelectableLabel::new(is_selected, drive));

                if response.clicked() {
                    self.selected_drive = Some(drive.clone());
                }
            }

            if let Some(drive) = &self.selected_drive {
                ui.label(format!("Selected Drive: {}", drive));
            }

            // Copy ISO
            if ui.button("📋 Copy ISO").clicked() {
                match self.copy_iso() {
                    Ok(_) => {
                        rfd::MessageDialog::new()
                            .set_title("Success")
                            .set_description("ISO copied successfully!")
                            .show();
                    }
                    Err(e) => {
                        rfd::MessageDialog::new()
                            .set_title("Error")
                            .set_description(&e)
                            .show();
                    }
                }
            }

            // Status Update
            if *self.is_burning.lock().unwrap() {
                ui.add(egui::ProgressBar::new(*self.burning_progress.lock().unwrap())
                    .show_percentage());
            }
        });
    }
}
