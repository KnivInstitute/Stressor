use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;

#[derive(Clone)]
pub struct RamStressConfig {
    pub buffer_size: usize, // in bytes
    pub duration_secs: u32,
    pub threads: usize,
}

impl RamStressConfig {
    pub fn from_config(config: &crate::app::config::Config) -> Self {
        Self {
            buffer_size: config.ram_buffer_size,
            duration_secs: config.ram_duration_secs,
            threads: config.ram_threads,
        }
    }
}

pub struct RamStress {
    pub config: RamStressConfig,
}

impl RamStress {
    pub fn from_config(config: &crate::app::config::Config) -> Self {
        Self {
            config: RamStressConfig::from_config(config),
        }
    }
    pub fn run_with_counts(&self, stop_flag: Arc<AtomicBool>, op_counts: &mut [u64]) -> u64 {
        let mut handles = Vec::new();
        let results = Arc::new(std::sync::Mutex::new(vec![0u64; self.config.threads]));
        let buffer_size = self.config.buffer_size;
        let duration = self.config.duration_secs;
        for tid in 0..self.config.threads {
            let stop_flag = stop_flag.clone();
            let results = results.clone();
            handles.push(thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut count = 0u64;
                let mut buffer = vec![0u8; buffer_size];
                let start = Instant::now();
                while !stop_flag.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(duration as u64) {
                    // Random write
                    let idx = rng.gen_range(0..buffer_size);
                    buffer[idx] = rng.r#gen();
                    // Optionally, random read
                    let _ = buffer[rng.gen_range(0..buffer_size)];
                    count += 1;
                }
                results.lock().unwrap()[tid] = count;
            }));
        }
        for h in handles { let _ = h.join(); }
        let results = results.lock().unwrap();
        for (i, &v) in results.iter().enumerate() {
            if i < op_counts.len() {
                op_counts[i] = v;
            }
        }
        results.iter().sum()
    }
}
