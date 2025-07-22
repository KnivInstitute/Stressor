use eframe::egui::{self, Color32, Stroke};
use sysinfo::{SystemExt, CpuExt};
use crate::app::SystemMonitorApp;

pub fn update_cpu_data(app: &mut SystemMonitorApp) {
    let now = std::time::Instant::now();
    if now.duration_since(app.last_update) >= std::time::Duration::from_millis(500) {
        app.sys.refresh_cpu();
        app.time_counter += 0.5;
        // Update CPU usage history
        let avg_cpu_usage = app.sys.cpus().iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .sum::<f64>() / app.sys.cpus().len() as f64;
        app.cpu_history.push_back((app.time_counter, avg_cpu_usage));
        if app.cpu_history.len() > 100 {
            app.cpu_history.pop_front();
        }
        // Update CPU frequency (platform-specific)
        #[cfg(windows)]
        {
            if let Some(ref wmi_con) = app.wmi_con {
                use serde::Deserialize;
                #[derive(Deserialize, Debug)]
                struct ProcessorInfo {
                    #[serde(rename = "CurrentClockSpeed")]
                    current_clock_speed: Option<u64>,
                    #[serde(rename = "MaxClockSpeed")]
                    max_clock_speed: Option<u64>,
                }
                let results: Result<Vec<ProcessorInfo>, _> = wmi_con.query();
                match results {
                    Ok(infos) => {
                        println!("[WMI] ProcessorInfo: {:?}", infos);
                        if let Some(info) = infos.get(0) {
                            if let Some(cur) = info.current_clock_speed {
                                app.current_cpu_freq = cur;
                            }
                            if let Some(max) = info.max_clock_speed {
                                app.max_cpu_freq = max;
                            }
                        }
                    },
                    Err(e) => {
                        println!("[WMI] Query error: {}", e);
                    }
                }
            }
        }
        #[cfg(not(windows))]
        {
            app.current_cpu_freq = app.sys.cpus().get(0)
                .map(|cpu| cpu.frequency())
                .unwrap_or(0);
        }
        app.last_update = now;
    }
}

pub fn ui_cpu_info(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    ui.heading("🖥️ System Information & Monitoring");
    ui.separator();
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                if let Some(cpu) = app.sys.cpus().get(0) {
                    ui.label(egui::RichText::new(format!("🧠 CPU: {}", cpu.brand())).strong());
                    ui.label(egui::RichText::new(format!(
                        "Cores: {} physical / {} logical",
                        app.sys.physical_core_count().unwrap_or(0),
                        app.sys.cpus().len()
                    )).color(egui::Color32::LIGHT_BLUE));
                }
                ui.add_space(10.0);
                // Always show frequency info as text
                if app.current_cpu_freq > 0 && app.max_cpu_freq > 0 {
                    ui.label(egui::RichText::new(format!(
                        "⏱️ CPU Frequency: {} MHz / {} MHz",
                        app.current_cpu_freq, app.max_cpu_freq
                    )).color(egui::Color32::YELLOW).strong());
                } else {
                    ui.colored_label(egui::Color32::RED, "CPU frequency unavailable");
                }
                // Only show the speedometer if clockspeed is updating
                if app.current_cpu_freq != app.max_cpu_freq && app.current_cpu_freq != 0 {
                    draw_cpu_speedometer(app, ui);
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                draw_simple_graph(app, ui, &app.cpu_history, Color32::LIGHT_BLUE, "CPU Usage History", 100.0);
            });
        });
    });
    ui.add_space(8.0);
    ui.separator();
}

pub fn draw_simple_graph(_app: &SystemMonitorApp, ui: &mut egui::Ui, data: &std::collections::VecDeque<(f64, f64)>, color: Color32, label: &str, max_val: f32) {
    ui.label(label);
    if data.is_empty() {
        ui.label("No data yet...");
        return;
    }
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(300.0, 100.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 5.0, Color32::from_gray(20));
        let stroke = Stroke::new(1.0, Color32::from_gray(60));
        painter.line_segment([rect.left_top(), rect.right_top()], stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
        painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
        painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
        for i in 1..4 {
            let y = rect.min.y + (rect.height() * i as f32 / 4.0);
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                Stroke::new(0.5, Color32::from_gray(40)),
            );
        }
        let points: Vec<egui::Pos2> = data.iter().enumerate().map(|(i, (_, value))| {
            let x = rect.min.x + (i as f32 / (data.len() - 1).max(1) as f32) * rect.width();
            let y = rect.max.y - (*value as f32 / max_val) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        }).collect();
        for i in 1..points.len() {
            painter.line_segment(
                [points[i-1], points[i]],
                Stroke::new(2.0, color),
            );
        }
        if let Some((_, current_value)) = data.back() {
            painter.text(
                egui::pos2(rect.max.x - 50.0, rect.min.y + 10.0),
                egui::Align2::LEFT_TOP,
                format!("{:.1}%", current_value),
                egui::FontId::proportional(12.0),
                color,
            );
        }
    }
}

pub fn draw_cpu_speedometer(app: &SystemMonitorApp, ui: &mut egui::Ui) {
    use std::f32::consts::PI;
    let size = egui::Vec2::new(80.0, 55.0);
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if !ui.is_rect_visible(rect) { return; }
    let painter = ui.painter();
    let center = egui::pos2(
        rect.center().x,
        rect.bottom() - 6.0,
    );
    let radius = 35.0;
    let thickness = 7.0;
    let start_angle = -2.0 * PI / 3.0;
    let end_angle = 2.0 * PI / 3.0;
    let sweep = end_angle - start_angle;
    let arc_segments = 60;
    draw_arc(painter, center, radius, start_angle, sweep, Stroke::new(thickness, Color32::from_gray(40)), arc_segments);
    let max = app.max_cpu_freq.max(1) as f32;
    let current = app.current_cpu_freq.min(app.max_cpu_freq) as f32;
    let percent = (current / max).clamp(0.0, 1.0);
    let filled_sweep = sweep * percent;
    let arc_color = if percent > 0.8 {
        Color32::RED
    } else if percent > 0.6 {
        Color32::YELLOW
    } else {
        Color32::LIGHT_BLUE
    };
    draw_arc(painter, center, radius, start_angle, filled_sweep, Stroke::new(thickness, arc_color), (arc_segments as f32 * percent.max(0.05)) as usize);
    let needle_angle = start_angle + filled_sweep;
    let needle_length = radius - thickness / 2.0;
    let needle_end = egui::pos2(
        center.x + needle_length * needle_angle.cos(),
        center.y + needle_length * needle_angle.sin(),
    );
    painter.line_segment(
        [center, needle_end],
        Stroke::new(3.0, Color32::WHITE),
    );
    for frac in [0.0, 0.5, 1.0] {
        let angle = start_angle + sweep * frac;
        let tick_start = egui::pos2(
            center.x + (radius - thickness) * angle.cos(),
            center.y + (radius - thickness) * angle.sin(),
        );
        let tick_end = egui::pos2(
            center.x + (radius + 2.0) * angle.cos(),
            center.y + (radius + 2.0) * angle.sin(),
        );
        painter.line_segment(
            [tick_start, tick_end],
            Stroke::new(2.0, Color32::GRAY),
        );
        let freq = (app.max_cpu_freq as f32 * frac).round() as u64;
        let label = if frac == 0.0 {
            "0"
        } else if frac == 1.0 {
            &format!("{}", app.max_cpu_freq)
        } else {
            &format!("{}", freq)
        };
        let label_pos = egui::pos2(
            center.x + (radius - thickness - 12.0) * angle.cos(),
            center.y + (radius - thickness - 12.0) * angle.sin(),
        );
        painter.text(
            label_pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            Color32::GRAY,
        );
    }
    let current_str = format!("Current: {} MHz", app.current_cpu_freq);
    let max_str = format!("Max: {} MHz", app.max_cpu_freq);
    let percent_str = format!("{:.1}%", percent * 100.0);
    let text_y = rect.bottom() - 2.0;
    painter.text(
        egui::pos2(center.x, text_y - 18.0),
        egui::Align2::CENTER_CENTER,
        percent_str,
        egui::FontId::proportional(14.0),
        arc_color,
    );
    painter.text(
        egui::pos2(center.x, text_y - 4.0),
        egui::Align2::CENTER_CENTER,
        current_str,
        egui::FontId::proportional(11.0),
        Color32::WHITE,
    );
    painter.text(
        egui::pos2(center.x, text_y + 8.0),
        egui::Align2::CENTER_CENTER,
        max_str,
        egui::FontId::proportional(9.0),
        Color32::GRAY,
    );
}

fn draw_arc(painter: &egui::Painter, center: egui::Pos2, radius: f32, start_angle: f32, sweep: f32, stroke: Stroke, segments: usize) {
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let angle = start_angle + sweep * t;
        let x = center.x + radius * angle.cos();
        let y = center.y + radius * angle.sin();
        points.push(egui::pos2(x, y));
    }
    for i in 1..points.len() {
        painter.line_segment([points[i - 1], points[i]], stroke);
    }
}
