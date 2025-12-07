#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use sysinfo::{Disks, Networks, System};
use tauri::{Emitter, Manager}; 
use std::{thread, time::{Duration, Instant}, net::TcpStream, process::Command};
use serde::Serialize;

// --- STRUCTURES DE DONNÉES ---

#[derive(Serialize, Clone)]
struct DiskInfo {
    name: String,
    total_space: u64,
    available_space: u64,
    kind: String,
}

#[derive(Serialize, Clone)]
struct Peripheral {
    name: String,
    kind: String, 
}

#[derive(Serialize, Clone)]
struct PcSpecs {
    os: String,
    host_name: String,
    cpu_brand: String,
    cpu_cores: usize,
    total_memory: u64,
    gpu_name: String,
    disks: Vec<DiskInfo>,
    peripherals: Vec<Peripheral>,
}

#[derive(Serialize, Clone)]
struct RealTimeData {
    cpu_usage: f32,
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
    net_down: u64,
    net_up: u64,
    uptime: u64,
    process_count: usize,
}

#[derive(Serialize, Clone)]
struct AppEntry {
    name: String,
}

// --- FONCTIONS UTILITAIRES ---

fn get_gpu_name() -> String {
    let output = Command::new("wmic")
        .args(&["path", "win32_videocontroller", "get", "name"])
        .output();

    match output {
        Ok(o) => {
            let raw_output = String::from_utf8_lossy(&o.stdout);
            let mut best_gpu = "GPU Inconnu".to_string();
            let mut found_dedicated = false;

            for line in raw_output.lines().skip(1) { 
                let name = line.trim();
                if name.is_empty() { continue; }

                let name_upper = name.to_uppercase();

                // On ignore les drivers virtuels ou de streaming
                if name_upper.contains("VIRTUAL") || name_upper.contains("PARSEC") || name_upper.contains("CITRIX") || name_upper.contains("REMOTE") || name_upper.contains("RDP") {
                    continue;
                }

                // Priorité aux GPU dédiés
                if name_upper.contains("NVIDIA") || name_upper.contains("AMD") || name_upper.contains("RTX") || name_upper.contains("GTX") || name_upper.contains("RADEON") {
                    best_gpu = name.to_string();
                    found_dedicated = true;
                    break; 
                }

                if !found_dedicated {
                    best_gpu = name.to_string();
                }
            }
            best_gpu
        },
        Err(_) => "Erreur détection GPU".to_string(),
    }
}

fn get_connected_peripherals() -> Vec<Peripheral> {
    let mut periphs = Vec::new();
    
    // On scanne large pour ensuite trier dans l'interface
    let script = "Get-PnpDevice -Class 'Monitor','AudioEndpoint','Keyboard','PointingDevice','Camera' -Status 'OK' | Select-Object FriendlyName, Class";
    
    let output = Command::new("powershell")
        .args(&["-NoProfile", "-Command", script])
        .output();

    if let Ok(o) = output {
        let txt = String::from_utf8_lossy(&o.stdout);
        for line in txt.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let kind = parts.last().unwrap().to_string();
                let name = parts[..parts.len()-1].join(" ");
                
                let n_low = name.to_lowercase();
                // Filtre pour enlever le bruit système inutile
                if !name.is_empty() 
                   && !n_low.contains("composite") 
                   && !n_low.contains("root hub") 
                   && !n_low.contains("print") 
                   && !n_low.contains("virtual") 
                   && !n_low.contains("volume") { // Ajout du filtre volume qui est souvent redondant
                    periphs.push(Peripheral { name, kind });
                }
            }
        }
    }
    periphs
}

// --- COMMANDES TAURI ---

#[tauri::command]
fn get_static_specs() -> PcSpecs {
    let mut sys = System::new_all();
    let disks_list = Disks::new_with_refreshed_list();
    sys.refresh_all();

    let mut my_disks = Vec::new();
    for disk in &disks_list {
        my_disks.push(DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            kind: format!("{:?}", disk.kind()),
        });
    }

    PcSpecs {
        os: System::name().unwrap_or("Windows".to_string()) + " " + &System::os_version().unwrap_or_default(),
        host_name: System::host_name().unwrap_or("User-PC".to_string()),
        cpu_brand: sys.cpus().first().map(|cpu| cpu.brand().to_string()).unwrap_or("Unknown CPU".to_string()),
        cpu_cores: sys.physical_core_count().unwrap_or(0),
        total_memory: sys.total_memory(),
        gpu_name: get_gpu_name(), 
        disks: my_disks,
        peripherals: get_connected_peripherals(),
    }
}

#[tauri::command]
async fn run_benchmark() -> u64 {
    let start = Instant::now();
    let mut count = 0;
    // Charge CPU artificielle pour le test
    for i in 2..250_000 { // Légèrement augmenté pour plus de précision
        let mut is_prime = true;
        let limit = (i as f64).sqrt() as i32 + 1;
        for j in 2..limit {
            if i % j == 0 { is_prime = false; break; }
        }
        if is_prime { count += 1; }
    }
    let duration = start.elapsed();
    let ms = duration.as_millis() as u64;
    if ms == 0 { return 0; }
    15_000_000 / ms
}

#[tauri::command]
async fn test_connection() -> String {
    let start = Instant::now();
    // Ping TCP rapide vers Google DNS
    match TcpStream::connect_timeout(&"8.8.8.8:53".parse().unwrap(), Duration::from_secs(2)) {
        Ok(_) => format!("{} ms", start.elapsed().as_millis()),
        Err(_) => "Offline".to_string(),
    }
}

#[tauri::command]
async fn get_recent_apps() -> Vec<AppEntry> {
    // Récupère les processus avec fenêtre, triés par heure de lancement
    let script = "Get-Process | Where-Object {$_.MainWindowTitle -ne \"\"} | Sort-Object StartTime -Descending | Select-Object -First 5 -ExpandProperty MainWindowTitle";
    let output = Command::new("powershell").args(&["-NoProfile", "-Command", script]).output();
    let mut apps = Vec::new();
    if let Ok(o) = output {
        let txt = String::from_utf8_lossy(&o.stdout);
        for line in txt.lines() {
            let name = line.trim().to_string();
            if !name.is_empty() { apps.push(AppEntry { name }); }
        }
    }
    apps
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Maximiser au lancement
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.maximize();
            }

            let app_handle = app.handle().clone();
            
            // Thread de surveillance système
            thread::spawn(move || {
                let mut sys = System::new_all();
                let mut networks = Networks::new_with_refreshed_list();
                
                loop {
                    sys.refresh_cpu();
                    sys.refresh_memory();
                    sys.refresh_processes();
                    networks.refresh(); 

                    let mut total_down = 0;
                    let mut total_up = 0;
                    for (_interface_name, data) in &networks {
                        total_down += data.received();
                        total_up += data.transmitted();
                    }
                    
                    let data = RealTimeData {
                        cpu_usage: sys.global_cpu_info().cpu_usage(),
                        ram_used: sys.used_memory(),
                        ram_total: sys.total_memory(),
                        swap_used: sys.used_swap(),
                        net_down: total_down,
                        net_up: total_up,
                        uptime: System::uptime(),
                        process_count: sys.processes().len(),
                    };

                    let _ = app_handle.emit("system-update", data);
                    thread::sleep(Duration::from_secs(1));
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_static_specs, 
            run_benchmark, 
            test_connection,
            get_recent_apps
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}