use battery::{Manager as BatteryManager, State};
use nvml_wrapper::Nvml;
use serde::Serialize;
use sysinfo::{Disks, Networks, System};

#[derive(Serialize, Clone)]
pub struct SystemStats {
    // 合并的资源监控区块
    pub resources: ResourceBlock,
    
    // 磁盘信息
    pub disks: Vec<DiskInfo>,
    
    // 网络信息
    pub networks: Vec<NetworkInfo>,
    
    // 系统信息
    pub hostname: String,
    pub os_version: String,
    pub uptime_hours: u64,
    
    // 电池信息（新增）
    pub battery: Option<BatteryInfo>,
}

#[derive(Serialize, Clone)]
pub struct ResourceBlock {
    // CPU
    pub cpu_usage: f32,
    pub cpu_count: usize,
    pub cpu_name: String,
    
    // 内存
    pub memory_total: f64,
    pub memory_used: f64,
    pub memory_usage_percent: f64,
    
    // GPU（新增）
    pub gpu: Option<GpuInfo>,
}

#[derive(Serialize, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub usage_percent: u32,      // GPU 利用率
    pub memory_total_mb: u64,    // 显存总量
    pub memory_used_mb: u64,     // 显存已用
    pub temperature: u32,        // 温度（摄氏度）
    pub power_draw_watts: u32,   // 功耗（瓦特）
}

#[derive(Serialize, Clone)]
pub struct BatteryInfo {
    pub percentage: f32,         // 电量百分比
    pub is_charging: bool,       // 是否充电中
    pub time_remaining_minutes: Option<i64>, // 剩余时间（分钟）
    pub health_percent: f32,     // 电池健康度
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
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(500));
    sys.refresh_cpu_all();

    // CPU 计算
    let cpus = sys.cpus();
    let cpu_usage = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };
    
    // 内存计算
    let memory_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_usage_percent = if memory_total > 0.0 {
        (memory_used / memory_total) * 100.0
    } else {
        0.0
    };

    // GPU 采集（NVIDIA）
    let gpu = collect_gpu_info();

    // 电池采集
    let battery = collect_battery_info();

    // 磁盘
    let disks = Disks::new_with_refreshed_list();
    let disk_infos: Vec<DiskInfo> = disks.iter().map(|disk| {
        let total = disk.total_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        let available = disk.available_space() as f64 / 1024.0 / 1024.0 / 1024.0;
        let used = total - available;
        DiskInfo {
            name: disk.name().to_string_lossy().to_string(),
            total_gb: total,
            used_gb: used,
            usage_percent: if total > 0.0 { (used / total) * 100.0 } else { 0.0 },
            mount_point: disk.mount_point().to_string_lossy().to_string(),
        }
    }).collect();

    // 网络
    let networks = Networks::new_with_refreshed_list();
    let network_infos: Vec<NetworkInfo> = networks.iter().map(|(name, data)| {
        NetworkInfo {
            name: name.to_string(),
            received_mb: data.total_received() / 1024 / 1024,
            transmitted_mb: data.total_transmitted() / 1024 / 1024,
        }
    }).collect();

    SystemStats {
        resources: ResourceBlock {
            cpu_usage,
            cpu_count: cpus.len(),
            cpu_name: cpus.get(0)
                .map(|c| c.brand().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            memory_total,
            memory_used,
            memory_usage_percent,
            gpu,
        },
        disks: disk_infos,
        networks: network_infos,
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
        uptime_hours: System::uptime() / 3600,
        battery,
    }
}

fn collect_gpu_info() -> Option<GpuInfo> {
    // 尝试初始化 NVML（NVIDIA Management Library）
    match Nvml::init() {
        Ok(nvml) => {
            // 获取第一块 GPU
            match nvml.device_by_index(0) {
                Ok(device) => {
                    let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                    
                    // 利用率
                    let usage_percent = device.utilization_rates()
                        .map(|u| u.gpu)
                        .unwrap_or(0);
                    
                    // 显存
                    let memory_info = device.memory_info().ok()?;
                    let memory_total_mb = memory_info.total / 1024 / 1024;
                    let memory_used_mb = memory_info.used / 1024 / 1024;
                    
                    // 温度
                    let temperature = device.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .unwrap_or(0);
                    
                    // 功耗（毫瓦转瓦特）
                    let power_draw_watts = device.power_usage()
                        .map(|p| p / 1000)
                        .unwrap_or(0);

                    Some(GpuInfo {
                        name,
                        usage_percent,
                        memory_total_mb,
                        memory_used_mb,
                        temperature,
                        power_draw_watts,
                    })
                }
                Err(_) => None,
            }
        }
        Err(_) => {
            // 没有 NVIDIA 显卡或驱动未安装
            None
        }
    }
}

fn collect_battery_info() -> Option<BatteryInfo> {
    let manager = BatteryManager::new().ok()?;
    
    // 获取第一块电池（笔记本通常只有一块）
    let mut batteries = manager.batteries().ok()?;
    let battery = batteries.next()?.ok()?;
    
    let percentage = battery.state_of_charge() * 100.0;
    let is_charging = matches!(battery.state(), State::Charging | State::Full);
    
    // 计算剩余时间
    let time_remaining_minutes = if is_charging {
        None // 充电时不显示剩余时间
    } else {
        battery.time_to_empty().map(|t| t.value as i64 / 60)
    };
    
    let health_percent = battery.state_of_health() * 100.0;

    Some(BatteryInfo {
        percentage,
        is_charging,
        time_remaining_minutes,
        health_percent,
    })
}
