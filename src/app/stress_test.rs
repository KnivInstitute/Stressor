use eframe::egui;
use crate::app::cpu_stress::CpuStress;
use crate::app::storage_stress::StorageStress;
use crate::app::selectable_stress::SelectableStress;


pub struct StressTest {
    pub cpu_stress: CpuStress,
    pub storage_stress: StorageStress,
    pub selectable_stress: SelectableStress,
}

impl Default for StressTest {
    fn default() -> Self {
        Self {
            cpu_stress: CpuStress::default(),
            storage_stress: StorageStress::default(),
            selectable_stress: SelectableStress::default(),
        }
    }
}

impl StressTest {
    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Stress Tests");
        ui.separator();
        egui::CollapsingHeader::new("CPU Stress Test").default_open(true).show(ui, |ui| {
            self.cpu_stress.ui(ctx, ui);
        });
        ui.separator();
        egui::CollapsingHeader::new("Storage Stress Test").default_open(true).show(ui, |ui| {
            self.storage_stress.ui(ctx, ui);
        });
        ui.separator();
        egui::CollapsingHeader::new("Custom/Selectable Stress Test").default_open(true).show(ui, |ui| {
            self.selectable_stress.ui(ctx, ui);
        });
    }
}