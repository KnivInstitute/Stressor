use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;

#[derive(Clone)]
pub struct MatrixStressConfig {
    pub matrix_size: usize,
    pub duration_secs: u32,
    pub threads: usize,
}

impl MatrixStressConfig {
    pub fn from_config(config: &crate::app::config::Config) -> Self {
        Self {
            matrix_size: config.matrix_size,
            duration_secs: config.matrix_duration_secs,
            threads: config.matrix_threads,
        }
    }
}

pub struct MatrixStress {
    pub config: MatrixStressConfig,
}

impl MatrixStress {
    pub fn from_config(config: &crate::app::config::Config) -> Self {
        Self {
            config: MatrixStressConfig::from_config(config),
        }
    }
    pub fn run_with_counts(&self, stop_flag: Arc<AtomicBool>, op_counts: &mut [u64]) -> u64 {
        let mut handles = Vec::new();
        let results = Arc::new(std::sync::Mutex::new(vec![0u64; self.config.threads]));
        let size = self.config.matrix_size;
        let duration = self.config.duration_secs;
        for tid in 0..self.config.threads {
            let stop_flag = stop_flag.clone();
            let results = results.clone();
            handles.push(thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut count = 0u64;
                let a: Vec<f64> = (0..size*size).map(|_| rng.r#gen::<f64>()).collect();
                let b: Vec<f64> = (0..size*size).map(|_| rng.r#gen::<f64>()).collect();
                let mut c = vec![0.0f64; size*size];
                let start = Instant::now();
                while !stop_flag.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(duration as u64) {
                    // Matrix multiplication: c = a * b
                    for i in 0..size {
                        for j in 0..size {
                            let mut sum = 0.0;
                            for k in 0..size {
                                sum += a[i*size + k] * b[k*size + j];
                            }
                            c[i*size + j] = sum;
                        }
                    }
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
