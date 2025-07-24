use eframe::egui;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};
use crate::app::storage_stress::StorageStress;


pub struct StressTest {
    running: Arc<AtomicBool>,
    pub storage_stress: StorageStress,
}

impl Default for StressTest {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            storage_stress: StorageStress::default(),
        }
    }
}

impl StressTest {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Stress Tests");
        ui.separator();
        egui::CollapsingHeader::new("CPU Stress Test").default_open(true).show(ui, |ui| {
            ui.add_space(10.0);
            ui.label("This section will contain CPU stress testing functionality.");
            ui.label("Implementation coming soon...");
            ui.add_space(20.0);
            if ui
                .button(if self.running.load(Ordering::SeqCst) {
                    "Stop CPU Stress"
                } else {
                    "Start CPU Stress"
                })
                .clicked()
            {
                let running = self.running.clone();
                if running.load(Ordering::SeqCst) {
                    running.store(false, Ordering::SeqCst);
                } else {
                    running.store(true, Ordering::SeqCst);
                    thread::spawn(move || {
                        while running.load(Ordering::SeqCst) {
                            std::hint::spin_loop();
                        }
                    });
                }
            }
            if self.running.load(Ordering::SeqCst) {
                ui.colored_label(egui::Color32::RED, "⚠ CPU Stress Test Running");
            }
        });
        ui.separator();
        egui::CollapsingHeader::new("Storage Stress Test").default_open(true).show(ui, |ui| {
            self.storage_stress.ui(ctx, ui);
        });
    }
}