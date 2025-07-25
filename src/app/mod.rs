pub mod cpu;
pub mod memory;
pub mod storage;
pub mod stress_test;
pub mod storage_stress;
pub mod cpu_stress;
pub mod analyzer;
pub mod onload;
pub mod selectable_stress;
pub mod config;
use self::analyzer::Analyzer;
use self::stress_test::StressTest;
use eframe::egui;
use std::{
    time::{Duration, Instant},
};
use crate::app::cpu::History;
use crate::app::config::Config;
use sysinfo::{System, CpuExt};
use sysinfo::SystemExt;

pub enum Tab {
    SystemInfo,
    Stress,
    Analyzers,
}

pub struct SystemMonitorApp {
    pub sys: System,
    pub current_tab: Tab,
    pub stress_test: StressTest,
    pub cpu_history: History<(f64, f64)>,
    pub memory_history: History<(f64, f64)>,
    pub last_update: Instant,
    pub time_counter: f64,
    pub max_cpu_freq: u64,
    pub current_cpu_freq: u64,
    #[cfg(windows)]
    pub wmi_con: Option<wmi::WMIConnection>,
    pub analyzer: Analyzer,
    pub cpu_temperature_celsius: Option<u32>,
    pub dev_mode: bool,
    pub last_error: Option<String>,
    pub config: Config,
}

impl Default for SystemMonitorApp {
    fn default() -> Self {
        Self::with_dev_mode(false)
    }
}

impl SystemMonitorApp {
    pub fn with_dev_mode(dev_mode: bool) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let max_freq = sys.cpus().iter()
            .map(|cpu| cpu.frequency())
            .max()
            .unwrap_or(3000);
        #[cfg(windows)]
        let (_wmi_com, wmi_con) = {
            let com = wmi::COMLibrary::new().ok();
            let con = com.as_ref().and_then(|c| wmi::WMIConnection::new(c.clone()).ok());
            (com, con)
        };
        let config = Config::load();
        Self {
            sys,
            current_tab: Tab::SystemInfo,
            stress_test: StressTest::from_config(&config),
            cpu_history: History::new(config.cpu_history_len),
            memory_history: History::new(config.cpu_history_len),
            last_update: Instant::now(),
            time_counter: 0.0,
            max_cpu_freq: max_freq,
            current_cpu_freq: 0,
            #[cfg(windows)]
            wmi_con,
            analyzer: Analyzer::default(),
            cpu_temperature_celsius: None,
            dev_mode,
            last_error: None,
            config,
        }
    }
    pub fn update_system_data(&mut self) {
        cpu::update_cpu_data(self);
        memory::update_memory_data(self);
        storage::update_storage_data(self);
    }

    pub fn ui_system_info(&mut self, ui: &mut egui::Ui) {
        cpu::ui_cpu_info(self, ui);
        storage::ui_storage_info(self, ui);
        // Only borrow self mutably once, call memory last
        let config_ptr: *const _ = &self.config;
        memory::ui_memory_info(self, ui, unsafe { &*config_ptr });
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
                Tab::Stress => self.stress_test.ui(ctx, ui, self.dev_mode),
                Tab::Analyzers => self.analyzer.ui(ctx, ui),
            }
        });
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
