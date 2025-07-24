use eframe::egui;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use chrono::{NaiveDateTime, Datelike, Timelike};

#[derive(PartialEq, Eq)]
pub enum AnalyzerTab {
    StorageStress,
    // Add more analyzer types here
}

pub struct Analyzer {
    pub analyzer_tab: AnalyzerTab,
    pub selected_log_index: Option<usize>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self {
            analyzer_tab: AnalyzerTab::StorageStress,
            selected_log_index: None,
        }
    }
}

fn parse_storage_stress_filename(name: &str) -> Option<(String, String, String, Option<u32>, Option<u32>)> {
    // Format: storage_stress_<hash>_<YYYYMMDD_HHMMSS>.csv
    // Optionally: storage_stress_<hash>_<YYYYMMDD_HHMMSS>_buf<buf>_dur<dur>.csv
    let base = name.strip_prefix("storage_stress_")?.strip_suffix(".csv")?;
    let parts: Vec<&str> = base.split('_').collect();
    if parts.len() < 3 {
        return None;
    }
    let hash = parts[0];
    let date_str = parts[1..3].join("_");
    // Parse date
    let dt = NaiveDateTime::parse_from_str(&date_str, "%Y%m%d_%H%M%S").ok()?;
    let formatted = format!(
        "{} {}, {}: {:02}:{:02}:{:02}",
        dt.format("%B"),
        dt.day(),
        dt.year(),
        dt.hour(),
        dt.minute(),
        dt.second()
    );
    // Try to parse buffer size and duration if present
    let mut buffer_size = None;
    let mut duration = None;
    for i in 3..parts.len() {
        if let Some(buf) = parts[i].strip_prefix("buf") {
            buffer_size = buf.parse::<u32>().ok();
        }
        if let Some(dur) = parts[i].strip_prefix("dur") {
            duration = dur.parse::<u32>().ok();
        }
    }
    Some((formatted, date_str, hash.to_string(), buffer_size, duration))
}

fn analyze_storage_stress_csv(path: &str) -> Option<(Vec<f64>, Vec<f64>, f64, f64, f64, f64, f64, f64, f64, f64)> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut write_speeds = vec![];
    let mut read_speeds = vec![];
    for line in reader.lines().skip(1) { // skip header
        if let Ok(l) = line {
            let parts: Vec<&str> = l.split(',').collect();
            if parts.len() == 3 {
                let op = parts[1];
                if let Ok(mbps) = parts[2].parse::<f64>() {
                    if op == "write" {
                        write_speeds.push(mbps);
                    } else if op == "read" {
                        read_speeds.push(mbps);
                    }
                }
            }
        }
    }
    let avg_write = if !write_speeds.is_empty() { write_speeds.iter().sum::<f64>() / write_speeds.len() as f64 } else { 0.0 };
    let avg_read = if !read_speeds.is_empty() { read_speeds.iter().sum::<f64>() / read_speeds.len() as f64 } else { 0.0 };
    let max_write = write_speeds.iter().cloned().fold(0./0., f64::max);
    let max_read = read_speeds.iter().cloned().fold(0./0., f64::max);
    let min_write = write_speeds.iter().cloned().fold(0./0., f64::min);
    let min_read = read_speeds.iter().cloned().fold(0./0., f64::min);
    let std_write = if !write_speeds.is_empty() {
        let mean = avg_write;
        (write_speeds.iter().map(|v| (v-mean).powi(2)).sum::<f64>() / write_speeds.len() as f64).sqrt()
    } else { 0.0 };
    let std_read = if !read_speeds.is_empty() {
        let mean = avg_read;
        (read_speeds.iter().map(|v| (v-mean).powi(2)).sum::<f64>() / read_speeds.len() as f64).sqrt()
    } else { 0.0 };
    Some((write_speeds, read_speeds, avg_write, avg_read, max_write, max_read, min_write, min_read, std_write, std_read))
}

// fn draw_speed_graph(ui: &mut egui::Ui, data: &[f64], label: &str, color: egui::Color32) {
//     if data.is_empty() { return; }
//     let points: PlotPoints = data.iter().enumerate().map(|(i, v)| [i as f64, *v]).collect();
//     let [r, g, b, a] = color.to_array();
//     let line = Line::new(label, points).color(Color32::from_rgba_premultiplied(r, g, b, a));
//     Plot::new(label)
//         .height(120.0)
//         .legend(Legend::default())
//         .show(ui, |plot_ui| {
//             plot_ui.line(line);
//         });
// }

impl Analyzer {
    pub fn ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Analyzers");
        ui.separator();
        // Test type selection
        ui.horizontal(|ui| {
            if ui.selectable_label(self.analyzer_tab == AnalyzerTab::StorageStress, "Storage Stress").clicked() {
                self.analyzer_tab = AnalyzerTab::StorageStress;
                self.selected_log_index = None;
            }
            // Add more test types here
        });
        ui.separator();
        // List available logs for the selected test type
        let mut log_files = vec![];
        if let AnalyzerTab::StorageStress = self.analyzer_tab {
            if let Ok(entries) = fs::read_dir("log") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("storage_stress_") && name.ends_with(".csv") {
                            log_files.push(name.to_string());
                        }
                    }
                }
            }
            // Sort by newest first (lexicographically works for this filename format)
            log_files.sort_by(|a, b| b.cmp(a));
        }
        ui.label("Select a test to analyze:");
        egui::Grid::new("log_table").striped(true).show(ui, |ui| {
            ui.heading("Date & Time");
            ui.heading("Hash");
            ui.end_row();
            for (i, log) in log_files.iter().enumerate() {
                if let Some((formatted, _date_str, hash, _buf, _dur)) = parse_storage_stress_filename(log) {
                    let selected = self.selected_log_index == Some(i);
                    if ui.selectable_label(selected, &formatted).clicked() {
                        self.selected_log_index = Some(i);
                    }
                    ui.label(format!("({})", hash));
                    ui.end_row();
                }
            }
        });
        ui.separator();
        if let Some(idx) = self.selected_log_index {
            if let Some(log) = log_files.get(idx) {
                if let Some((formatted, _date_str, hash, buf, dur)) = parse_storage_stress_filename(log) {
                    ui.label(format!("Analyzing: {} ({})", formatted, hash));
                    ui.label(format!(
                        "Buffer Size: {} MB | Duration: {} s",
                        buf.map(|b| b.to_string()).unwrap_or_else(|| "Unknown".to_string()),
                        dur.map(|d| d.to_string()).unwrap_or_else(|| "Unknown".to_string())
                    ));
                }
                let path = format!("log/{}", log);
                if let Some((_write_speeds, _read_speeds, avg_write, avg_read, max_write, max_read, min_write, min_read, std_write, std_read)) = analyze_storage_stress_csv(&path) {
                    ui.label("Storage Stress Test Analysis:");
                    egui::Grid::new("analysis_table").striped(true).show(ui, |ui| {
                        ui.label("Avg Write MB/s"); ui.label(format!("{:.2}", avg_write)); ui.end_row();
                        ui.label("Avg Read MB/s"); ui.label(format!("{:.2}", avg_read)); ui.end_row();
                        ui.label("Max Write MB/s"); ui.label(format!("{:.2}", max_write)); ui.end_row();
                        ui.label("Max Read MB/s"); ui.label(format!("{:.2}", max_read)); ui.end_row();
                        ui.label("Min Write MB/s"); ui.label(format!("{:.2}", min_write)); ui.end_row();
                        ui.label("Min Read MB/s"); ui.label(format!("{:.2}", min_read)); ui.end_row();
                        ui.label("StdDev Write MB/s"); ui.label(format!("{:.2}", std_write)); ui.end_row();
                        ui.label("StdDev Read MB/s"); ui.label(format!("{:.2}", std_read)); ui.end_row();
                    });
                    ui.separator();
                    ui.label("Write Speed Graph:");
                    // draw_speed_graph(ui, &write_speeds, "Write MB/s", egui::Color32::LIGHT_BLUE);
                    ui.label("Read Speed Graph:");
                    // draw_speed_graph(ui, &read_speeds, "Read MB/s", egui::Color32::LIGHT_GREEN);
                    // TODO: Re-enable graphing when egui_plot version issues are resolved.
                } else {
                    ui.label("Failed to analyze log file.");
                }
            }
        }
    }
}
