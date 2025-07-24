use eframe::egui;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

mod matrix_stress;
mod compression_stress;
mod ram_stress;
mod tightloop_stress;

use matrix_stress::{MatrixStress, MatrixStressConfig};
use compression_stress::{CompressionStress, CompressionStressConfig};
use ram_stress::{RamStress, RamStressConfig};
use tightloop_stress::{TightLoopStress, TightLoopStressConfig};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CpuWorkloadKind {
    TightLoop,
    MatrixMultiplication,
    Compression,
    RandomMemoryAccess,
}

impl CpuWorkloadKind {
    pub fn all() -> &'static [CpuWorkloadKind] {
        &[
            CpuWorkloadKind::TightLoop,
            CpuWorkloadKind::MatrixMultiplication,
            CpuWorkloadKind::Compression,
            CpuWorkloadKind::RandomMemoryAccess,
        ]
    }
    pub fn label(self) -> &'static str {
        match self {
            CpuWorkloadKind::TightLoop => "Tight Loop",
            CpuWorkloadKind::MatrixMultiplication => "Matrix Multiplication",
            CpuWorkloadKind::Compression => "Compression",
            CpuWorkloadKind::RandomMemoryAccess => "Random Memory Access",
        }
    }
}

pub struct SelectableStress {
    pub selected_cpu_workload: CpuWorkloadKind,
    pub running: bool,
    pub progress: Arc<Mutex<f32>>,
    pub result: Arc<Mutex<Option<u64>>>,
    pub matrix_config: MatrixStressConfig,
    pub compression_config: CompressionStressConfig,
    pub ram_config: RamStressConfig,
    pub tightloop_config: TightLoopStressConfig,
    pub stop_flag: Option<Arc<AtomicBool>>,
    pub log_path: Arc<Mutex<Option<String>>>,
}

impl Default for SelectableStress {
    fn default() -> Self {
        Self {
            selected_cpu_workload: CpuWorkloadKind::TightLoop,
            running: false,
            progress: Arc::new(Mutex::new(0.0)),
            result: Arc::new(Mutex::new(None)),
            matrix_config: MatrixStressConfig::default(),
            compression_config: CompressionStressConfig::default(),
            ram_config: RamStressConfig::default(),
            tightloop_config: TightLoopStressConfig::default(),
            stop_flag: None,
            log_path: Arc::new(Mutex::new(None)),
        }
    }
}

impl SelectableStress {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Custom/Selectable Stress Test");
        ui.add_space(10.0);
        ui.label("Choose a CPU workload:");
        egui::ComboBox::from_label("")
            .selected_text(self.selected_cpu_workload.label())
            .show_ui(ui, |ui| {
                for &kind in CpuWorkloadKind::all() {
                    ui.selectable_value(&mut self.selected_cpu_workload, kind, kind.label());
                }
            });
        ui.add_space(10.0);
        match self.selected_cpu_workload {
            CpuWorkloadKind::TightLoop => {
                ui.label("A simple tight loop for maximum raw CPU usage.");
                ui.horizontal(|ui| {
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.tightloop_config.threads).range(1..=num_cpus::get()));
                    ui.label("Duration (s):");
                    ui.add(egui::DragValue::new(&mut self.tightloop_config.duration_secs).range(1..=300));
                });
            }
            CpuWorkloadKind::MatrixMultiplication => {
                ui.label("Performs repeated matrix multiplications (CPU, cache, and memory stress).");
                ui.horizontal(|ui| {
                    ui.label("Matrix size:");
                    ui.add(egui::DragValue::new(&mut self.matrix_config.matrix_size).range(8..=512));
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.matrix_config.threads).range(1..=num_cpus::get()));
                    ui.label("Duration (s):");
                    ui.add(egui::DragValue::new(&mut self.matrix_config.duration_secs).range(1..=300));
                });
            }
            CpuWorkloadKind::Compression => {
                ui.label("Performs repeated compression/decompression (CPU and memory stress).");
                ui.horizontal(|ui| {
                    ui.label("Block size (bytes):");
                    ui.add(egui::DragValue::new(&mut self.compression_config.block_size).range(1024..=16*1024*1024));
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.compression_config.threads).range(1..=num_cpus::get()));
                    ui.label("Duration (s):");
                    ui.add(egui::DragValue::new(&mut self.compression_config.duration_secs).range(1..=300));
                });
            }
            CpuWorkloadKind::RandomMemoryAccess => {
                ui.label("Performs random memory accesses (memory bandwidth and latency stress).");
                ui.horizontal(|ui| {
                    ui.label("Buffer size (bytes):");
                    ui.add(egui::DragValue::new(&mut self.ram_config.buffer_size).range(1024..=1024*1024*1024));
                    ui.label("Threads:");
                    ui.add(egui::DragValue::new(&mut self.ram_config.threads).range(1..=num_cpus::get()));
                    ui.label("Duration (s):");
                    ui.add(egui::DragValue::new(&mut self.ram_config.duration_secs).range(1..=300));
                });
            }
        }
        ui.add_space(10.0);
        if !self.running {
            if ui.button("Start").clicked() {
                *self.result.lock().unwrap() = None;
                *self.progress.lock().unwrap() = 0.0;
                *self.log_path.lock().unwrap() = None;
                let stop_flag = Arc::new(AtomicBool::new(false));
                self.stop_flag = Some(stop_flag.clone());
                self.running = true;
                let kind = self.selected_cpu_workload;
                let matrix_config = self.matrix_config.clone();
                let matrix_config_for_csv = self.matrix_config.clone();
                let compression_config = self.compression_config.clone();
                let compression_config_for_csv = self.compression_config.clone();
                let ram_config = self.ram_config.clone();
                let _ram_config_for_csv = self.ram_config.clone();
                let tightloop_config = self.tightloop_config.clone();
                let _tightloop_config_for_csv = self.tightloop_config.clone();
                let ctx = ctx.clone();
                let progress = self.progress.clone();
                let result = self.result.clone();
                let log_path = self.log_path.clone();
                thread::spawn(move || {
                    let duration = match kind {
                        CpuWorkloadKind::MatrixMultiplication => matrix_config.duration_secs,
                        CpuWorkloadKind::Compression => compression_config.duration_secs,
                        CpuWorkloadKind::TightLoop => tightloop_config.duration_secs,
                        CpuWorkloadKind::RandomMemoryAccess => ram_config.duration_secs,
                    };
                    let start = Instant::now();
                    let mut op_counts = vec![0u64; match kind {
                        CpuWorkloadKind::MatrixMultiplication => matrix_config.threads,
                        CpuWorkloadKind::Compression => compression_config.threads,
                        CpuWorkloadKind::TightLoop => tightloop_config.threads,
                        CpuWorkloadKind::RandomMemoryAccess => ram_config.threads,
                    }];
                    let stop_flag2 = stop_flag.clone();
                    // Progress updater
                    let progress_clone = progress.clone();
                    let updater = thread::spawn(move || {
                        while !stop_flag2.load(Ordering::SeqCst) {
                            let elapsed = start.elapsed().as_secs_f32();
                            let prog = (elapsed / duration as f32).min(1.0);
                            *progress_clone.lock().unwrap() = prog;
                            thread::sleep(Duration::from_millis(100));
                        }
                        *progress_clone.lock().unwrap() = 1.0;
                    });
                    // Run workload
                    let _res = match kind {
                        CpuWorkloadKind::MatrixMultiplication => {
                            let stress = MatrixStress { config: matrix_config.clone() };
                            stress.run_with_counts(stop_flag.clone(), &mut op_counts)
                        }
                        CpuWorkloadKind::Compression => {
                            let stress = CompressionStress { config: compression_config.clone() };
                            stress.run_with_counts(stop_flag.clone(), &mut op_counts)
                        }
                        CpuWorkloadKind::TightLoop => {
                            let stress = TightLoopStress { config: tightloop_config.clone() };
                            stress.run_with_counts(stop_flag.clone(), &mut op_counts)
                        }
                        CpuWorkloadKind::RandomMemoryAccess => {
                            let stress = RamStress { config: ram_config.clone() };
                            stress.run_with_counts(stop_flag.clone(), &mut op_counts)
                        }
                    };
                    stop_flag.store(true, Ordering::SeqCst);
                    let _ = updater.join();
                    let total_ops = op_counts.iter().sum();
                    *result.lock().unwrap() = Some(total_ops);
                    *progress.lock().unwrap() = 1.0;
                    // Save CSV
                    let date = Local::now().format("%Y%m%d_%H%M%S");
                    let filename = match kind {
                        CpuWorkloadKind::MatrixMultiplication => format!("log/selectable_matrix_{}_size{}_threads{}_dur{}.csv", date, matrix_config_for_csv.matrix_size, matrix_config_for_csv.threads, matrix_config_for_csv.duration_secs),
                        CpuWorkloadKind::Compression => format!("log/selectable_compression_{}_block{}_threads{}_dur{}.csv", date, compression_config_for_csv.block_size, compression_config_for_csv.threads, compression_config_for_csv.duration_secs),
                        CpuWorkloadKind::TightLoop => format!("log/selectable_tightloop_{}_threads{}_dur{}.csv", date, tightloop_config.threads, tightloop_config.duration_secs),
                        CpuWorkloadKind::RandomMemoryAccess => format!("log/selectable_ram_{}_buf{}_threads{}_dur{}.csv", date, ram_config.buffer_size, ram_config.threads, ram_config.duration_secs),
                    };
                    let mut file = OpenOptions::new().create(true).append(true).open(&filename).unwrap();
                    writeln!(file, "timestamp,workload,params,total_ops,thread,thread_ops").unwrap();
                    for (tid, &count) in op_counts.iter().enumerate() {
                        let line = format!(
                            "{}\t{}\t{}\t{}\t{}\t{}",
                            date,
                            kind.label(),
                            match kind {
                                CpuWorkloadKind::MatrixMultiplication => format!("size={},threads={},dur={}", matrix_config_for_csv.matrix_size, matrix_config_for_csv.threads, matrix_config_for_csv.duration_secs),
                                CpuWorkloadKind::Compression => format!("block={},threads={},dur={}", compression_config_for_csv.block_size, compression_config_for_csv.threads, compression_config_for_csv.duration_secs),
                                CpuWorkloadKind::TightLoop => format!("threads={},dur={}", tightloop_config.threads, tightloop_config.duration_secs),
                                CpuWorkloadKind::RandomMemoryAccess => format!("buf={},threads={},dur={}", ram_config.buffer_size, ram_config.threads, ram_config.duration_secs),
                            },
                            total_ops,
                            tid,
                            count
                        ).replace('\t', ",");
                        writeln!(file, "{}", line).unwrap();
                    }
                    *log_path.lock().unwrap() = Some(filename);
                    ctx.request_repaint();
                });
            }
        } else {
            if ui.button("Stop").clicked() {
                if let Some(flag) = &self.stop_flag {
                    flag.store(true, Ordering::SeqCst);
                }
                self.running = false;
            }
            ui.add(egui::ProgressBar::new(*self.progress.lock().unwrap()).show_percentage());
            ui.label("Running...");
        }
        if let Some(res) = *self.result.lock().unwrap() {
            ui.label(format!("Result: {} operations performed.", res));
        }
        if let Some(ref path) = *self.log_path.lock().unwrap() {
            ui.label(format!("Log saved to: {}", path));
        }
    }
}
