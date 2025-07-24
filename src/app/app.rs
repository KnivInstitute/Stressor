use eframe::egui;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
    fs,
};
use sysinfo::{System, SystemExt, CpuExt};
use super::stress_test::StressTest;

pub mod cpu;
pub mod memory;
pub mod storage;

impl SystemMonitorApp {
    pub fn update_system_data(&mut self) {
        cpu::update_cpu_data(self);
        memory::update_memory_data(self);
        storage::update_storage_data(self);
    }

    pub fn ui_system_info(&mut self, ui: &mut egui::Ui) {
        cpu::ui_cpu_info(self, ui);
        memory::ui_memory_info(self, ui);
        storage::ui_storage_info(self, ui);
    }
}

impl eframe::App for SystemMonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_system_data();
        egui::TopBottomPanel::top("tab_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.selectable_label(matches!(self.current_tab, Tab::SystemInfo), "System Info").clicked() {
                    self.current_tab = Tab::SystemInfo;
                }
                if ui
                    .selectable_label(matches!(self.current_tab, Tab::Stress), "Stress Test")
                    .clicked()
                {
                    self.current_tab = Tab::Stress;
                }
                if ui
                    .selectable_label(matches!(self.current_tab, Tab::Analyzers), "Analyzers")
                    .clicked()
                {
                    self.current_tab = Tab::Analyzers;
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::SystemInfo => self.ui_system_info(ui),
                Tab::Stress => self.stress_test.ui(ctx, ui),
                Tab::Analyzers => {
                    // Analyzer logic will be in analyzer.rs
                    ui.label("Analyzer UI will be here");
                }
            }
        });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

pub fn run_app() -> eframe::Result<()> {
    let mut options = eframe::NativeOptions::default();
    options.viewport.inner_size = Some([800.0 * 1.3, 600.0 * 1.1].into());
    eframe::run_native(
        "System Monitor & Stress Tool",
        options,
        Box::new(|_cc| Ok(Box::new(SystemMonitorApp::default()))),
    )
}
