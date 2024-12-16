use eframe::egui;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::path::Path;
use rfd::FileDialog;

#[derive(Clone, PartialEq)]
enum FlashMode {
    ManualCopy,
    Unsupported,
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "AyumiISO",
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
    flash_mode: FlashMode,
}

impl AyumiApp {
    fn get_usb_drives() -> Vec<String> {
        // For a standalone app, we'll use a simplified drive detection
        // This is a mock implementation - you'll want to replace with actual detection
        vec![
            "/media/user/USB1 (16GB)".to_string(),
            "/media/user/USB2 (32GB)".to_string(),
        ]
    }

    fn manual_copy_iso(&self) -> Result<(), String> {
        if self.iso_path.is_empty() {
            return Err("Please select an ISO file".to_string());
        }

        if self.selected_drive.is_none() {
            return Err("Please select a USB drive".to_string());
        }

        let iso_path = Path::new(&self.iso_path);
        let usb_path = Path::new(self.selected_drive.as_ref().unwrap());

        // Confirm burn
        let confirm = rfd::MessageDialog::new()
            .set_title("Confirm ISO Copy")
            .set_description(&format!(
                "Are you sure you want to copy\n{}\nto {}?", 
                self.iso_path, 
                usb_path.display()
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        if confirm == rfd::MessageDialogResult::Yes {
            // Prepare thread-safe progress tracking
            let progress = Arc::clone(&self.burning_progress);
            let is_burning = Arc::clone(&self.is_burning);
            let burn_error = Arc::clone(&self.burn_error);
            
            let iso_path = iso_path.to_path_buf();
            let usb_path = usb_path.to_path_buf();

            thread::spawn(move || {
                *is_burning.lock().unwrap() = true;
                *burn_error.lock().unwrap() = None;

                match std::fs::copy(&iso_path, &usb_path.join(iso_path.file_name().unwrap())) {
                    Ok(bytes_copied) => {
                        // Estimate progress based on file size
                        if let Ok(metadata) = std::fs::metadata(&iso_path) {
                            *progress.lock().unwrap() = 1.0;
                        }
                        *is_burning.lock().unwrap() = false;
                        Ok(())
                    },
                    Err(e) => {
                        *burn_error.lock().unwrap() = Some(format!("Copy failed: {}", e));
                        *is_burning.lock().unwrap() = false;
                        Err(e)
                    }
                };
            });

            Ok(())
        } else {
            Err("Copy cancelled by user".to_string())
        }
    }
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
            flash_mode: FlashMode::ManualCopy,
        }
    }
}

impl eframe::App for AyumiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🌸 AyumiISO 🌸");

            // ISO selection
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

            // Display file size
            if !self.iso_path.is_empty() {
                if let Ok(metadata) = std::fs::metadata(&self.iso_path) {
                    ui.label(format!(
                        "File Size: {:.2} MB",
                        metadata.len() as f64 / 1_048_576.0
                    ));
                }
            }

            // USB drive selection
            ui.separator();
            ui.heading("Select USB Drive");

            // Refresh USB drives
            if ui.button("🔄 Refresh USB Drives").clicked() {
                self.usb_drives = Self::get_usb_drives();
            }

            ui.horizontal_wrapped(|ui| {
                for drive in self.usb_drives.iter() {
                    let is_selected = self.selected_drive.as_ref() == Some(drive);

                    let response = ui.add(
                        egui::Button::new(format!("💾 {}", drive))
                            .fill(if is_selected {
                                egui::Color32::from_rgb(200, 230, 255)
                            } else {
                                egui::Color32::TRANSPARENT
                            }),
                    );

                    if response.clicked() {
                        self.selected_drive = Some(drive.clone());
                    }
                }
            });

            // Show selected USB
            if let Some(selected) = &self.selected_drive {
                ui.label(format!("Selected Drive: {}", selected));
            }

            // Show USB count
            ui.label(format!("USB Drives Found: {}", self.usb_drives.len()));

            // Burning progress and status
            let is_burning = *self.is_burning.lock().unwrap();
            let progress = *self.burning_progress.lock().unwrap();
            
            // Progress bar during burning
            if is_burning {
                ui.add(egui::ProgressBar::new(progress)
                    .show_percentage());
            }

            // Copy ISO button
            let copy_button = ui.button("📋 Copy ISO");
            
            // Handle copy button click
            if copy_button.clicked() {
                match self.manual_copy_iso() {
                    Ok(_) => {
                        rfd::MessageDialog::new()
                            .set_title("Success")
                            .set_description("ISO copied successfully!")
                            .show();
                    },
                    Err(e) => {
                        rfd::MessageDialog::new()
                            .set_title("Error")
                            .set_description(&e)
                            .show();
                    }
                }
            }

            // Check for burning errors
            if let Some(error) = self.burn_error.lock().unwrap().take() {
                rfd::MessageDialog::new()
                    .set_title("Copying Error")
                    .set_description(&error)
                    .show();
            }

            // Request a repaint to update progress
            if is_burning {
                ctx.request_repaint();
            }
        });
    }
}
