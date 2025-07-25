use eframe::egui::{self, Color32};
use sysinfo::SystemExt;
use crate::app::SystemMonitorApp;
use crate::app::config::Config;
// use crate::app::cpu::History; // Only if needed

pub fn update_memory_data(app: &mut SystemMonitorApp) {
    app.sys.refresh_memory();
    let used = app.sys.used_memory() as f64;
    let total = app.sys.total_memory() as f64;
    if total == 0.0 {
        app.last_error = Some("Memory information unavailable".to_string());
        return;
    }
    let percent = (used / total) * 100.0;
    app.memory_history.push((app.time_counter, percent));
}

pub fn ui_memory_info(app: &mut SystemMonitorApp, ui: &mut egui::Ui, config: &Config) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        if let Some(ref err) = app.last_error {
            ui.colored_label(egui::Color32::RED, err);
        }
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("💾 Memory Usage").strong());
            draw_memory_bar(app, ui, config);
        });
        ui.add_space(6.0);
        super::cpu::draw_simple_graph(ui, &app.memory_history, Color32::LIGHT_GREEN, "Memory Usage History", 100.0);
    });
    ui.add_space(8.0);
    ui.separator();
}

pub fn draw_memory_bar(app: &SystemMonitorApp, ui: &mut egui::Ui, config: &Config) {
    let used_gb = app.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_gb = app.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let usage_percent = (used_gb / total_gb) * 100.0;
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(config.memory_bar_width, config.memory_bar_height), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 5.0, Color32::from_gray(40));
        let fill_width = rect.width() * (usage_percent / 100.0) as f32;
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, rect.height()));
        let color = if usage_percent > config.memory_warn_threshold {
            Color32::RED
        } else if usage_percent > config.memory_caution_threshold {
            Color32::YELLOW
        } else {
            Color32::GREEN
        };
        painter.rect_filled(fill_rect, 5.0, color);
        let text = format!("{:.1} GB / {:.1} GB ({:.1}%)", used_gb, total_gb, usage_percent);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }
}
