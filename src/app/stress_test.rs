use eframe::egui;
use crate::app::cpu_stress::CpuStress;
use crate::app::storage_stress::StorageStress;
use crate::app::selectable_stress::SelectableStress;
use crate::app::config::Config;


pub struct StressTest {
    pub cpu_stress: CpuStress,
    pub storage_stress: StorageStress,
    pub selectable_stress: SelectableStress,
}

impl StressTest {
    pub fn from_config(config: &Config) -> Self {
        Self {
            cpu_stress: CpuStress::from_config(config),
            storage_stress: StorageStress::from_config(config),
            selectable_stress: SelectableStress::from_config(config),
        }
    }
}

impl StressTest {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, dev_mode: bool) {
        ui.heading("Stress Tests");
        ui.separator();
        egui::CollapsingHeader::new("CPU Stress Test").default_open(true).show(ui, |ui| {
            self.cpu_stress.ui(ctx, ui, dev_mode);
        });
        ui.separator();
        egui::CollapsingHeader::new("Storage Stress Test").default_open(true).show(ui, |ui| {
            self.storage_stress.ui(ctx, ui, dev_mode);
        });
        ui.separator();
        egui::CollapsingHeader::new("Custom/Selectable Stress Test").default_open(true).show(ui, |ui| {
            self.selectable_stress.ui(ctx, ui, dev_mode);
        });
    }
}