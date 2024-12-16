use std::fs::File;
use std::io::{Read, Write};
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
    
            // Clone iso_path and selected_drive for use in the thread
            let iso_path = self.iso_path.clone();
            let selected_drive = self.selected_drive.clone();
    
            let progress = Arc::clone(&self.burning_progress);
            let is_burning = Arc::clone(&self.is_burning);
            let burn_error = Arc::clone(&self.burn_error);
    
            std::thread::spawn(move || {
                *is_burning.lock().unwrap() = true;
                *burn_error.lock().unwrap() = None;
    
                if let Err(e) = copy_with_progress(&std::path::Path::new(&iso_path), &destination, &progress) {
                    *burn_error.lock().unwrap() = Some(e);
                }
    
                *is_burning.lock().unwrap() = false;
            });
    
            Ok(())
        } else {
            Err("Please select a USB drive.".to_string())
        }
    }
}

fn copy_with_progress(
    source: &std::path::Path,
    destination: &std::path::Path,
    progress: &Arc<Mutex<f32>>,
) -> Result<(), String> {
    let mut src_file = File::open(source).map_err(|e| e.to_string())?;
    let mut dest_file = File::create(destination).map_err(|e| e.to_string())?;

    let total_size = src_file.metadata().map_err(|e| e.to_string())?.len();
    let mut buffer = vec![0; 8192];
    let mut copied = 0;

    loop {
        let bytes_read = src_file.read(&mut buffer).map_err(|e| e.to_string())?;
        if bytes_read == 0 {
            break;
        }

        dest_file
            .write_all(&buffer[..bytes_read])
            .map_err(|e| e.to_string())?;
        copied += bytes_read as u64;

        // Update progress
        let mut progress_lock = progress.lock().unwrap();
        *progress_lock = copied as f32 / total_size as f32;
    }

    Ok(())
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
                            .set_description("ISO copy started!")
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

            // Progress Bar
            let is_burning = *self.is_burning.lock().unwrap();
            if is_burning {
                let progress = *self.burning_progress.lock().unwrap();
                ui.add(egui::ProgressBar::new(progress).show_percentage());
            }

            // Display Errors
            if let Some(error) = self.burn_error.lock().unwrap().take() {
                rfd::MessageDialog::new()
                    .set_title("Error")
                    .set_description(&error)
                    .show();
            }

            // Request a repaint to update the UI
            if is_burning {
                ctx.request_repaint();
            }
        });
    }
}
