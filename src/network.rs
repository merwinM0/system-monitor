use std::net::{IpAddr, Ipv4Addr};

/// 获取所有可用的局域网 IP 地址
pub fn get_local_ips() -> Vec<String> {
    let mut ips = Vec::new();

    match local_ip_address::local_ip() {
        Ok(IpAddr::V4(ip)) => {
            // 过滤掉回环地址和虚拟网卡
            if !ip.is_loopback() && !is_virtual_interface(&ip) {
                ips.push(ip.to_string());
            }
        }
        Ok(IpAddr::V6(_)) => {}
        Err(_) => {}
    }

    // 如果上面的方法失败，尝试获取所有接口
    if ips.is_empty() {
        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            for (_, addr) in interfaces {
                if let IpAddr::V4(ip) = addr {
                    if !ip.is_loopback() && !is_virtual_interface(&ip) {
                        let ip_str = ip.to_string();
                        if !ips.contains(&ip_str) {
                            ips.push(ip_str);
                        }
                    }
                }
            }
        }
    }

    // 保底：至少返回 0.0.0.0
    if ips.is_empty() {
        ips.push("0.0.0.0".to_string());
    }

    ips
}

/// 过滤虚拟网卡（Docker、VMware 等）
fn is_virtual_interface(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();

    // 排除常见虚拟网卡网段
    match octets[0] {
        172 if octets[1] == 17 || octets[1] == 18 => true, // Docker
        192 if octets[1] == 168 && octets[2] == 56 => true, // VirtualBox
        10 if octets[1] == 211 || octets[1] == 37 => true, // VMware
        _ => false,
    }
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
