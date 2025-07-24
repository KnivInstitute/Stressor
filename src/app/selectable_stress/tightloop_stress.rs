use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct TightLoopStressConfig {
    pub duration_secs: u32,
    pub threads: usize,
}

impl Default for TightLoopStressConfig {
    fn default() -> Self {
        Self {
            duration_secs: 10,
            threads: num_cpus::get(),
        }
    }
}

pub struct TightLoopStress {
    pub config: TightLoopStressConfig,
}

impl TightLoopStress {
    pub fn run_with_counts(&self, stop_flag: Arc<AtomicBool>, op_counts: &mut [u64]) -> u64 {
        let mut handles = Vec::new();
        let results = Arc::new(std::sync::Mutex::new(vec![0u64; self.config.threads]));
        let duration = self.config.duration_secs;
        for tid in 0..self.config.threads {
            let stop_flag = stop_flag.clone();
            let results = results.clone();
            handles.push(thread::spawn(move || {
                let mut count = 0u64;
                let start = Instant::now();
                while !stop_flag.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(duration as u64) {
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
