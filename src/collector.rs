use battery::{Manager as BatteryManager, State};
use nvml_wrapper::Nvml;
use serde::Serialize;
use std::fs;
use std::path::Path;
use sysinfo::{Disks, Networks, ProcessStatus, System};

#[derive(Serialize, Clone)]
pub struct SystemStats {
    // 系统信息（移到顶部）
    pub hostname: String,
    pub os_version: String,

    // 合并的资源监控区块
    pub resources: ResourceBlock,

    // CPU 进阶信息（新增）
    pub cpu_advanced: CpuAdvanced,

    // GPU 信息
    pub gpu: Option<GpuInfo>,

    // 进程管理（新增）
    pub processes: Vec<ProcessInfo>,

    // 磁盘信息
    pub disks: Vec<DiskInfo>,

    // 网络进阶 + 硬件传感器（新增）
    pub network_advanced: NetworkAdvanced,
    pub sensors: HardwareSensors,

    // 电池信息
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
}

#[derive(Serialize, Clone)]
pub struct CpuAdvanced {
    // 每个核心的占用率
    pub per_core_usage: Vec<f32>,
    // CPU 频率（MHz）
    pub cpu_frequency_mhz: u64,
    // 负载均衡（1/5/15分钟）
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
}

#[derive(Serialize, Clone)]
pub struct GpuInfo {
    pub vendor: String, // 厂商：NVIDIA / AMD / Intel
    pub name: String,
    pub usage_percent: u32,
    pub memory_total_mb: u64,
    pub memory_used_mb: u64,
    pub temperature: u32,
    pub fan_speed_percent: Option<u32>,     // 风扇转速百分比
    pub core_clock_mhz: Option<u32>,        // 核心频率
    pub memory_clock_mhz: Option<u32>,      // 显存频率
    pub top_processes: Vec<GpuProcessInfo>, // 占用显存的进程
}

#[derive(Serialize, Clone)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub memory_mb: u64,
}

#[derive(Serialize, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_mb: f64,
    pub status: String,
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
pub struct NetworkAdvanced {
    // 网络接口及其流量
    pub interfaces: Vec<NetworkInterface>,
    // 实时网速（需要计算，这里简化为当前值）
    pub download_speed_mbps: f64,
    pub upload_speed_mbps: f64,
}

#[derive(Serialize, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub received_mb: u64,
    pub transmitted_mb: u64,
}

#[derive(Serialize, Clone)]
pub struct HardwareSensors {
    // CPU 温度
    pub cpu_temp_celsius: Option<f32>,
    // 主板温度
    pub motherboard_temp_celsius: Option<f32>,
    // CPU 风扇转速
    pub cpu_fan_rpm: Option<u32>,
    // CPU 电压
    pub cpu_voltage: Option<f32>,
}

#[derive(Serialize, Clone)]
pub struct BatteryInfo {
    pub percentage: f32,
    pub is_charging: bool,
    pub time_remaining_minutes: Option<i64>,
    pub health_percent: f32,
}

// 全局变量存储上次的网络数据（用于计算速度）
static mut LAST_NETWORK_DATA: Option<(u64, u64, std::time::Instant)> = None;

pub async fn collect_stats() -> SystemStats {
    let mut sys = System::new_all();
    sys.refresh_all();
    std::thread::sleep(std::time::Duration::from_millis(500));
    sys.refresh_cpu();

    // CPU 计算
    let cpus = sys.cpus();
    let cpu_usage = if cpus.is_empty() {
        0.0
    } else {
        cpus.iter().map(|cpu| cpu.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };

    // CPU 进阶信息
    let per_core_usage: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();
    let cpu_frequency_mhz = cpus.get(0).map(|c| c.frequency()).unwrap_or(0);
    let load_avg = System::load_average();

    let cpu_advanced = CpuAdvanced {
        per_core_usage,
        cpu_frequency_mhz,
        load_avg_1: load_avg.one,
        load_avg_5: load_avg.five,
        load_avg_15: load_avg.fifteen,
    };

    // 内存计算
    let memory_total = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_usage_percent = if memory_total > 0.0 {
        (memory_used / memory_total) * 100.0
    } else {
        0.0
    };

    // GPU 采集（自动检测）
    let gpu = collect_gpu_info();

    // 进程采集
    let processes = collect_process_info(&sys);

    // 磁盘
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

    // 网络进阶
    let network_advanced = collect_network_advanced();

    // 硬件传感器
    let sensors = collect_hardware_sensors();

    // 电池采集
    let battery = collect_battery_info();

    SystemStats {
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
        os_version: System::long_os_version().unwrap_or_else(|| "Unknown".to_string()),
        resources: ResourceBlock {
            cpu_usage,
            cpu_count: cpus.len(),
            cpu_name: cpus
                .get(0)
                .map(|c| c.brand().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            memory_total,
            memory_used,
            memory_usage_percent,
        },
        cpu_advanced,
        gpu,
        processes,
        disks: disk_infos,
        network_advanced,
        sensors,
        battery,
    }
}

fn collect_gpu_info() -> Option<GpuInfo> {
    // 尝试 NVIDIA
    if let Some(info) = collect_nvidia_gpu() {
        return Some(info);
    }

    // 尝试 AMD
    if let Some(info) = collect_amd_gpu() {
        return Some(info);
    }

    // 尝试 Intel
    if let Some(info) = collect_intel_gpu() {
        return Some(info);
    }

    None
}

fn collect_nvidia_gpu() -> Option<GpuInfo> {
    match Nvml::init() {
        Ok(nvml) => {
            match nvml.device_by_index(0) {
                Ok(device) => {
                    let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                    let usage_percent = device.utilization_rates().map(|u| u.gpu).unwrap_or(0);
                    let memory_info = device.memory_info().ok()?;
                    let memory_total_mb = memory_info.total / 1024 / 1024;
                    let memory_used_mb = memory_info.used / 1024 / 1024;
                    let temperature = device
                        .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                        .unwrap_or(0);

                    // 风扇转速
                    let fan_speed_percent = device.fan_speed(0).ok();

                    // 时钟频率
                    let core_clock_mhz = device
                        .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Graphics)
                        .ok();
                    let memory_clock_mhz = device
                        .clock_info(nvml_wrapper::enum_wrappers::device::Clock::Memory)
                        .ok();

                    // 占用显存的进程（使用正确的 API）
                    let top_processes = device
                        .running_graphics_processes()
                        .ok()
                        .map(|processes| {
                            processes
                                .iter()
                                .filter_map(|p| {
                                    Some(GpuProcessInfo {
                                        pid: p.pid,
                                        name: p
                                            .process_name
                                            .clone()
                                            .unwrap_or_else(|| "unknown".to_string()),
                                        memory_mb: p.used_gpu_memory / 1024 / 1024,
                                    })
                                })
                                .take(5)
                                .collect()
                        })
                        .unwrap_or_default();

                    Some(GpuInfo {
                        vendor: "NVIDIA".to_string(),
                        name,
                        usage_percent,
                        memory_total_mb,
                        memory_used_mb,
                        temperature,
                        fan_speed_percent,
                        core_clock_mhz,
                        memory_clock_mhz,
                        top_processes,
                    })
                }
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

fn collect_amd_gpu() -> Option<GpuInfo> {
    // AMD GPU 通过 sysfs 读取
    // 路径通常是 /sys/class/drm/card0/device/
    let amd_path = Path::new("/sys/class/drm/card0/device");

    if !amd_path.exists() {
        return None;
    }

    // 尝试读取基本信息
    let name = fs::read_to_string(amd_path.join("product_name"))
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "AMD GPU".to_string());

    // 读取频率
    let core_clock_mhz = fs::read_to_string(amd_path.join("pp_dpm_sclk"))
        .ok()
        .and_then(|s| {
            s.lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && line.contains('*') {
                        parts[1]
                            .trim_end_matches('M')
                            .trim_end_matches('H')
                            .trim_end_matches('z')
                            .parse::<u32>()
                            .ok()
                    } else {
                        None
                    }
                })
                .next()
        });

    // 读取温度（需要转换）
    let temperature = fs::read_to_string(Path::new(
        "/sys/class/drm/card0/device/hwmon/hwmon1/temp1_input",
    ))
    .ok()
    .and_then(|s| s.trim().parse::<u32>().ok())
    .map(|temp| temp / 1000); // 毫度转摄氏度

    // 读取风扇转速
    let fan_speed_percent = fs::read_to_string(Path::new(
        "/sys/class/drm/card0/device/hwmon/hwmon1/fan1_input",
    ))
    .ok()
    .and_then(|s| s.trim().parse::<u32>().ok())
    .and_then(|rpm| {
        // 简化计算，假设最大3000RPM
        let percent = (rpm as f32 / 3000.0 * 100.0) as u32;
        Some(if percent > 100 { 100 } else { percent })
    });

    // AMD 占用率和显存需要更复杂的实现，这里简化
    Some(GpuInfo {
        vendor: "AMD".to_string(),
        name,
        usage_percent: 0, // 需要更复杂的计算
        memory_total_mb: 0,
        memory_used_mb: 0,
        temperature: temperature.unwrap_or(0),
        fan_speed_percent,
        core_clock_mhz,
        memory_clock_mhz: None,
        top_processes: vec![], // AMD 需要通过其他方式获取
    })
}

fn collect_intel_gpu() -> Option<GpuInfo> {
    // Intel GPU 通过 sysfs 读取
    let intel_path = Path::new("/sys/class/drm/card0");

    if !intel_path.exists() {
        return None;
    }

    // 读取设备信息
    let device_path = intel_path.join("device");
    if !device_path.exists() {
        return None;
    }

    let name = "Intel Integrated Graphics".to_string();

    // 读取频率
    let core_clock_mhz = fs::read_to_string(intel_path.join("gt_cur_freq_mhz"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok());

    // Intel 集显没有专用显存，温度通常与 CPU 共享
    Some(GpuInfo {
        vendor: "Intel".to_string(),
        name,
        usage_percent: 0,
        memory_total_mb: 0,
        memory_used_mb: 0,
        temperature: 0, // 改为 0，不是 None
        fan_speed_percent: None,
        core_clock_mhz,
        memory_clock_mhz: None,
        top_processes: vec![],
    })
}

fn collect_process_info(sys: &System) -> Vec<ProcessInfo> {
    let mut processes: Vec<ProcessInfo> = sys
        .processes()
        .iter()
        .map(|(pid, process)| {
            ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string(), // 改为 to_string()
                cpu_usage: process.cpu_usage(),
                memory_mb: process.memory() as f64 / 1024.0 / 1024.0,
                status: match process.status() {
                    ProcessStatus::Run => "运行中".to_string(),
                    ProcessStatus::Sleep => "睡眠".to_string(),
                    ProcessStatus::Stop => "停止".to_string(),
                    ProcessStatus::Zombie => "僵尸".to_string(),
                    ProcessStatus::Dead => "死亡".to_string(),
                    ProcessStatus::Idle => "空闲".to_string(),
                    _ => "未知".to_string(),
                },
            }
        })
        .collect();

    // 按CPU占用排序，取前10
    processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
    processes.truncate(10);

    processes
}

fn collect_network_advanced() -> NetworkAdvanced {
    let networks = Networks::new_with_refreshed_list();

    let interfaces: Vec<NetworkInterface> = networks
        .iter()
        .map(|(name, data)| NetworkInterface {
            name: name.to_string(),
            received_mb: data.total_received() / 1024 / 1024,
            transmitted_mb: data.total_transmitted() / 1024 / 1024,
        })
        .collect();

    // 计算实时网速
    let total_received: u64 = interfaces.iter().map(|i| i.received_mb).sum();
    let total_transmitted: u64 = interfaces.iter().map(|i| i.transmitted_mb).sum();

    let (download_speed, upload_speed) = unsafe {
        if let Some((last_rx, last_tx, last_time)) = LAST_NETWORK_DATA {
            let now = std::time::Instant::now();
            let duration = now.duration_since(last_time).as_secs_f64();

            if duration > 0.0 {
                let dl_speed = (total_received - last_rx) as f64 / duration * 8.0 / 1000.0; // Mbps
                let ul_speed = (total_transmitted - last_tx) as f64 / duration * 8.0 / 1000.0;

                LAST_NETWORK_DATA = Some((total_received, total_transmitted, now));
                (dl_speed, ul_speed)
            } else {
                (0.0, 0.0)
            }
        } else {
            LAST_NETWORK_DATA =
                Some((total_received, total_transmitted, std::time::Instant::now()));
            (0.0, 0.0)
        }
    };

    NetworkAdvanced {
        interfaces,
        download_speed_mbps: download_speed,
        upload_speed_mbps: upload_speed,
    }
}

fn collect_hardware_sensors() -> HardwareSensors {
    // 尝试从 /sys/class/hwmon 读取
    let hwmon_base = Path::new("/sys/class/hwmon");

    let mut cpu_temp: Option<f32> = None;
    let mut motherboard_temp: Option<f32> = None;
    let mut cpu_fan: Option<u32> = None;
    let mut cpu_voltage: Option<f32> = None;

    if let Ok(entries) = fs::read_dir(hwmon_base) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = fs::read_to_string(path.join("name"))
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_default();

            // CPU 温度
            if name.contains("coretemp") || name.contains("k10temp") || name.contains("cpu") {
                cpu_temp = fs::read_to_string(path.join("temp1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .map(|t| t / 1000.0);
            }

            // 主板温度
            if name.contains("acpitz") || name.contains("board") {
                motherboard_temp = fs::read_to_string(path.join("temp1_input"))
                    .ok()
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .map(|t| t / 1000.0);
            }

            // 风扇转速
            cpu_fan = fs::read_to_string(path.join("fan1_input"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok());

            // CPU 电压
            cpu_voltage = fs::read_to_string(path.join("in1_input"))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok())
                .map(|v| v / 1000.0);
        }
    }

    // 如果没找到，尝试从 thermal zone 读取
    if cpu_temp.is_none() {
        cpu_temp = fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .ok()
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|t| t / 1000.0);
    }

    HardwareSensors {
        cpu_temp_celsius: cpu_temp,
        motherboard_temp_celsius: motherboard_temp,
        cpu_fan_rpm: cpu_fan,
        cpu_voltage,
    }
}

fn collect_battery_info() -> Option<BatteryInfo> {
    let manager = BatteryManager::new().ok()?;
    let mut batteries = manager.batteries().ok()?;
    let battery = batteries.next()?.ok()?;

    let percentage = battery.state_of_charge().value * 100.0;
    let is_charging = matches!(battery.state(), State::Charging | State::Full);
    let time_remaining_minutes = if is_charging {
        None
    } else {
        battery.time_to_empty().map(|t| t.value as i64 / 60)
    };
    let health_percent = battery.state_of_health().value * 100.0;

    Some(BatteryInfo {
        percentage,
        is_charging,
        time_remaining_minutes,
        health_percent,
    })
}
