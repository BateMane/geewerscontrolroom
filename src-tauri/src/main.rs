#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::{
    net::TcpStream,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use sysinfo::{Disks, Networks, Pid, System};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

// Flag Windows pour ne pas créer de fenêtre console
const CREATE_NO_WINDOW: u32 = 0x08000000;
const TOP_PROCESSES: usize = 60;

// --- STRUCTURES DE DONNÉES ---

#[derive(Serialize, Clone)]
struct DiskInfo {
    name: String,
    mount: String,
    total_space: u64,
    available_space: u64,
    kind: String,
    fs: String,
}

#[derive(Serialize, Clone)]
struct PcSpecs {
    os: String,
    kernel: String,
    host_name: String,
    cpu_brand: String,
    cpu_arch: String,
    cpu_cores: usize,
    cpu_threads: usize,
    cpu_freq: u64,
    total_memory: u64,
    disks: Vec<DiskInfo>,
    // GPU, carte mère, BIOS, barrettes RAM, périphériques (JSON brut de PowerShell/CIM)
    hardware: serde_json::Value,
}

#[derive(Serialize, Clone)]
struct ProcInfo {
    pid: u32,
    name: String,
    cpu: f32,
    memory: u64,
}

#[derive(Serialize, Clone)]
struct RealTimeData {
    cpu_usage: f32,
    cpu_cores: Vec<f32>,
    cpu_freq: u64,
    ram_used: u64,
    ram_total: u64,
    swap_used: u64,
    swap_total: u64,
    net_down: u64,
    net_up: u64,
    net_total_down: u64,
    net_total_up: u64,
    disk_read: u64,
    disk_write: u64,
    uptime: u64,
    process_count: usize,
    processes: Vec<ProcInfo>,
}

#[derive(Serialize, Clone)]
struct GpuStats {
    name: String,
    usage: f32,
    temp: f32,
    mem_used: u64,
    mem_total: u64,
    power: f32,
    fan: f32,
}

#[derive(Serialize, Clone)]
struct AppEntry {
    name: String,
    process: String,
    memory: u64,
}

#[derive(Serialize, Clone)]
struct BenchResult {
    single: u64,
    multi: u64,
    threads: usize,
}

#[derive(Serialize, Clone)]
struct PingResult {
    target: String,
    host: String,
    avg: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    jitter: Option<f64>,
    loss: u32,
}

// --- FONCTIONS UTILITAIRES ---

fn powershell(script: &str) -> Option<String> {
    // Force l'UTF-8 sinon les accents (périphériques, titres de fenêtres) sont cassés
    let script = format!("[Console]::OutputEncoding=[Text.Encoding]::UTF8; {script}");
    let out = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn query_hardware() -> serde_json::Value {
    // wmic n'existe plus sur les Windows 11 récents : on passe par CIM
    let script = r#"
$gpus = @(Get-CimInstance Win32_VideoController | Select-Object Name, DriverVersion, VideoModeDescription)
$board = Get-CimInstance Win32_BaseBoard | Select-Object Manufacturer, Product
$bios = Get-CimInstance Win32_BIOS | Select-Object Manufacturer, SMBIOSBIOSVersion
$ram = @(Get-CimInstance Win32_PhysicalMemory | Select-Object Capacity, Speed, Manufacturer, PartNumber)
$per = @(Get-PnpDevice -Class Monitor,AudioEndpoint,Keyboard,Mouse,Camera,Image,Bluetooth -PresentOnly -Status OK -ErrorAction SilentlyContinue | Select-Object FriendlyName, Class)
@{ gpus = $gpus; board = $board; bios = $bios; ram = $ram; peripherals = $per } | ConvertTo-Json -Depth 3 -Compress
"#;
    powershell(script)
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(serde_json::Value::Null)
}

fn parse_f32(s: &str) -> f32 {
    s.trim().parse().unwrap_or(-1.0)
}

// shortcut: stats GPU live uniquement via nvidia-smi (NVIDIA), ajouter ADL/perf counters pour AMD/Intel si besoin
fn query_nvidia() -> Option<GpuStats> {
    let out = Command::new("nvidia-smi")
        .creation_flags(CREATE_NO_WINDOW)
        .args([
            "--query-gpu=name,utilization.gpu,temperature.gpu,memory.used,memory.total,power.draw,fan.speed",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let txt = String::from_utf8_lossy(&out.stdout);
    let f: Vec<&str> = txt.lines().next()?.split(',').collect();
    if f.len() < 7 {
        return None;
    }
    Some(GpuStats {
        name: f[0].trim().to_string(),
        usage: parse_f32(f[1]),
        temp: parse_f32(f[2]),
        mem_used: f[3].trim().parse::<u64>().unwrap_or(0) * 1024 * 1024,
        mem_total: f[4].trim().parse::<u64>().unwrap_or(0) * 1024 * 1024,
        power: parse_f32(f[5]),
        fan: parse_f32(f[6]),
    })
}

fn count_primes(limit: u32) -> u32 {
    let mut count = 0;
    for i in 2..limit {
        let mut is_prime = true;
        let mut j = 2;
        while j * j <= i {
            if i % j == 0 {
                is_prime = false;
                break;
            }
            j += 1;
        }
        if is_prime {
            count += 1;
        }
    }
    count
}

fn ping_target(target: &str, host: &str) -> PingResult {
    const SAMPLES: u32 = 5;
    let addr = host.parse().unwrap();
    let mut times = Vec::new();
    for _ in 0..SAMPLES {
        let start = Instant::now();
        if TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok() {
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        thread::sleep(Duration::from_millis(100));
    }
    let loss = (SAMPLES - times.len() as u32) * 100 / SAMPLES;
    if times.is_empty() {
        return PingResult { target: target.into(), host: host.into(), avg: None, min: None, max: None, jitter: None, loss };
    }
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let jitter = times.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (times.len().max(2) - 1) as f64;
    PingResult {
        target: target.into(),
        host: host.into(),
        avg: Some(avg),
        min: times.iter().cloned().reduce(f64::min),
        max: times.iter().cloned().reduce(f64::max),
        jitter: Some(jitter),
        loss,
    }
}

// --- COMMANDES TAURI ---

#[tauri::command]
async fn get_static_specs() -> PcSpecs {
    tauri::async_runtime::spawn_blocking(|| {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list()
            .iter()
            .map(|d| DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount: d.mount_point().to_string_lossy().to_string(),
                total_space: d.total_space(),
                available_space: d.available_space(),
                kind: format!("{:?}", d.kind()),
                fs: d.file_system().to_string_lossy().to_string(),
            })
            .collect();

        PcSpecs {
            os: System::long_os_version().unwrap_or_else(|| "Windows".into()),
            kernel: System::kernel_version().unwrap_or_default(),
            host_name: System::host_name().unwrap_or_else(|| "User-PC".into()),
            cpu_brand: sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_else(|| "Unknown CPU".into()),
            cpu_arch: System::cpu_arch().unwrap_or_default(),
            cpu_cores: sys.physical_core_count().unwrap_or(0),
            cpu_threads: sys.cpus().len(),
            cpu_freq: sys.cpus().first().map(|c| c.frequency()).unwrap_or(0),
            total_memory: sys.total_memory(),
            disks,
            hardware: query_hardware(),
        }
    })
    .await
    .expect("specs thread panicked")
}

#[tauri::command]
async fn run_benchmark() -> BenchResult {
    const LIMIT: u32 = 200_000;
    // Une passe ne dure que ~35 ms en release : on répète pour un score stable
    const REPS: u64 = 10;
    tauri::async_runtime::spawn_blocking(|| {
        let work = || (0..REPS).map(|_| count_primes(LIMIT)).sum::<u32>();
        // Même échelle que l'ancien score (10 000 000 / ms par passe), en microsecondes pour la précision
        let score = |us: u128| (10_000_000_000 * REPS as u128 / us.max(1)) as u64;

        let start = Instant::now();
        work();
        let single = score(start.elapsed().as_micros());

        let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let start = Instant::now();
        let handles: Vec<_> = (0..threads).map(|_| thread::spawn(work)).collect();
        for h in handles {
            let _ = h.join();
        }
        let multi = score(start.elapsed().as_micros()) * threads as u64;

        BenchResult { single, multi, threads }
    })
    .await
    .expect("benchmark thread panicked")
}

#[tauri::command]
async fn test_connection() -> Vec<PingResult> {
    let targets = [("Google", "8.8.8.8:53"), ("Cloudflare", "1.1.1.1:53"), ("Quad9", "9.9.9.9:53")];
    let handles: Vec<_> = targets
        .iter()
        .map(|&(t, h)| tauri::async_runtime::spawn_blocking(move || ping_target(t, h)))
        .collect();
    let mut results = Vec::new();
    for h in handles {
        if let Ok(r) = h.await {
            results.push(r);
        }
    }
    results
}

#[tauri::command]
async fn get_recent_apps() -> Vec<AppEntry> {
    // StartTime est inaccessible pour les processus élevés : on trie sans planter
    let script = r#"@(Get-Process | Where-Object { $_.MainWindowTitle -ne '' } | Sort-Object { try { $_.StartTime } catch { [datetime]::MinValue } } -Descending | Select-Object -First 9 @{n='name';e={$_.MainWindowTitle}}, @{n='process';e={$_.ProcessName}}, @{n='memory';e={[int64]$_.WorkingSet64}}) | ConvertTo-Json -Compress"#;
    tauri::async_runtime::spawn_blocking(move || {
        #[derive(serde::Deserialize)]
        struct Raw {
            name: String,
            process: String,
            memory: u64,
        }
        let raw = powershell(script).unwrap_or_default();
        // ConvertTo-Json renvoie un objet seul (pas un tableau) quand il n'y a qu'une fenêtre
        serde_json::from_str::<Vec<Raw>>(&raw)
            .or_else(|_| serde_json::from_str::<Raw>(&raw).map(|r| vec![r]))
            .unwrap_or_default()
            .into_iter()
            .map(|r| AppEntry { name: r.name, process: r.process, memory: r.memory })
            .collect()
    })
    .await
    .unwrap_or_default()
}

#[tauri::command]
fn kill_process(pid: u32) -> Result<(), String> {
    if pid <= 4 || pid == std::process::id() {
        return Err("Processus protégé".into());
    }
    let mut sys = System::new();
    let pid = Pid::from_u32(pid);
    sys.refresh_process(pid);
    match sys.process(pid) {
        Some(p) if p.kill() => Ok(()),
        Some(_) => Err("Accès refusé".into()),
        None => Err("Processus introuvable".into()),
    }
}

// --- BOUCLES DE MONITORING ---

fn spawn_monitor(app: AppHandle) {
    thread::spawn(move || {
        let mut sys = System::new_all();
        let mut networks = Networks::new_with_refreshed_list();
        let threads = sys.cpus().len().max(1) as f32;
        let (mut total_down, mut total_up) = (0u64, 0u64);

        loop {
            // Attendre avant la mesure : le 1er échantillon CPU juste après new_all() est faux (pic à 100 %)
            thread::sleep(Duration::from_secs(1));
            sys.refresh_cpu();
            sys.refresh_memory();
            sys.refresh_processes();
            networks.refresh();

            let (mut down, mut up) = (0, 0);
            for (_, data) in &networks {
                down += data.received();
                up += data.transmitted();
            }
            total_down += down;
            total_up += up;

            let (mut disk_read, mut disk_write) = (0, 0);
            let mut processes: Vec<ProcInfo> = sys
                .processes()
                .iter()
                .map(|(pid, p)| {
                    let du = p.disk_usage();
                    disk_read += du.read_bytes;
                    disk_write += du.written_bytes;
                    ProcInfo {
                        pid: pid.as_u32(),
                        name: p.name().to_string(),
                        // sysinfo donne un % par cœur (jusqu'à 100 * threads)
                        cpu: p.cpu_usage() / threads,
                        memory: p.memory(),
                    }
                })
                .filter(|p| p.pid != 0)
                .collect();
            processes.sort_by(|a, b| b.cpu.total_cmp(&a.cpu).then(b.memory.cmp(&a.memory)));
            processes.truncate(TOP_PROCESSES);

            let cpu_usage = sys.global_cpu_info().cpu_usage();
            let data = RealTimeData {
                cpu_usage,
                cpu_cores: sys.cpus().iter().map(|c| c.cpu_usage()).collect(),
                cpu_freq: sys.cpus().iter().map(|c| c.frequency()).max().unwrap_or(0),
                ram_used: sys.used_memory(),
                ram_total: sys.total_memory(),
                swap_used: sys.used_swap(),
                swap_total: sys.total_swap(),
                net_down: down,
                net_up: up,
                net_total_down: total_down,
                net_total_up: total_up,
                disk_read,
                disk_write,
                uptime: System::uptime(),
                process_count: sys.processes().len(),
                processes,
            };

            if let Some(tray) = app.tray_by_id("main") {
                let ram = sys.used_memory() as f64 / sys.total_memory().max(1) as f64 * 100.0;
                let _ = tray.set_tooltip(Some(format!("Geewer's Control Room\nCPU {cpu_usage:.0}% · RAM {ram:.0}%")));
            }
            let _ = app.emit("system-update", data);
        }
    });
}

fn spawn_gpu_monitor(app: AppHandle) {
    thread::spawn(move || loop {
        match query_nvidia() {
            Some(stats) => {
                let _ = app.emit("gpu-update", stats);
                thread::sleep(Duration::from_secs(1));
            }
            // Pas de GPU NVIDIA : inutile de relancer le processus en boucle
            None => return,
        }
    });
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.maximize();
            }

            let show = MenuItem::with_id(app, "show", "Ouvrir / Open", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quitter / Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Geewer's Control Room")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            spawn_monitor(app.handle().clone());
            spawn_gpu_monitor(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_static_specs,
            run_benchmark,
            test_connection,
            get_recent_apps,
            kill_process
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primes_below_100() {
        assert_eq!(count_primes(100), 25);
    }

    #[test]
    fn protected_pids_cannot_be_killed() {
        assert!(kill_process(4).is_err());
        assert!(kill_process(std::process::id()).is_err());
    }
}
