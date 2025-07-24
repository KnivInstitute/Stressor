use eframe::egui;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use atomic_float::AtomicF64;
use rand::{thread_rng, Rng};
use chrono::Local;

pub struct StorageStress {
    running: Arc<AtomicBool>,
    write_speeds: Arc<Mutex<Vec<f64>>>,
    read_speeds: Arc<Mutex<Vec<f64>>>,
    current_write_speed: Arc<AtomicF64>,
    current_read_speed: Arc<AtomicF64>,
    log_path: Arc<Mutex<Option<PathBuf>>>,
    duration_secs: Arc<Mutex<u32>>,
    avg_write: Arc<AtomicF64>,
    avg_read: Arc<AtomicF64>,
    buffer_mb: Arc<Mutex<u32>>,
}

impl Default for StorageStress {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            write_speeds: Arc::new(Mutex::new(Vec::new())),
            read_speeds: Arc::new(Mutex::new(Vec::new())),
            current_write_speed: Arc::new(AtomicF64::new(0.0)),
            current_read_speed: Arc::new(AtomicF64::new(0.0)),
            log_path: Arc::new(Mutex::new(None)),
            duration_secs: Arc::new(Mutex::new(20)),
            avg_write: Arc::new(AtomicF64::new(0.0)),
            avg_read: Arc::new(AtomicF64::new(0.0)),
            buffer_mb: Arc::new(Mutex::new(8)),
        }
    }
}

impl StorageStress {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Storage Stress Test");
        ui.add_space(10.0);
        ui.label("This test will write and read a large file to measure disk throughput (MB/s).");
        ui.add_space(10.0);
        let mut duration = *self.duration_secs.lock().unwrap();
        let mut buffer_mb = *self.buffer_mb.lock().unwrap();
        ui.horizontal(|ui| {
            ui.label("Duration (seconds):");
            if ui.add(egui::DragValue::new(&mut duration).range(5..=300)).changed() {
                *self.duration_secs.lock().unwrap() = duration;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Chunk Size (MB):");
            if ui.add(egui::DragValue::new(&mut buffer_mb).range(1..=128)).changed() {
                *self.buffer_mb.lock().unwrap() = buffer_mb;
            }
        });
        ui.add_space(10.0);
        if ui.button(if self.running.load(Ordering::SeqCst) { "Stop Storage Stress" } else { "Start Storage Stress" }).clicked() {
            let running = self.running.clone();
            let write_speeds = self.write_speeds.clone();
            let read_speeds = self.read_speeds.clone();
            let current_write_speed = self.current_write_speed.clone();
            let current_read_speed = self.current_read_speed.clone();
            let avg_write = self.avg_write.clone();
            let avg_read = self.avg_read.clone();
            let log_path = self.log_path.clone();
            let duration_secs = *self.duration_secs.lock().unwrap();
            let buffer_mb = *self.buffer_mb.lock().unwrap();
            let ctx = ctx.clone();
            if running.load(Ordering::SeqCst) {
                running.store(false, Ordering::SeqCst);
            } else {
                running.store(true, Ordering::SeqCst);
                write_speeds.lock().unwrap().clear();
                read_speeds.lock().unwrap().clear();
                current_write_speed.store(0.0, Ordering::SeqCst);
                current_read_speed.store(0.0, Ordering::SeqCst);
                avg_write.store(0.0, Ordering::SeqCst);
                avg_read.store(0.0, Ordering::SeqCst);
                thread::spawn(move || {
                    let mut rng = thread_rng();
                    let hash: u16 = rng.gen_range(1000..9999);
                    let date = Local::now().format("%Y%m%d_%H%M%S");
                    let log_file_name = format!("log/storage_stress_{}_{}_buf{}_dur{}.csv", hash, date, buffer_mb, duration_secs);
                    let mut log_file = OpenOptions::new().create(true).append(true).open(&log_file_name).unwrap();
                    {
                        let mut log_path_guard = log_path.lock().unwrap();
                        *log_path_guard = Some(PathBuf::from(&log_file_name));
                    }
                    writeln!(log_file, "timestamp,operation,mbps").unwrap();
                    let test_file_path = "storage_stress_testfile.tmp";
                    let file_size_mb = 512; // 512 MB
                    let buffer_size = buffer_mb as usize * 1024 * 1024;
                    let mut buffer = vec![0u8; buffer_size];
                    rng.fill(&mut buffer[..]);
                    // Write test
                    let write_end = Instant::now() + Duration::from_secs((duration_secs / 2).max(1) as u64);
                    let mut file = File::create(test_file_path).unwrap();
                    let mut written = 0;
                    let mut total_written = 0;
                    let mut last_report = Instant::now();
                    let mut last_written = 0;
                    let write_start = Instant::now();
                    while Instant::now() < write_end && running.load(Ordering::SeqCst) {
                        let to_write = std::cmp::min(buffer_size, file_size_mb * 1024 * 1024 - written);
                        file.write_all(&buffer[..to_write]).unwrap();
                        file.flush().unwrap();
                        written += to_write;
                        total_written += to_write;
                        if written >= file_size_mb * 1024 * 1024 {
                            // Overwrite from start
                            file.seek(SeekFrom::Start(0)).unwrap();
                            written = 0;
                            last_written = 0;
                        }
                        if last_report.elapsed() >= Duration::from_millis(200) {
                            let elapsed = last_report.elapsed().as_secs_f64();
                            let mbps = (written - last_written) as f64 / 1024.0 / 1024.0 / elapsed;
                            current_write_speed.store(mbps, Ordering::SeqCst);
                            write_speeds.lock().unwrap().push(mbps);
                            writeln!(log_file, "{},{},{}", Local::now().to_rfc3339(), "write", mbps).unwrap();
                            last_report = Instant::now();
                            last_written = written;
                            ctx.request_repaint();
                        }
                    }
                    let write_total_time = write_start.elapsed().as_secs_f64();
                    let avg_write_val = total_written as f64 / 1024.0 / 1024.0 / write_total_time;
                    avg_write.store(avg_write_val, Ordering::SeqCst);
                    file.sync_all().unwrap();
                    // Read test
                    let read_end = Instant::now() + Duration::from_secs((duration_secs / 2).max(1) as u64);
                    let mut file = File::open(test_file_path).unwrap();
                    file.seek(SeekFrom::Start(0)).unwrap();
                    let mut read = 0;
                    let mut total_read = 0;
                    let mut last_report = Instant::now();
                    let mut last_read = 0;
                    let read_start = Instant::now();
                    while Instant::now() < read_end && running.load(Ordering::SeqCst) {
                        let to_read = std::cmp::min(buffer_size, file_size_mb * 1024 * 1024 - read);
                        file.read_exact(&mut buffer[..to_read]).unwrap();
                        read += to_read;
                        total_read += to_read;
                        if read >= file_size_mb * 1024 * 1024 {
                            // Re-read from start
                            file.seek(SeekFrom::Start(0)).unwrap();
                            read = 0;
                            last_read = 0;
                        }
                        if last_report.elapsed() >= Duration::from_millis(200) {
                            let elapsed = last_report.elapsed().as_secs_f64();
                            let mbps = (read - last_read) as f64 / 1024.0 / 1024.0 / elapsed;
                            current_read_speed.store(mbps, Ordering::SeqCst);
                            read_speeds.lock().unwrap().push(mbps);
                            writeln!(log_file, "{},{},{}", Local::now().to_rfc3339(), "read", mbps).unwrap();
                            last_report = Instant::now();
                            last_read = read;
                            ctx.request_repaint();
                        }
                    }
                    let read_total_time = read_start.elapsed().as_secs_f64();
                    let avg_read_val = total_read as f64 / 1024.0 / 1024.0 / read_total_time;
                    avg_read.store(avg_read_val, Ordering::SeqCst);
                    std::fs::remove_file(test_file_path).ok();
                    running.store(false, Ordering::SeqCst);
                    ctx.request_repaint();
                });
            }
        }
        // Visualizer and stats
        let live_write = self.current_write_speed.load(Ordering::SeqCst);
        let live_read = self.current_read_speed.load(Ordering::SeqCst);
        let avg_write = self.avg_write.load(Ordering::SeqCst);
        let avg_read = self.avg_read.load(Ordering::SeqCst);
        ui.add_space(10.0);
        ui.label(format!("Write: Avg {:.2} MB/s | Live {:.2} MB/s", avg_write, live_write));
        draw_speed_bar(ui, live_write, "Write Speed");
        ui.label(format!("Read:  Avg {:.2} MB/s | Live {:.2} MB/s", avg_read, live_read));
        draw_speed_bar(ui, live_read, "Read Speed");
        if self.running.load(Ordering::SeqCst) {
            ui.colored_label(egui::Color32::YELLOW, "Test Running...");
        } else if let Some(log_path) = &*self.log_path.lock().unwrap() {
            ui.label(format!("Log saved to: {}", log_path.display()));
        }
    }
}

fn draw_speed_bar(ui: &mut egui::Ui, mbps: f64, label: &str) {
    let max_mbps = 1000.0; // Arbitrary max for bar scaling
    let width = 300.0;
    let percent = (mbps / max_mbps).min(1.0);
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(width, 20.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0, egui::Color32::from_gray(40));
        let fill_width = rect.width() * percent as f32;
        let fill_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, rect.height()));
        let color = if mbps > 900.0 {
            egui::Color32::RED
        } else if mbps > 500.0 {
            egui::Color32::YELLOW
        } else {
            egui::Color32::GREEN
        };
        painter.rect_filled(fill_rect, 3.0, color);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}: {:.2} MB/s", label, mbps),
            egui::FontId::proportional(14.0),
            egui::Color32::WHITE,
        );
    }
}
