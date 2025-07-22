use eframe::egui;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

#[cfg(windows)]
pub fn print_cpu_temperatures() {
    use wmi::{WMIConnection, COMLibrary};
    use serde::Deserialize;
    if let Ok(com_lib) = COMLibrary::new() {
        if let Ok(wmi_con) = WMIConnection::new(com_lib) {
            #[derive(Deserialize, Debug)]
            struct TempSensor {
                #[serde(rename = "CurrentTemperature")]
                current_temperature: Option<u64>,
                #[serde(rename = "InstanceName")]
                instance_name: Option<String>,
            }
            let results: Result<Vec<TempSensor>, _> = wmi_con.query();
            match results {
                Ok(sensors) => {
                    if sensors.is_empty() {
                        println!("No temperature sensors found via WMI (MSAcpi_ThermalZoneTemperature)");
                    } else {
                        for sensor in sensors {
                            if let Some(temp) = sensor.current_temperature {
                                // WMI returns tenths of Kelvin
                                let celsius = (temp as f64 / 10.0) - 273.15;
                                println!("{}: {:.1} °C", sensor.instance_name.as_deref().unwrap_or("Unknown"), celsius);
                            } else {
                                println!("{}: temperature unavailable", sensor.instance_name.as_deref().unwrap_or("Unknown"));
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("WMI query error: {}", e);
                }
            }
        } else {
            println!("Failed to create WMIConnection");
        }
    } else {
        println!("Failed to initialize COMLibrary");
    }
}

#[cfg(not(windows))]
pub fn print_cpu_temperatures() {
    println!("CPU temperature reading is only supported on Windows via WMI and may not be available on all systems.");
}

// Example usage for demonstration:
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_print_cpu_temperatures() {
        print_cpu_temperatures();
    }
}

pub struct StressTest {
    running: Arc<AtomicBool>,
}

impl Default for StressTest {
    fn default() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl StressTest {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("CPU Stress Test");
        
        ui.add_space(20.0);
        
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
                        // Placeholder stress test - will be implemented later
                        std::hint::spin_loop();
                    }
                });
            }
        }
        
        if self.running.load(Ordering::SeqCst) {
            ui.colored_label(egui::Color32::RED, "⚠ CPU Stress Test Running");
        }
    }
}