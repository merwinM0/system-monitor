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
    
    // CPU
    document.getElementById('cpuValue').textContent = r.cpu_usage.toFixed(1) + '%';
    document.getElementById('cpuBar').style.width = r.cpu_usage + '%';
    document.getElementById('cpuMeta').textContent = `${r.cpu_name} · ${r.cpu_count}核`;
    
    // 内存
    document.getElementById('memValue').textContent = r.memory_usage_percent.toFixed(1) + '%';
    document.getElementById('memBar').style.width = r.memory_usage_percent + '%';
    document.getElementById('memMeta').textContent = 
        `${r.memory_used.toFixed(2)}GB / ${r.memory_total.toFixed(2)}GB`;
    
    // GPU
    const gpuEl = document.getElementById('gpuContent');
    if (r.gpu) {
        const g = r.gpu;
        gpuEl.innerHTML = `
            <div class="resource-value">${g.usage_percent}%</div>
            <div class="progress-bar">
                <div class="progress-fill gpu" style="width: ${g.usage_percent}%"></div>
            </div>
            <div class="resource-meta" style="margin-top: 12px;">${g.name}</div>
            <div class="gpu-details">
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">显存</span>
                    <span class="gpu-detail-value">${(g.memory_used_mb/1024).toFixed(2)} / ${(g.memory_total_mb/1024).toFixed(2)} GB</span>
                </div>
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">温度</span>
                    <span class="gpu-detail-value">${g.temperature}°C</span>
                </div>
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">功耗</span>
                    <span class="gpu-detail-value">${g.power_draw_watts}W</span>
                </div>
                <div class="gpu-detail-item">
                    <span class="gpu-detail-label">显存占用</span>
                    <span class="gpu-detail-value">${((g.memory_used_mb/g.memory_total_mb)*100).toFixed(1)}%</span>
                </div>
            </div>
        `;
    } else {
        gpuEl.innerHTML = '<div class="no-gpu">未检测到 NVIDIA GPU</div>';
    }
    
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
    
    // 网络
    document.getElementById('networkList').innerHTML = data.networks.map(n => `
        <div class="network-item">
            <span class="network-name">${n.name}</span>
            <span class="network-traffic">↓${n.received_mb}MB ↑${n.transmitted_mb}MB</span>
        </div>
    `).join('');
    
    // 系统信息
    document.getElementById('systemInfo').innerHTML = `
        <div class="system-info-row">
            <span class="system-info-label">主机名</span>
            <span class="system-info-value">${data.hostname}</span>
        </div>
        <div class="system-info-row">
            <span class="system-info-label">操作系统</span>
            <span class="system-info-value">${data.os_version}</span>
        </div>
        <div class="system-info-row">
            <span class="system-info-label">运行时间</span>
            <span class="system-info-value">${data.uptime_hours}h</span>
        </div>
        ${data.battery ? `
        <div class="system-info-row">
            <span class="system-info-label">电池健康</span>
            <span class="system-info-value">${data.battery.health_percent.toFixed(1)}%</span>
        </div>
        ` : ''}
    `;
}

// 页面可见性控制
document.addEventListener('visibilitychange', () => {
    if (document.hidden) {
        clearInterval(refreshInterval);
    } else {
        refreshInterval = setInterval(fetchStats, 2000);
    }
});
