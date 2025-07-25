use eframe::egui;
use std::fs::File;
use std::io::{BufRead, BufReader};
use chrono::{NaiveDateTime, Datelike, Timelike};

#[derive(PartialEq, Eq)]
pub enum AnalyzerTab {
    StorageStress,
    CpuStress,
    SelectableStress,
    // Add more analyzer types here
}

pub struct Analyzer {
    pub analyzer_tab: AnalyzerTab,
    pub selected_log_index: Option<usize>,
    pub dev_mode: bool,
    pub marked_for_delete: Option<(usize, std::time::Instant)>,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self {
            analyzer_tab: AnalyzerTab::StorageStress,
            selected_log_index: None,
            dev_mode: false,
            marked_for_delete: None,
        }
    }
}

impl Analyzer {
    pub fn with_dev_mode(dev_mode: bool) -> Self {
        Self {
            analyzer_tab: AnalyzerTab::StorageStress,
            selected_log_index: None,
            dev_mode,
            marked_for_delete: None,
        }
    }

    pub fn log_dir(&self) -> std::path::PathBuf {
        if self.dev_mode {
            std::path::PathBuf::from("log")
        } else {
            std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("log")
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

fn parse_cpu_stress_filename(name: &str) -> Option<(String, String, String, Option<u32>, Option<u32>)> {
    // Format: cpu_stress_<hash>_<YYYYMMDD_HHMMSS>_int<intensity>_dur<duration>.csv
    let base = name.strip_prefix("cpu_stress_")?.strip_suffix(".csv")?;
    let parts: Vec<&str> = base.split('_').collect();
    if parts.len() < 3 {
        return None;
    }
    let hash = parts[0];
    let date_str = parts[1..3].join("_");
    // Parse date
    let dt = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y%m%d_%H%M%S").ok()?;
    let formatted = format!(
        "{} {}, {}: {:02}:{:02}:{:02}",
        dt.format("%B"),
        dt.day(),
        dt.year(),
        dt.hour(),
        dt.minute(),
        dt.second()
    );
    // Try to parse intensity and duration if present
    let mut intensity = None;
    let mut duration = None;
    for i in 3..parts.len() {
        if let Some(ints) = parts[i].strip_prefix("int") {
            intensity = ints.parse::<u32>().ok();
        }
        if let Some(dur) = parts[i].strip_prefix("dur") {
            duration = dur.parse::<u32>().ok();
        }
    }
    Some((formatted, date_str, hash.to_string(), intensity, duration))
}

fn analyze_cpu_stress_csv(path: &str) -> Option<(Vec<Vec<f64>>, Vec<f64>, f64, f64, f64, f64)> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut thread_rates: Vec<Vec<f64>> = vec![];
    let mut max_tid = 0;
    for line in reader.lines().skip(1) { // skip header
        if let Ok(l) = line {
            let parts: Vec<&str> = l.split(',').collect();
            if parts.len() == 3 {
                if let (Ok(tid), Ok(rate)) = (parts[1].parse::<usize>(), parts[2].parse::<f64>()) {
                    if tid >= thread_rates.len() {
                        thread_rates.resize(tid + 1, vec![]);
                    }
                    thread_rates[tid].push(rate);
                    if tid > max_tid { max_tid = tid; }
                }
            }
        }
    }
    // Compute per-thread avg, and overall stats
    let mut all_rates = vec![];
    for rates in &thread_rates {
        all_rates.extend(rates);
    }
    let avg = if !all_rates.is_empty() { all_rates.iter().sum::<f64>() / all_rates.len() as f64 } else { 0.0 };
    let max = all_rates.iter().cloned().fold(0./0., f64::max);
    let min = all_rates.iter().cloned().fold(0./0., f64::min);
    let stddev = if !all_rates.is_empty() {
        let mean = avg;
        (all_rates.iter().map(|v| (v-mean).powi(2)).sum::<f64>() / all_rates.len() as f64).sqrt()
    } else { 0.0 };
    Some((thread_rates, all_rates, avg, max, min, stddev))
}

fn parse_selectable_stress_filename(name: &str) -> Option<(String, String, String, Option<u32>, Option<u32>)> {
    // Format: selectable_<type>_<YYYYMMDD_HHMMSS>_params.csv
    let base = name.strip_prefix("selectable_")?.strip_suffix(".csv")?;
    let parts: Vec<&str> = base.split('_').collect();
    if parts.len() < 3 {
        return None;
    }
    let typ = parts[0];
    let date_str = parts[1..3].join("_");
    let _params = parts[3..].join("_");
    let dt = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y%m%d_%H%M%S").ok()?;
    let formatted = format!(
        "{} {}, {}: {:02}:{:02}:{:02}",
        dt.format("%B"),
        dt.day(),
        dt.year(),
        dt.hour(),
        dt.minute(),
        dt.second()
    );
    Some((formatted, date_str, typ.to_string(), None, None))
}

fn analyze_selectable_stress_csv(path: &str) -> Option<(String, String, u64, Vec<(usize, u64)>)> {
    let file = std::fs::File::open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    let mut workload = String::new();
    let mut params = String::new();
    let mut thread_ops = vec![];
    for (i, line) in reader.lines().enumerate() {
        let l = line.ok()?;
        if i == 0 { continue; } // skip header
        let parts: Vec<&str> = l.split(',').collect();
        if parts.len() >= 6 {
            workload = parts[1].to_string();
            params = parts[2].to_string();
            let tid = parts[4].parse().unwrap_or(0);
            let ops = parts[5].parse().unwrap_or(0);
            thread_ops.push((tid, ops));
        }
    }
    let total_ops = thread_ops.iter().map(|&(_, ops)| ops).sum();
    Some((workload, params, total_ops, thread_ops))
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
                self.marked_for_delete = None;
            }
            if ui.selectable_label(self.analyzer_tab == AnalyzerTab::CpuStress, "CPU Stress").clicked() {
                self.analyzer_tab = AnalyzerTab::CpuStress;
                self.selected_log_index = None;
                self.marked_for_delete = None;
            }
            if ui.selectable_label(self.analyzer_tab == AnalyzerTab::SelectableStress, "Selectable Stress").clicked() {
                self.analyzer_tab = AnalyzerTab::SelectableStress;
                self.selected_log_index = None;
                self.marked_for_delete = None;
            }
        });
        ui.separator();
        // List available logs for the selected test type
        let log_dir = self.log_dir();
        let mut log_files = vec![];
        match self.analyzer_tab {
            AnalyzerTab::StorageStress => {
                if let Ok(entries) = std::fs::read_dir(&log_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("storage_stress_") && name.ends_with(".csv") {
                                log_files.push(name.to_string());
                            }
                        }
                    }
                }
                log_files.sort_by(|a, b| {
                    let adt = parse_storage_stress_filename(a).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    let bdt = parse_storage_stress_filename(b).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    bdt.cmp(&adt)
                });
            },
            AnalyzerTab::CpuStress => {
                if let Ok(entries) = std::fs::read_dir(&log_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("cpu_stress_") && name.ends_with(".csv") {
                                log_files.push(name.to_string());
                            }
                        }
                    }
                }
                log_files.sort_by(|a, b| {
                    let adt = parse_cpu_stress_filename(a).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    let bdt = parse_cpu_stress_filename(b).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    bdt.cmp(&adt)
                });
            },
            AnalyzerTab::SelectableStress => {
                if let Ok(entries) = std::fs::read_dir(&log_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            if name.starts_with("selectable_") && name.ends_with(".csv") {
                                log_files.push(name.to_string());
                            }
                        }
                    }
                }
                log_files.sort_by(|a, b| {
                    let adt = parse_selectable_stress_filename(a).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    let bdt = parse_selectable_stress_filename(b).and_then(|(_, date, _, _, _)| chrono::NaiveDateTime::parse_from_str(&date, "%Y%m%d_%H%M%S").ok());
                    bdt.cmp(&adt)
                });
            },
        }
        ui.label("Select a test to analyze:");
        egui::Grid::new("log_table").striped(true).show(ui, |ui| {
            ui.heading("Date & Time");
            ui.heading("Type/Hash");
            ui.end_row();
            let mut to_delete: Option<usize> = None;
            let now = std::time::Instant::now();
            if let Some((idx, mark_time)) = self.marked_for_delete {
                if now.duration_since(mark_time) > std::time::Duration::from_secs(10) {
                    if self.dev_mode {
                        println!("[DEV] Expired delete mark for log index {} after 10s", idx);
                    }
                    self.marked_for_delete = None;
                }
            }
            for (i, log) in log_files.iter().enumerate() {
                let parsed = match self.analyzer_tab {
                    AnalyzerTab::StorageStress => parse_storage_stress_filename(log),
                    AnalyzerTab::CpuStress => parse_cpu_stress_filename(log),
                    AnalyzerTab::SelectableStress => parse_selectable_stress_filename(log),
                };
                if let Some((formatted, _date_str, typ, _, _)) = parsed {
                    let _selected = self.selected_log_index == Some(i);
                    let marked = self.marked_for_delete.map(|(idx, _)| idx) == Some(i);
                    if marked {
                        egui::Frame::NONE
                            .fill(egui::Color32::RED)
                            .show(ui, |ui| {
                                let response = ui.add_sized([
                                    ui.available_width() * 0.6,
                                    24.0
                                ], egui::Label::new(&formatted).sense(egui::Sense::click_and_drag()));
                                if response.clicked_by(egui::PointerButton::Primary) {
                                    if self.dev_mode {
                                        println!("[DEV] Selected log index {} (left click) while marked for delete", i);
                                    }
                                    self.selected_log_index = Some(i);
                                    self.marked_for_delete = None;
                                } else if response.clicked_by(egui::PointerButton::Secondary) {
                                    if self.dev_mode {
                                        println!("[DEV] Deleting log index {}: {}", i, log);
                                    }
                                    let path = log_dir.join(log);
                                    let _ = std::fs::remove_file(&path);
                                    to_delete = Some(i);
                                    self.marked_for_delete = None;
                                    if self.selected_log_index == Some(i) {
                                        self.selected_log_index = None;
                                    }
                                }
                                ui.label(format!("({})", typ));
                            });
                        ui.end_row();
                    } else {
                        let response = ui.add_sized([
                            ui.available_width() * 0.6,
                            24.0
                        ], egui::Label::new(&formatted)
                            .sense(egui::Sense::click_and_drag()));
                        if response.clicked_by(egui::PointerButton::Primary) {
                            if self.dev_mode {
                                println!("[DEV] Selected log index {} (left click)", i);
                            }
                            self.selected_log_index = Some(i);
                            self.marked_for_delete = None;
                        } else if response.clicked_by(egui::PointerButton::Secondary) {
                            if self.dev_mode {
                                println!("[DEV] Marked log index {} for delete (right click)", i);
                            }
                            self.marked_for_delete = Some((i, std::time::Instant::now()));
                        }
                        ui.label(format!("({})", typ));
                        ui.end_row();
                    }
                }
            }
            if let Some(idx) = to_delete {
                if self.dev_mode {
                    println!("[DEV] Removed log index {} from list after deletion", idx);
                }
                log_files.remove(idx);
            }
        });
        ui.separator();
        if let Some(idx) = self.selected_log_index {
            if let Some(log) = log_files.get(idx) {
                match self.analyzer_tab {
                    AnalyzerTab::StorageStress => {
                        if let Some((formatted, _date_str, hash, buf, dur)) = parse_storage_stress_filename(log) {
                            ui.label(format!("Analyzing: {} ({})", formatted, hash));
                            ui.label(format!(
                                "Buffer Size: {} MB | Duration: {} s",
                                buf.map(|b| b.to_string()).unwrap_or_else(|| "Unknown".to_string()),
                                dur.map(|d| d.to_string()).unwrap_or_else(|| "Unknown".to_string())
                            ));
                        }
                        let path = format!("{}/{}", log_dir.to_string_lossy(), log);
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
                    },
                    AnalyzerTab::CpuStress => {
                        if let Some((formatted, _date_str, hash, intensity, dur)) = parse_cpu_stress_filename(log) {
                            ui.label(format!("Analyzing: {} ({})", formatted, hash));
                            ui.label(format!(
                                "Intensity: {} | Duration: {} s",
                                intensity.map(|b| b.to_string()).unwrap_or_else(|| "Unknown".to_string()),
                                dur.map(|d| d.to_string()).unwrap_or_else(|| "Unknown".to_string())
                            ));
                        }
                        let path = format!("{}/{}", log_dir.to_string_lossy(), log);
                        if let Some((thread_rates, _all_rates, avg, max, min, stddev)) = analyze_cpu_stress_csv(&path) {
                            ui.label("CPU Stress Test Analysis:");
                            egui::Grid::new("cpu_analysis_table").striped(true).show(ui, |ui| {
                                ui.label("Avg Iter/s"); ui.label(format!("{:.2}", avg)); ui.end_row();
                                ui.label("Max Iter/s"); ui.label(format!("{:.2}", max)); ui.end_row();
                                ui.label("Min Iter/s"); ui.label(format!("{:.2}", min)); ui.end_row();
                                ui.label("StdDev Iter/s"); ui.label(format!("{:.2}", stddev)); ui.end_row();
                            });
                            ui.separator();
                            ui.label("Per-thread stats:");
                            egui::Grid::new("cpu_thread_table").striped(true).show(ui, |ui| {
                                ui.label("Thread"); ui.label("Avg"); ui.label("Max"); ui.label("Min"); ui.label("StdDev"); ui.end_row();
                                for (tid, rates) in thread_rates.iter().enumerate() {
                                    if rates.is_empty() { continue; }
                                    let avg = rates.iter().sum::<f64>() / rates.len() as f64;
                                    let max = rates.iter().cloned().fold(0./0., f64::max);
                                    let min = rates.iter().cloned().fold(0./0., f64::min);
                                    let stddev = if !rates.is_empty() {
                                        let mean = avg;
                                        (rates.iter().map(|v| (v-mean).powi(2)).sum::<f64>() / rates.len() as f64).sqrt()
                                    } else { 0.0 };
                                    ui.label(format!("{}", tid));
                                    ui.label(format!("{:.2}", avg));
                                    ui.label(format!("{:.2}", max));
                                    ui.label(format!("{:.2}", min));
                                    ui.label(format!("{:.2}", stddev));
                                    ui.end_row();
                                }
                            });
                        } else {
                            ui.label("Failed to analyze log file.");
                        }
                    },
                    AnalyzerTab::SelectableStress => {
                        if let Some((workload, params, total_ops, thread_ops)) = analyze_selectable_stress_csv(&format!("{}/{}", log_dir.to_string_lossy(), log)) {
                            ui.label(format!("Workload: {}", workload));
                            ui.label(format!("Params: {}", params));
                            ui.label(format!("Total Operations: {}", total_ops));
                            egui::Grid::new("selectable_thread_table").striped(true).show(ui, |ui| {
                                ui.label("Thread"); ui.label("Ops"); ui.end_row();
                                for (tid, ops) in thread_ops {
                                    ui.label(format!("{}", tid));
                                    ui.label(format!("{}", ops));
                                    ui.end_row();
                                }
                            });
                        } else {
                            ui.label("Failed to analyze log file.");
                        }
                    },
                }
            }
        }
    }
}
