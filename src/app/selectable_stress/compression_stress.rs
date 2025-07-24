use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::{Duration, Instant};
use rand::Rng;
use flate2::{Compression, write::ZlibEncoder, read::ZlibDecoder};
use std::io::{Write, Read};

#[derive(Clone)]
pub struct CompressionStressConfig {
    pub block_size: usize,
    pub duration_secs: u32,
    pub threads: usize,
}

impl Default for CompressionStressConfig {
    fn default() -> Self {
        Self {
            block_size: 1024 * 1024, // 1MB
            duration_secs: 10,
            threads: num_cpus::get(),
        }
    }
}

pub struct CompressionStress {
    pub config: CompressionStressConfig,
}

impl CompressionStress {
    pub fn run_with_counts(&self, stop_flag: Arc<AtomicBool>, op_counts: &mut [u64]) -> u64 {
        let mut handles = Vec::new();
        let results = Arc::new(std::sync::Mutex::new(vec![0u64; self.config.threads]));
        let block_size = self.config.block_size;
        let duration = self.config.duration_secs;
        for tid in 0..self.config.threads {
            let stop_flag = stop_flag.clone();
            let results = results.clone();
            handles.push(thread::spawn(move || {
                let mut rng = rand::thread_rng();
                let mut count = 0u64;
                let mut data = vec![0u8; block_size];
                rng.fill(&mut data[..]);
                let start = Instant::now();
                while !stop_flag.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(duration as u64) {
                    // Compress
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&data).unwrap();
                    let compressed = encoder.finish().unwrap();
                    // Decompress
                    let mut decoder = ZlibDecoder::new(&compressed[..]);
                    let mut out = Vec::with_capacity(block_size);
                    decoder.read_to_end(&mut out).unwrap();
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
