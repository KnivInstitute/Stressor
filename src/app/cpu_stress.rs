use eframe::egui;
use std::{
    collections::VecDeque,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{atomic::{AtomicBool, Ordering, AtomicU64}, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use atomic_float::AtomicF64;
use rand::{thread_rng, Rng};
use chrono::Local;
use num_cpus;
use sysinfo::{System, SystemExt, CpuExt};

#[cfg(windows)]
fn set_thread_priority_high() {
    use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};
    use winapi::um::winbase::THREAD_PRIORITY_HIGHEST;
    unsafe {
        let handle = GetCurrentThread();
        SetThreadPriority(handle, THREAD_PRIORITY_HIGHEST as i32);
    }
}
#[cfg(not(windows))]
fn set_thread_priority_high() {}

#[cfg(windows)]
fn set_thread_priority_for_mode(max_stress: bool) {
    use winapi::um::processthreadsapi::{GetCurrentThread, SetThreadPriority};
    use winapi::um::winbase::{THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_NORMAL};
    unsafe {
        let handle = GetCurrentThread();
        let priority = if max_stress {
            THREAD_PRIORITY_HIGHEST as i32
        } else {
            THREAD_PRIORITY_NORMAL as i32
        };
        SetThreadPriority(handle, priority);
    }
}
#[cfg(not(windows))]
fn set_thread_priority_for_mode(_max_stress: bool) {}

const CPU_USAGE_HISTORY_LEN: usize = 100;

pub struct CpuStress {
    running: Arc<AtomicBool>,
    cycle_secs: Arc<Mutex<u32>>,
    intensity: Arc<Mutex<u32>>,
    last_score: Arc<AtomicF64>,
    live_rate: Arc<AtomicF64>,
    log_path: Arc<Mutex<Option<PathBuf>>>,
    cpu_usage_history: Arc<Mutex<VecDeque<f64>>>,
    responsiveness_mode: Arc<Mutex<bool>>, // true = safe, false = max
}

impl Default for CpuStress {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            cycle_secs: Arc::new(Mutex::new(10)),
            intensity: Arc::new(Mutex::new(10000)),
            last_score: Arc::new(AtomicF64::new(0.0)),
            live_rate: Arc::new(AtomicF64::new(0.0)),
            log_path: Arc::new(Mutex::new(None)),
            cpu_usage_history: Arc::new(Mutex::new(VecDeque::with_capacity(CPU_USAGE_HISTORY_LEN))),
            responsiveness_mode: Arc::new(Mutex::new(true)), // default to safe (checked)
        }
    }
}

impl CpuStress {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("CPU Stress Test");
        ui.add_space(10.0);
        ui.label("This test will run a tight loop on all logical CPU threads to stress the CPU and measure how many iterations it can perform in a cycle. At maximum intensity, it will attempt to use 100% of all threads.");
        ui.add_space(10.0);
        let mut cycle = *self.cycle_secs.lock().unwrap();
        let mut intensity = *self.intensity.lock().unwrap();
        ui.horizontal(|ui| {
            ui.label("Cycle Duration (seconds):");
            if ui.add(egui::DragValue::new(&mut cycle).range(1..=60)).changed() {
                *self.cycle_secs.lock().unwrap() = cycle;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Workload Intensity (1-100000):");
            if ui.add(egui::DragValue::new(&mut intensity).range(1..=100000)).changed() {
                *self.intensity.lock().unwrap() = intensity;
            }
        });
        ui.horizontal(|ui| {
            let mut safe_stress = *self.responsiveness_mode.lock().unwrap();
            let label = if safe_stress { "Safe Stress (UI responsive)" } else { "Max Stress (may freeze system)" };
            if ui.checkbox(&mut safe_stress, label).changed() {
                *self.responsiveness_mode.lock().unwrap() = safe_stress;
            }
        });
        ui.add_space(10.0);
        if ui.button(if self.running.load(Ordering::SeqCst) { "Stop CPU Stress" } else { "Start CPU Stress" }).clicked() {
            let running = self.running.clone();
            let cycle_secs = *self.cycle_secs.lock().unwrap();
            let intensity = *self.intensity.lock().unwrap();
            let last_score = self.last_score.clone();
            let live_rate = self.live_rate.clone();
            let log_path = self.log_path.clone();
            let cpu_usage_history = self.cpu_usage_history.clone();
            let responsiveness_mode = *self.responsiveness_mode.lock().unwrap();
            let ctx = ctx.clone();
            if running.load(Ordering::SeqCst) {
                running.store(false, Ordering::SeqCst);
            } else {
                running.store(true, Ordering::SeqCst);
                last_score.store(0.0, Ordering::SeqCst);
                live_rate.store(0.0, Ordering::SeqCst);
                {
                    let mut hist = cpu_usage_history.lock().unwrap();
                    hist.clear();
                }
                let num_threads = num_cpus::get();
                let mut rng = thread_rng();
                let hash: u16 = rng.gen_range(1000..9999);
                let date = Local::now().format("%Y%m%d_%H%M%S");
                let log_file_name = format!("log/cpu_stress_{}_{}_int{}_dur{}.csv", hash, date, intensity, cycle_secs);
                let log_path_val = PathBuf::from(&log_file_name);
                {
                    let mut log_path_guard = log_path.lock().unwrap();
                    *log_path_guard = Some(log_path_val.clone());
                }
                thread::spawn(move || {
                    let mut log_file = OpenOptions::new().create(true).append(true).open(&log_file_name).unwrap();
                    writeln!(log_file, "timestamp,thread,iterations_per_sec").unwrap();
                    let start = Instant::now();
                    let end = start + Duration::from_secs(cycle_secs as u64);
                    let mut handles = Vec::new();
                    let thread_iters: Arc<Vec<AtomicU64>> = Arc::new((0..num_threads).map(|_| AtomicU64::new(0)).collect());
                    let thread_running = running.clone();
                    for tid in 0..num_threads {
                        let thread_iters = thread_iters.clone();
                        let thread_running = thread_running.clone();
                        let thread_intensity = intensity;
                        let thread_safe_stress = responsiveness_mode;
                        handles.push(thread::spawn(move || {
                            set_thread_priority_for_mode(!thread_safe_stress); // false = max, true = safe
                            let mut _local_iters = 0u64;
                            let mut update_counter = 0u64;
                            while thread_running.load(Ordering::SeqCst) && Instant::now() < end {
                                for _ in 0..thread_intensity {
                                    let mut acc = 1u64;
                                    for i in 1..1000 {
                                        acc = acc.wrapping_mul(i ^ tid as u64);
                                    }
                                    std::hint::black_box(acc);
                                    _local_iters += 1;
                                    update_counter += 1;
                                    if update_counter >= 100_000 {
                                        thread_iters[tid].fetch_add(update_counter, Ordering::SeqCst);
                                        update_counter = 0;
                                        if thread_safe_stress {
                                            std::thread::yield_now();
                                        }
                                    }
                                }
                            }
                            if update_counter > 0 {
                                thread_iters[tid].fetch_add(update_counter, Ordering::SeqCst);
                            }
                        }));
                    }
                    let mut last_report = Instant::now();
                    let mut last_iters = vec![0u64; num_threads];
                    let mut sys = System::new_all();
                    while Instant::now() < end && running.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(200));
                        let elapsed = start.elapsed().as_secs_f64();
                        let mut total_iters = 0u64;
                        for tid in 0..num_threads {
                            let iters = thread_iters[tid].load(Ordering::SeqCst);
                            let delta = iters - last_iters[tid];
                            let rate = delta as f64 / (last_report.elapsed().as_secs_f64().max(1e-6));
                            writeln!(log_file, "{},{},{}", Local::now().to_rfc3339(), tid, rate).ok();
                            last_iters[tid] = iters;
                            total_iters += iters;
                        }
                        let rate = total_iters as f64 / elapsed;
                        live_rate.store(rate, Ordering::SeqCst);
                        // Sample system CPU usage
                        sys.refresh_cpu();
                        let avg_cpu_usage = sys.cpus().iter().map(|cpu| cpu.cpu_usage() as f64).sum::<f64>() / sys.cpus().len() as f64;
                        {
                            let mut hist = cpu_usage_history.lock().unwrap();
                            if hist.len() >= CPU_USAGE_HISTORY_LEN {
                                hist.pop_front();
                            }
                            hist.push_back(avg_cpu_usage);
                        }
                        last_report = Instant::now();
                        ctx.request_repaint();
                    }
                    running.store(false, Ordering::SeqCst);
                    for handle in handles {
                        let _ = handle.join();
                    }
                    let elapsed = start.elapsed().as_secs_f64();
                    let total_iters: u64 = (0..num_threads).map(|tid| thread_iters[tid].load(Ordering::SeqCst)).sum();
                    let score = total_iters as f64 / elapsed * intensity as f64;
                    last_score.store(score, Ordering::SeqCst);
                    live_rate.store(0.0, Ordering::SeqCst);
                    ctx.request_repaint();
                });
            }
        }
        let score = self.last_score.load(Ordering::SeqCst);
        let live = self.live_rate.load(Ordering::SeqCst);
        ui.add_space(10.0);
        ui.label(format!("Score: {:.2} (iterations/sec * intensity)", score));
        ui.label(format!("Live Iteration Rate: {:.2} iters/sec", live));
        // Draw CPU usage timeline graph
        {
            let hist = self.cpu_usage_history.lock().unwrap();
            if !hist.is_empty() {
                let points: Vec<(f64, f64)> = hist.iter().enumerate().map(|(i, v)| (i as f64, *v)).collect();
                draw_cpu_stress_graph(ui, &points, egui::Color32::RED, "CPU Usage Timeline", 100.0);
            }
        }
        if !*self.responsiveness_mode.lock().unwrap() {
            ui.colored_label(egui::Color32::RED, "WARNING: Max Stress may freeze your system!");
        }
        if self.running.load(Ordering::SeqCst) {
            ui.colored_label(egui::Color32::YELLOW, "Test Running...");
        } else if let Some(log_path) = &*self.log_path.lock().unwrap() {
            ui.label(format!("Log saved to: {}", log_path.display()));
        }
    }
}

fn draw_cpu_stress_graph(ui: &mut egui::Ui, data: &[(f64, f64)], color: egui::Color32, label: &str, max_val: f32) {
    ui.label(label);
    if data.is_empty() {
        ui.label("No data yet...");
        return;
    }
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(300.0, 100.0), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, 5.0, egui::Color32::from_gray(20));
        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(60));
        painter.line_segment([rect.left_top(), rect.right_top()], stroke);
        painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
        painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
        painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
        for i in 1..4 {
            let y = rect.min.y + (rect.height() * i as f32 / 4.0);
            painter.line_segment(
                [egui::pos2(rect.min.x, y), egui::pos2(rect.max.x, y)],
                egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
            );
        }
        let points: Vec<egui::Pos2> = data.iter().enumerate().map(|(i, &(_, value))| {
            let x = rect.min.x + (i as f32 / (data.len() - 1).max(1) as f32) * rect.width();
            let y = rect.max.y - (value as f32 / max_val) * rect.height();
            egui::pos2(x, y.clamp(rect.min.y, rect.max.y))
        }).collect();
        for i in 1..points.len() {
            painter.line_segment(
                [points[i-1], points[i]],
                egui::Stroke::new(2.0, color),
            );
        }
        if let Some((_, current_value)) = data.last() {
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
