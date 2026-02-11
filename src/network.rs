use std::net::{IpAddr, Ipv4Addr};

/// 网络接口类型
#[derive(Debug, Clone)]
pub enum InterfaceType {
    WiFi,     // 无线网卡
    Ethernet, // 有线网卡
    Virtual,  // 虚拟网卡（Docker/VMware）
    Loopback, // 回环地址
    Other,    // 其他
}

/// 网络接口信息
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip: String,
    pub interface_type: InterfaceType,
}

/// 获取所有可用的网络接口，按优先级排序（WiFi > 以太网 > 其他）
pub fn get_network_interfaces() -> Vec<NetworkInterface> {
    let mut interfaces = Vec::new();

    // 使用 local_ip_address 的正确 API
    match local_ip_address::list_afinet_netifas() {
        Ok(ifas) => {
            for (name, addr) in ifas {
                if let IpAddr::V4(ip) = addr {
                    let ip_str = ip.to_string();
                    let interface_type = classify_interface(&name, &ip);

                    // 跳过回环和虚拟网卡
                    if matches!(
                        interface_type,
                        InterfaceType::Loopback | InterfaceType::Virtual
                    ) {
                        continue;
                    }

                    interfaces.push(NetworkInterface {
                        name: name.to_string(),
                        ip: ip_str,
                        interface_type,
                    });
                }
            }
        }
        Err(e) => {
            eprintln!("获取网络接口失败: {}", e);
        }
    }

    // 按优先级排序：WiFi > 以太网 > 其他
    interfaces.sort_by_key(|i| match i.interface_type {
        InterfaceType::WiFi => 0,
        InterfaceType::Ethernet => 1,
        InterfaceType::Other => 2,
        _ => 3,
    });

    // 去重（同一 IP 只保留一次）
    let mut seen = std::collections::HashSet::new();
    interfaces.retain(|i| seen.insert(i.ip.clone()));

    interfaces
}

/// 获取局域网 IP（优先 WiFi）
pub fn get_local_ips() -> Vec<String> {
    let interfaces = get_network_interfaces();

    if interfaces.is_empty() {
        return vec!["0.0.0.0".to_string()];
    }

    // 优先返回 WiFi，其次是其他局域网 IP
    let wifi_ips: Vec<String> = interfaces
        .iter()
        .filter(|i| matches!(i.interface_type, InterfaceType::WiFi))
        .map(|i| i.ip.clone())
        .collect();

    if !wifi_ips.is_empty() {
        return wifi_ips;
    }

    // 如果没有 WiFi，返回所有非虚拟网卡的 IP
    interfaces.iter().map(|i| i.ip.clone()).collect()
}

/// 获取 WiFi IP（如果有）
pub fn get_wifi_ip() -> Option<String> {
    get_network_interfaces()
        .into_iter()
        .find(|i| matches!(i.interface_type, InterfaceType::WiFi))
        .map(|i| i.ip)
}

/// 判断接口类型
fn classify_interface(name: &str, ip: &Ipv4Addr) -> InterfaceType {
    let name_lower = name.to_lowercase();

    // 1. 检查回环
    if ip.is_loopback() || name_lower.contains("lo") {
        return InterfaceType::Loopback;
    }

    // 2. 检查 WiFi（常见命名）
    let wifi_keywords = [
        "wlan", "wlp", "wifi", "wi-fi", "wl", "ath", "wireless", "radio",
    ];
    for kw in &wifi_keywords {
        if name_lower.contains(kw) {
            return InterfaceType::WiFi;
        }
    }

    // 3. 检查虚拟网卡（Docker/VMware/VirtualBox）
    let virtual_keywords = [
        "docker", "vmware", "virtual", "vbox", "tun", "tap", "br-", "veth", "virbr", "dummy",
        "ifb", "gre", "sit",
    ];
    for kw in &virtual_keywords {
        if name_lower.contains(kw) {
            return InterfaceType::Virtual;
        }
    }

    // 4. 检查 IP 段（Docker 默认网段）
    let octets = ip.octets();
    if octets[0] == 172 && (octets[1] == 17 || octets[1] == 18) {
        return InterfaceType::Virtual; // Docker
    }
    if octets[0] == 192 && octets[1] == 168 && octets[2] == 56 {
        return InterfaceType::Virtual; // VirtualBox
    }

    // 5. 检查以太网（常见命名）
    let eth_keywords = ["eth", "enp", "eno", "ens", "ethernet"];
    for kw in &eth_keywords {
        if name_lower.contains(kw) {
            return InterfaceType::Ethernet;
        }
    }

    InterfaceType::Other
}

/// 判断是否为局域网 IP
pub fn is_lan_ip(ip: &str) -> bool {
    if let Ok(IpAddr::V4(ip)) = ip.parse() {
        let octets = ip.octets();
        // 10.x.x.x
        if octets[0] == 10 {
            return true;
        }
        // 172.16-31.x.x
        if octets[0] == 172 && (16..=31).contains(&octets[1]) {
            return true;
        }
        // 192.168.x.x
        if octets[0] == 192 && octets[1] == 168 {
            return true;
        }
    }
    false
}

/// 打印网络诊断信息（调试用）
pub fn print_network_debug() {
    println!("网络接口诊断：");
    for iface in get_network_interfaces() {
        let type_str = match iface.interface_type {
            InterfaceType::WiFi => "WiFi",
            InterfaceType::Ethernet => "以太网",
            InterfaceType::Virtual => "虚拟",
            InterfaceType::Loopback => "回环",
            InterfaceType::Other => "其他",
        };
        println!("  {} [{}] -> {}", iface.name, type_str, iface.ip);
    }
}
