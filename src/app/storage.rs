use eframe::egui::{self, Color32};
use sysinfo::{SystemExt, DiskExt};
use crate::app::SystemMonitorApp;

pub fn update_storage_data(_app: &mut SystemMonitorApp) {
    // No-op for now, as storage is refreshed in update_system_data
}
pub fn ui_storage_info(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.label(egui::RichText::new("🗄️ Storage Information").strong());
        for disk in app.sys.disks() {
            let total_gb = disk.total_space() as f64 / 1e9;
            let available_gb = disk.available_space() as f64 / 1e9;
            let used_gb = total_gb - available_gb;
            let usage_percent = (used_gb / total_gb) * 100.0;

            ui.horizontal(|ui| {
                ui.label(format!("Drive: {}", disk.name().to_string_lossy()));
                ui.label(format!(
                    "Type: {:?}, File System: {}",
                    disk.kind(),
                    String::from_utf8_lossy(disk.file_system())
                ));
                ui.label("Connection: N/A");
            });
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Used: {:.2} GB / Total: {:.2} GB ({:.1}%)",
                    used_gb, total_gb, usage_percent
                ));
            });
            ui.add_space(4.0);
            draw_storage_visualizer(app, ui);
            ui.separator();
        }
    });
}

pub fn draw_storage_visualizer(app: &SystemMonitorApp, ui: &mut egui::Ui) {
    for disk in app.sys.disks() {
        let total_gb = disk.total_space() as f64 / 1e9;
        let available_gb = disk.available_space() as f64 / 1e9;
        let used_gb = total_gb - available_gb;
        let usage_percent = (used_gb / total_gb) * 100.0;
        ui.horizontal(|ui| {
            ui.label(format!("Drive {}:", disk.name().to_string_lossy()));
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(200.0, 20.0), egui::Sense::hover());
            if ui.is_rect_visible(rect) {
                let painter = ui.painter();
                painter.rect_filled(rect, 3.0, Color32::from_gray(40));
                let fill_width = rect.width() * (usage_percent / 100.0) as f32;
                let fill_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, rect.height()));
                let color = if usage_percent > 90.0 {
                    Color32::RED
                } else if usage_percent > 75.0 {
                    Color32::YELLOW
                } else {
                    Color32::GREEN
                };
                painter.rect_filled(fill_rect, 3.0, color);
            }
            ui.label(format!("{:.0} GB / {:.0} GB ({:.1}%)", used_gb, total_gb, usage_percent));
        });
    }
}
