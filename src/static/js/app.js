// System Monitor - 前端逻辑
// 白底黑边极简风格

let token = localStorage.getItem('monitor_token');
let refreshInterval;

// 初始化
if (token) {
    showMain();
}

// 登录
async function login() {
    const username = document.getElementById('username').value;
    const password = document.getElementById('password').value;
    
    try {
        const res = await fetch('/api/login', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ username, password })
        });
        
        const data = await res.json();
        
        if (res.ok) {
            token = data.token;
            localStorage.setItem('monitor_token', token);
            showMain();
        } else {
            document.getElementById('errorMsg').textContent = data.error || '登录失败';
        }
    } catch (e) {
        document.getElementById('errorMsg').textContent = '网络错误';
    }
}

// 显示主界面
function showMain() {
    document.getElementById('loginModal').style.display = 'none';
    document.getElementById('mainContent').style.display = 'block';
    fetchStats();
    refreshInterval = setInterval(fetchStats, 2000);
}

// 退出登录
function logout() {
    localStorage.removeItem('monitor_token');
    location.reload();
}

// 获取数据
async function fetchStats() {
    try {
        const res = await fetch('/api/stats', {
            headers: { 'Authorization': `Bearer ${token}` }
        });
        
        if (res.status === 401) {
            logout();
            return;
        }
        
        const data = await res.json();
        updateUI(data);
    } catch (e) {
        console.error('Fetch error:', e);
    }
}

// 更新界面
function updateUI(data) {
    const r = data.resources;
    
    // 系统信息（替换标题）
    document.getElementById('systemTitle').textContent = 
        `${data.hostname} · ${data.os_version}`;
    
    // CPU
    document.getElementById('cpuValue').textContent = r.cpu_usage.toFixed(1) + '%';
    document.getElementById('cpuBar').style.width = r.cpu_usage + '%';
    document.getElementById('cpuMeta').textContent = `${r.cpu_name} · ${r.cpu_count}核`;
    
    // 内存
    document.getElementById('memValue').textContent = r.memory_usage_percent.toFixed(1) + '%';
    document.getElementById('memBar').style.width = r.memory_usage_percent + '%';
    document.getElementById('memMeta').textContent = 
        `${r.memory_used.toFixed(2)}GB / ${r.memory_total.toFixed(2)}GB`;
    
    // GPU（自动识别）
    const gpuEl = document.getElementById('gpuContent');
    if (data.gpu) {
        const g = data.gpu;
        const vendorIcon = g.vendor === 'NVIDIA' ? '🟢' : 
                          g.vendor === 'AMD' ? '🔴' : '🔵';
        
        let gpuHtml = `
            <div class="resource-value">${g.usage_percent}%</div>
            <div class="progress-bar">
                <div class="progress-fill gpu" style="width: ${g.usage_percent}%"></div>
            </div>
            <div class="resource-meta" style="margin-top: 12px;">${vendorIcon} ${g.vendor} - ${g.name}</div>
            <div class="gpu-details">
        `;
        
        // 显存（如果有）
        if (g.memory_total_mb > 0) {
            gpuHtml += `
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">显存</span>
                    <span class="gpu-detail-value">${(g.memory_used_mb/1024).toFixed(2)} / ${(g.memory_total_mb/1024).toFixed(2)} GB</span>
                </div>
            `;
        }
        
        // 温度（如果有）
        if (g.temperature > 0) {
            gpuHtml += `
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">温度</span>
                    <span class="gpu-detail-value">${g.temperature}°C</span>
                </div>
            `;
        }
        
        // 风扇转速（如果有）
        if (g.fan_speed_percent !== null && g.fan_speed_percent !== undefined) {
            gpuHtml += `
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">风扇</span>
                    <span class="gpu-detail-value">${g.fan_speed_percent}%</span>
                </div>
            `;
        }
        
        // 核心频率（如果有）
        if (g.core_clock_mhz !== null && g.core_clock_mhz !== undefined) {
            gpuHtml += `
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">核心频率</span>
                    <span class="gpu-detail-value">${g.core_clock_mhz} MHz</span>
                </div>
            `;
        }
        
        // 显存频率（如果有）
        if (g.memory_clock_mhz !== null && g.memory_clock_mhz !== undefined) {
            gpuHtml += `
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">显存频率</span>
                    <span class="gpu-detail-value">${g.memory_clock_mhz} MHz</span>
                </div>
            `;
        }
        
        gpuHtml += '</div>';
        
        // 占用显存的进程（如果有）
        if (g.top_processes && g.top_processes.length > 0) {
            gpuHtml += `
                <div class="gpu-processes">
                    <div class="gpu-process-title">显存占用进程</div>
                    ${g.top_processes.map(p => `
                        <div class="gpu-process-item">
                            <span>${p.name} (PID: ${p.pid})</span>
                            <span>${p.memory_mb} MB</span>
                        </div>
                    `).join('')}
                </div>
            `;
        }
        
        gpuEl.innerHTML = gpuHtml;
    } else {
        gpuEl.innerHTML = '<div class="no-gpu">未检测到 GPU</div>';
    }
    
    // 进程管理
    const processEl = document.getElementById('processList');
    if (data.processes && data.processes.length > 0) {
        processEl.innerHTML = data.processes.map(p => `
            <div class="process-item">
                <div class="process-name">${p.name}</div>
                <div class="process-stats">
                    <span class="process-cpu">CPU: ${p.cpu_usage.toFixed(1)}%</span>
                    <span class="process-mem">MEM: ${p.memory_mb.toFixed(0)}MB</span>
                    <span class="process-status ${p.status}">${p.status}</span>
                </div>
            </div>
        `).join('');
    } else {
        processEl.innerHTML = '<div class="no-data">无进程数据</div>';
    }
    
    // CPU 进阶信息
    const cpuAdvEl = document.getElementById('cpuAdvanced');
    const ca = data.cpu_advanced;
    cpuAdvEl.innerHTML = `
        <div class="cpu-info-row">
            <span class="cpu-info-label">CPU 频率</span>
            <span class="cpu-info-value">${ca.cpu_frequency_mhz} MHz</span>
        </div>
        <div class="cpu-info-row">
            <span class="cpu-info-label">负载均衡</span>
            <span class="cpu-info-value">${ca.load_avg_1.toFixed(2)} / ${ca.load_avg_5.toFixed(2)} / ${ca.load_avg_15.toFixed(2)}</span>
        </div>
        <div class="cpu-core-usage">
            <div class="cpu-core-title">各核心占用率</div>
            <div class="cpu-core-grid">
                ${ca.per_core_usage.map((usage, i) => `
                    <div class="cpu-core-item">
                        <div class="core-label">核心 ${i + 1}</div>
                        <div class="core-bar">
                            <div class="core-fill" style="width: ${usage}%"></div>
                        </div>
                        <div class="core-value">${usage.toFixed(0)}%</div>
                    </div>
                `).join('')}
            </div>
        </div>
    `;
    
    // 电池
    const batEl = document.getElementById('batteryIndicator');
    if (data.battery) {
        const b = data.battery;
        const icon = b.is_charging ? '⚡' : '🔋';
        const cls = b.is_charging ? 'charging' : 'not-charging';
        const time = b.time_remaining_minutes 
            ? `· ${Math.floor(b.time_remaining_minutes / 60)}h ${b.time_remaining_minutes % 60}m`
            : '';
        batEl.innerHTML = `
            <div class="battery-indicator ${cls}">
                <span>${icon}</span>
                <span>${b.percentage.toFixed(0)}% ${time}</span>
            </div>
        `;
    } else {
        batEl.innerHTML = '';
    }
    
    // 磁盘
    document.getElementById('diskList').innerHTML = data.disks.map(d => `
        <div class="disk-item">
            <div class="disk-header">
                <span class="disk-name">${d.name}</span>
                <span class="disk-percent">${d.usage_percent.toFixed(1)}%</span>
            </div>
            <div class="disk-bar">
                <div class="disk-fill" style="width: ${d.usage_percent}%"></div>
            </div>
            <div class="disk-info">${d.used_gb.toFixed(1)} GB / ${d.total_gb.toFixed(1)} GB · ${d.mount_point}</div>
        </div>
    `).join('');
    
    // 网络进阶 + 硬件传感器
    const netSenEl = document.getElementById('networkSensors');
    const net = data.network_advanced;
    const sen = data.sensors;
    
    let netSenHtml = '<div class="section-title">网络状态</div>';
    
    // 实时网速
    netSenHtml += `
        <div class="network-speed">
            <div class="speed-item">
                <span class="speed-label">↓ 下载</span>
                <span class="speed-value">${net.download_speed_mbps.toFixed(2)} Mbps</span>
            </div>
            <div class="speed-item">
                <span class="speed-label">↑ 上传</span>
                <span class="speed-value">${net.upload_speed_mbps.toFixed(2)} Mbps</span>
            </div>
        </div>
    `;
    
    // 网络接口
    if (net.interfaces && net.interfaces.length > 0) {
        netSenHtml += '<div class="network-interfaces">';
        net.interfaces.forEach(n => {
            netSenHtml += `
                <div class="network-item">
                    <span class="network-name">${n.name}</span>
                    <span class="network-traffic">↓${n.received_mb}MB ↑${n.transmitted_mb}MB</span>
                </div>
            `;
        });
        netSenHtml += '</div>';
    }
    
    // 硬件传感器
    netSenHtml += '<div class="section-title" style="margin-top: 16px;">硬件传感器</div>';
    netSenHtml += '<div class="sensor-grid">';
    
    if (sen.cpu_temp_celsius !== null) {
        netSenHtml += `
            <div class="sensor-item">
                <span class="sensor-label">CPU 温度</span>
                <span class="sensor-value">${sen.cpu_temp_celsius.toFixed(1)}°C</span>
            </div>
        `;
    }
    
    if (sen.motherboard_temp_celsius !== null) {
        netSenHtml += `
            <div class="sensor-item">
                <span class="sensor-label">主板温度</span>
                <span class="sensor-value">${sen.motherboard_temp_celsius.toFixed(1)}°C</span>
            </div>
        `;
    }
    
    if (sen.cpu_fan_rpm !== null) {
        netSenHtml += `
            <div class="sensor-item">
                <span class="sensor-label">CPU 风扇</span>
                <span class="sensor-value">${sen.cpu_fan_rpm} RPM</span>
            </div>
        `;
    }
    
    if (sen.cpu_voltage !== null) {
        netSenHtml += `
            <div class="sensor-item">
                <span class="sensor-label">CPU 电压</span>
                <span class="sensor-value">${sen.cpu_voltage.toFixed(2)}V</span>
            </div>
        `;
    }
    
    // 如果没有传感器数据
    if (sen.cpu_temp_celsius === null && sen.motherboard_temp_celsius === null && 
        sen.cpu_fan_rpm === null && sen.cpu_voltage === null) {
        netSenHtml += `
            <div class="no-data">未检测到传感器数据</div>
        `;
    }
    
    netSenHtml += '</div>';
    netSenEl.innerHTML = netSenHtml;
}

// 页面可见性控制
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        clearInterval(refreshInterval);
    } else {
        refreshInterval = setInterval(fetchStats, 2000);
    }
});
