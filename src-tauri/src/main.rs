#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sysinfo::System; // J'ai retiré Components qui ne servait pas
use tauri::Emitter;  // J'ai retiré Manager qui ne servait pas ici
use std::{thread, time::Duration};
use serde::Serialize;

#[derive(Serialize, Clone)]
struct PcSpecs {
    os: String,
    host_name: String,
    cpu_brand: String,
    cpu_cores: usize,
    total_memory: u64,
    gpu_name: String,
}

#[derive(Serialize, Clone)]
struct RealTimeData {
    cpu_usage: f32,
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
}

#[tauri::command]
fn get_static_specs() -> PcSpecs {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    let gpu = "Carte Graphique (Détection Standard)".to_string();

    PcSpecs {
        os: System::name().unwrap_or("Windows".to_string()) + " " + &System::os_version().unwrap_or_default(),
        host_name: System::host_name().unwrap_or("User-PC".to_string()),
        cpu_brand: sys.cpus().first().map(|cpu| cpu.brand().to_string()).unwrap_or("Unknown".to_string()),
        cpu_cores: sys.physical_core_count().unwrap_or(0),
        total_memory: sys.total_memory(),
        gpu_name: gpu, 
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            
            thread::spawn(move || {
                let mut sys = System::new_all();
                loop {
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    
                    let data = RealTimeData {
                        cpu_usage: sys.global_cpu_info().cpu_usage(),
                        ram_used: sys.used_memory(),
                        ram_total: sys.total_memory(),
                        swap_used: sys.used_swap(),
                    };

                    let _ = app_handle.emit("system-update", data);
                    thread::sleep(Duration::from_secs(1));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_static_specs])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}