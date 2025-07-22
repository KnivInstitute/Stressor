use eframe::egui::{self, Color32};
use sysinfo::SystemExt;
use crate::app::SystemMonitorApp;

pub fn update_memory_data(app: &mut SystemMonitorApp) {
    app.sys.refresh_memory();
    let used = app.sys.used_memory() as f64;
    let total = app.sys.total_memory() as f64;
    let percent = (used / total) * 100.0;
    app.memory_history.push_back((app.time_counter, percent));
    if app.memory_history.len() > 100 {
        app.memory_history.pop_front();
    }
}

pub fn ui_memory_info(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("💾 Memory Usage").strong());
            draw_memory_bar(app, ui);
        });
        ui.add_space(6.0);
        super::cpu::draw_simple_graph(app, ui, &app.memory_history, Color32::LIGHT_GREEN, "Memory Usage History", 100.0);
    });
    ui.add_space(8.0);
    ui.separator();
}

pub fn draw_memory_bar(app: &SystemMonitorApp, ui: &mut egui::Ui) {
    let used_gb = app.sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let total_gb = app.sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let usage_percent = (used_gb / total_gb) * 100.0;
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(400.0, 30.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 5.0, Color32::from_gray(40));
        let fill_width = rect.width() * (usage_percent / 100.0) as f32;
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, rect.height()));
        let color = if usage_percent > 80.0 {
            Color32::RED
        } else if usage_percent > 60.0 {
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
