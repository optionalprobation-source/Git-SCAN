use rayon::prelude::*;
use crate::scanner::files::should_scan_file;

pub struct ScanEngine;

impl ScanEngine {
    // Ye function dikhata hai CPU multi-threading kaise use karni hai
    pub fn scan_files_in_parallel(file_paths: Vec<String>) -> Vec<String> {
        // .iter() ki jagah .par_iter() use karne se auto-multithreading on ho jati hai
        let files_to_scan: Vec<String> = file_paths
            .into_par_iter() // 🚀 Rayon magic: uses all CPU cores
            .filter(|path| should_scan_file(path))
            .collect();
            
        files_to_scan
    }
}
