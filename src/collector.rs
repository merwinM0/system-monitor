use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Serialize, Clone)]
pub struct SystemStats {
    // CPU 信息
    pub cpu_usage: f32,
    pub cpu_count: usize,
    pub cpu_name: String,

    // 内存信息（单位：GB）
    pub memory_total: f64,
    pub memory_used: f64,
    pub memory_usage_percent: f64,

    // 磁盘信息
    pub disks: Vec<DiskInfo>,

    // 网络信息
    pub networks: Vec<NetworkInfo>,

    // 系统信息
    pub hostname: String,
    pub os_version: String,
    pub uptime_hours: u64,
}

#[derive(Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub total_gb: f64,
    pub used_gb: f64,
    pub usage_percent: f64,
    pub mount_point: String,
}

#[derive(Serialize, Clone)]
pub struct NetworkInfo {
    pub name: String,
    pub received_mb: u64,
    pub transmitted_mb: u64,
}

pub async fn collect_stats() -> SystemStats {
    let mut sys = System::new_all();

    // 刷新所有信息
    sys.refresh_all();

    // 等待一小会儿让 CPU 使用率计算更准确
    std::thread::sleep(std::time::Duration::from_millis(200));
    sys.refresh_cpu_usage();

    let cpus = sys.cpus();
    let cpu_usage = cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32;
    let cpu_count = sys.cpus().len();
    let cpu_name = sys
        .cpus()
        .get(0)
        .map(|c| c.brand().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // 内存计算（转换为 GB）
    let memory_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_usage_percent = (memory_used / memory_total) * 100.0;

    // 磁盘信息
    let disks = Disks::new_with_refreshed_list();
    let disk_infos: Vec<DiskInfo> = disks
        .iter()
        .map(|disk| {
            let total = disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
            let available = disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
            let used = total - available;
            DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                total_gb: total,
                used_gb: used,
                usage_percent: if total > 0.0 {
                    (used / total) * 100.0
                } else {
                    0.0
                },
                mount_point: disk.mount_point().to_string_lossy().to_string(),
            }
        })
        .collect();

    // 网络信息
    let networks = Networks::new_with_refreshed_list();
    let network_infos: Vec<NetworkInfo> = networks
        .iter()
        .map(|(name, data)| NetworkInfo {
            name: name.to_string(),
            received_mb: data.total_received() / 1024 / 1024,
            transmitted_mb: data.total_transmitted() / 1024 / 1024,
        })
        .collect();

    SystemStats {
        cpu_usage,
        cpu_count,
        cpu_name,
        memory_total,
        memory_used,
        memory_usage_percent,
        disks: disk_infos,
        networks: network_infos,
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
        uptime_hours: System::uptime() / 3600,
    }
}
