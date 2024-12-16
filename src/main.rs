use eframe::egui;
use std::process::Command;
use rfd::FileDialog;
use sysinfo::{System, SystemExt, DiskExt}; // Remove Disks import as it's not needed

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 400.0]), // Corrected window size setting
        ..Default::default()
    };
    
    eframe::run_native(
        "AyumiISO",
        options,
        Box::new(|_cc| Ok(Box::new(AyumiApp::default()))), // Corrected Box::new usage
    )
}

struct AyumiApp {
    iso_path: String,
    usb_drives: Vec<String>,
    selected_drive: Option<String>,
}

impl AyumiApp {
    fn get_usb_drives() -> Vec<String> {
        let mut system = System::new_all(); // Initialize system info
        system.refresh_disks(); // Use refresh_disks() method

        system
            .disks()
            .iter()
            .filter(|disk| {
                disk.is_removable()
                    && disk.mount_point().to_str().unwrap_or("").len() > 0
            })
            .map(|disk| {
                format!(
                    "{} ({}) - {:.1} GB",
                    disk.mount_point().to_str().unwrap_or("Unknown"),
                    disk.name().to_str().unwrap_or("USB"),
                    disk.total_space() as f64 / 1_000_000_000.0
                )
            })
            .collect()
    }
    fn burn_iso(&self) -> Result<(), String> {
        if self.iso_path.is_empty () {
            return Err("Please select an ISO file".to_string());
        }

        if self.selected_drive.is_none() {
            return Err("Please select a USB drive".to_string());
        }
                //Extract mount point
        let usb_path = self.selected_drive.as_ref().unwrap()
            .split("(")
            .next()
            .ok_or("Invaild USB drive selection")?;
        //confirm
        // Confirm burn
        let confirm = rfd::MessageDialog::new()
            .set_title("Confirm ISO Burn")
            .set_description(&format!(
                "Are you sure you want to burn\n{}\nto {}?", 
                self.iso_path, 
                usb_path
            ))
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

            if confirm == rfd::MessageDialogResult::Yes {
                // Use dd command to burn ISO (be VERY careful with this!)
                let output = Command::new("sudo")
                    .args(&[
                        "dd", 
                        "bs=4M", 
                        &format!("if={}", self.iso_path), 
                        &format!("of={}", usb_path), 
                        "status=progress"
                    ])
                    .output()
                    .map_err(|e| format!("Failed to execute dd: {}", e))?;
            
                if output.status.success() {
                    Ok(())
                } else {
                    Err(String::from_utf8_lossy(&output.stderr).to_string())
                }
            } else {
                Err("Burn cancelled by user".to_string())
            }
    }
}

impl Default for AyumiApp {
    fn default() -> Self {
        Self {
            iso_path: String::new(),
            usb_drives: Self::get_usb_drives(),
            selected_drive: None,
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

            // Burn ISO button
            if ui.button("🔥 Burn ISO").clicked() {
                match self.burn_iso() {
                    Ok(_) => {
                        // Show success popup
                        rfd::MessageDialog::new()
                            .set_title("Success")
                            .set_description("ISO burned successfully!")
                            .show();
                    },
                    Err(e) => {
                        // Show error popup
                        rfd::MessageDialog::new()
                            .set_title("Error")
                            .set_description(&e)
                            .show();
                    }
                }
            }
        });
    }
}
