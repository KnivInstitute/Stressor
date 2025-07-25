use configparser::ini::Ini;

pub struct Config {
    // GUI
    pub memory_bar_width: f32,
    pub memory_bar_height: f32,
    // Stressors
    pub memory_warn_threshold: f64,
    pub memory_caution_threshold: f64,
    pub cpu_history_len: usize,
    pub cpu_usage_history_len: usize,
    pub matrix_size: usize,
    pub matrix_duration_secs: u32,
    pub matrix_threads: usize,
    pub ram_buffer_size: usize,
    pub ram_duration_secs: u32,
    pub ram_threads: usize,
    pub compression_block_size: usize,
    pub compression_duration_secs: u32,
    pub compression_threads: usize,
    pub tightloop_duration_secs: u32,
    pub tightloop_threads: usize,
    pub storage_duration_secs: u32,
    pub storage_buffer_mb: u32,
}

impl Config {
    pub fn load() -> Self {
        let mut gui = Ini::new();
        gui.load("src/vals/gui.ini").unwrap();
        println!("Loaded GUI map: {:?}", gui.get_map_ref());
        let mut stressors = Ini::new();
        stressors.load("src/vals/stressors.ini").unwrap();
        let get_f32 = |ini: &Ini, key: &str| {
            ini.getfloat("default", &key.to_lowercase())
                .unwrap_or_else(|_| panic!("Missing or invalid float for key: {}", key))
                .unwrap_or_else(|| panic!("Missing float value for key: {}", key)) as f32
        };
        let get_f64 = |ini: &Ini, key: &str| {
            ini.getfloat("default", &key.to_lowercase())
                .unwrap_or_else(|_| panic!("Missing or invalid float for key: {}", key))
                .unwrap_or_else(|| panic!("Missing float value for key: {}", key))
        };
        let get_usize = |ini: &Ini, key: &str| {
            ini.getint("default", &key.to_lowercase())
                .unwrap_or_else(|_| panic!("Missing or invalid int for key: {}", key))
                .unwrap_or_else(|| panic!("Missing int value for key: {}", key)) as usize
        };
        let get_u32 = |ini: &Ini, key: &str| {
            ini.getint("default", &key.to_lowercase())
                .unwrap_or_else(|_| panic!("Missing or invalid int for key: {}", key))
                .unwrap_or_else(|| panic!("Missing int value for key: {}", key)) as u32
        };
        let get_threads = |ini: &Ini, key: &str| {
            let v = ini.get("default", &key.to_lowercase()).unwrap_or_else(|| panic!("Missing value for key: {}", key));
            if v == "auto" { num_cpus::get() } else { v.parse::<usize>().unwrap_or_else(|_| panic!("Invalid thread count for key: {}", key)) }
        };
        Config {
            memory_bar_width: get_f32(&gui, "MEMORY_BAR_WIDTH"),
            memory_bar_height: get_f32(&gui, "MEMORY_BAR_HEIGHT"),
            memory_warn_threshold: get_f64(&stressors, "MEMORY_WARN_THRESHOLD"),
            memory_caution_threshold: get_f64(&stressors, "MEMORY_CAUTION_THRESHOLD"),
            cpu_history_len: get_usize(&stressors, "CPU_HISTORY_LEN"),
            cpu_usage_history_len: get_usize(&stressors, "CPU_USAGE_HISTORY_LEN"),
            matrix_size: get_usize(&stressors, "MATRIX_SIZE"),
            matrix_duration_secs: get_u32(&stressors, "MATRIX_DURATION_SECS"),
            matrix_threads: get_threads(&stressors, "MATRIX_THREADS"),
            ram_buffer_size: get_usize(&stressors, "RAM_BUFFER_SIZE"),
            ram_duration_secs: get_u32(&stressors, "RAM_DURATION_SECS"),
            ram_threads: get_threads(&stressors, "RAM_THREADS"),
            compression_block_size: get_usize(&stressors, "COMPRESSION_BLOCK_SIZE"),
            compression_duration_secs: get_u32(&stressors, "COMPRESSION_DURATION_SECS"),
            compression_threads: get_threads(&stressors, "COMPRESSION_THREADS"),
            tightloop_duration_secs: get_u32(&stressors, "TIGHTLOOP_DURATION_SECS"),
            tightloop_threads: get_threads(&stressors, "TIGHTLOOP_THREADS"),
            storage_duration_secs: get_u32(&stressors, "STORAGE_DURATION_SECS"),
            storage_buffer_mb: get_u32(&stressors, "STORAGE_BUFFER_MB"),
        }
    }
} 