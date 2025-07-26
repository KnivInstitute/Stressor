use eframe::egui::{self, Color32, Stroke};
use sysinfo::{SystemExt, CpuExt};
use crate::app::SystemMonitorApp;
use std::collections::VecDeque;
use log::{error, info};

pub struct History<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> History<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }
    pub fn push(&mut self, value: T) {
        if self.data.len() == self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(value);
    }
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

#[cfg(windows)]
fn try_read_cpu_temperature() -> Option<u32> {
    use std::ptr::null_mut;
    use winapi::um::fileapi::{CreateFileW, OPEN_EXISTING};
    use winapi::um::handleapi::CloseHandle;
    use winapi::um::winnt::{GENERIC_READ, GENERIC_WRITE, FILE_ATTRIBUTE_NORMAL};
    use winapi::um::ioapiset::DeviceIoControl;
    use winapi::shared::minwindef::{DWORD, ULONG, FALSE};
    use winapi::shared::ntdef::HANDLE;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    // These must match the driver
    const FILE_DEVICE_UNKNOWN: DWORD = 0x00000022;
    const METHOD_BUFFERED: DWORD = 0;
    const FILE_READ_DATA: DWORD = 0x0001;
    const FILE_WRITE_DATA: DWORD = 0x0002;
    const IOCTL_GET_CPU_TEMP: DWORD =
        FILE_DEVICE_UNKNOWN << 16 | (FILE_READ_DATA | FILE_WRITE_DATA) << 14 | 0x800 << 2 | METHOD_BUFFERED;

    let device_name: Vec<u16> = OsStr::new(r"\\.\CpuTempDrv").encode_wide().chain(Some(0)).collect();
    unsafe {
        let h_device: HANDLE = CreateFileW(
            device_name.as_ptr(),
            GENERIC_READ | GENERIC_WRITE,
            0,
            null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            null_mut(),
        );
        if h_device == winapi::um::handleapi::INVALID_HANDLE_VALUE {
            return None;
        }
        let mut temp: ULONG = 0;
        let mut bytes_returned: DWORD = 0;
        let success = DeviceIoControl(
            h_device,
            IOCTL_GET_CPU_TEMP,
            null_mut(),
            0,
            &mut temp as *mut ULONG as *mut _,
            std::mem::size_of::<ULONG>() as DWORD,
            &mut bytes_returned,
            null_mut(),
        );
        CloseHandle(h_device);
        if success == FALSE || bytes_returned < std::mem::size_of::<ULONG>() as u32 {
            return None;
        }
        Some(temp as u32)
    }
}

pub fn update_cpu_data(app: &mut SystemMonitorApp) {
    let now = std::time::Instant::now();
    if now.duration_since(app.last_update) >= std::time::Duration::from_millis(500) {
        app.sys.refresh_cpu();
        app.time_counter += 0.5;
        // Update CPU usage history
        let avg_cpu_usage = app.sys.cpus().iter()
            .map(|cpu| cpu.cpu_usage() as f64)
            .sum::<f64>() / app.sys.cpus().len() as f64;
        app.cpu_history.push((app.time_counter, avg_cpu_usage));
        // No need to pop_front, handled by History struct
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
                        info!("[WMI] ProcessorInfo: {:?}", infos);
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
                        error!("[WMI] Query error: {}", e);
                        app.last_error = Some(format!("WMI Query error: {}", e));
                    }
                }
            }
            // Try to get temperature from driver
            app.cpu_temperature_celsius = try_read_cpu_temperature();
            if app.cpu_temperature_celsius.is_none() {
            }
        }
        #[cfg(not(windows))]
        {
            app.current_cpu_freq = app.sys.cpus().get(0)
                .map(|cpu| cpu.frequency())
                .unwrap_or(0);
            app.cpu_temperature_celsius = None;
        }
        app.last_update = now;
    }
}

pub fn ui_cpu_info(app: &mut SystemMonitorApp, ui: &mut egui::Ui) {
    ui.heading("🖥️ System Information & Monitoring");
    ui.separator();
    if let Some(ref err) = app.last_error {
        ui.colored_label(egui::Color32::RED, err);
    }
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
                // Show temperature if available (Windows only)
                #[cfg(windows)]
                {
                    if let Some(temp) = app.cpu_temperature_celsius {
                        ui.label(egui::RichText::new(format!("🌡️ CPU Temperature: {} °C", temp)).color(egui::Color32::LIGHT_RED).strong());
                    } else {
                        ui.colored_label(egui::Color32::RED, "CPU temperature unavailable");
                    }
                }
                // Only show the speedometer if clockspeed is updating
                if app.current_cpu_freq != app.max_cpu_freq && app.current_cpu_freq != 0 {
                    draw_cpu_speedometer(app, ui);
                }
            });
            ui.separator();
            ui.vertical(|ui| {
                draw_simple_graph(ui, &app.cpu_history, Color32::LIGHT_BLUE, "CPU Usage History", 100.0);
            });
        });
    });
    ui.add_space(8.0);
    ui.separator();
}

// Move draw_simple_graph to a shared location (e.g., mod.rs or a new ui_utils.rs), but for now, make it generic and reusable for both CPU and memory.
pub fn draw_simple_graph<T: Copy + Into<f64>>(
    ui: &mut egui::Ui,
    data: &History<(f64, T)>,
    color: Color32,
    label: &str,
    max_val: f32,
) {
    ui.label(label);
    if data.iter().count() == 0 {
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
        let points: Vec<egui::Pos2> = data.iter().enumerate().map(|(i, &(_, value))| {
            let x = rect.min.x + (i as f32 / (data.iter().count() - 1).max(1) as f32) * rect.width();
            let y = rect.max.y - (value.into() as f32 / max_val) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        }).collect();
        for i in 1..points.len() {
            painter.line_segment(
                [points[i-1], points[i]],
                Stroke::new(2.0, color),
            );
        }
        if let Some(&(_, current_value)) = data.iter().last() {
            painter.text(
                egui::pos2(rect.max.x - 50.0, rect.min.y + 10.0),
                egui::Align2::LEFT_TOP,
                format!("{:.1}%", current_value.into()),
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
