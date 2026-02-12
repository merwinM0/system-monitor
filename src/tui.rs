use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io::{self, Stdout};

pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

/// 初始化终端
pub fn init_terminal() -> io::Result<AppTerminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// 恢复终端
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// 计算居中区域
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// 绘制主界面
pub fn draw_ui(
    f: &mut Frame,
    port: u16,
    username: &str,
    password: &str,
    interfaces: &[super::network::NetworkInterface],
) {
    let area = centered_rect(60, 70, f.area());

    // 清除背景
    f.render_widget(Clear, area);

    // 外框
    let block = Block::default()
        .title(" System Monitor Pro ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // 内部布局
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // 服务状态
            Constraint::Length(5), // 访问地址
            Constraint::Length(4), // 认证信息
            Constraint::Min(1),    // 提示
        ])
        .split(inner_area);

    // 服务状态
    let status = Paragraph::new("● 服务状态: 运行中")
        .style(Style::default().fg(Color::Green))
        .alignment(Alignment::Center);
    f.render_widget(status, chunks[0]);

    // 访问地址
    let mut address_lines = vec![
        Line::from(Span::styled(
            "访问地址:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(format!("  本机: http://127.0.0.1:{}", port)),
    ];

    for iface in interfaces {
        let icon = match iface.interface_type {
            super::network::InterfaceType::WiFi => "📶",
            super::network::InterfaceType::Ethernet => "🔌",
            _ => "🌐",
        };
        address_lines.push(Line::from(format!(
            "  {} {}: http://{}:{}",
            icon, iface.name, iface.ip, port
        )));
    }

    let addresses = Paragraph::new(address_lines);
    f.render_widget(addresses, chunks[1]);

    // 认证信息
    let auth = Paragraph::new(vec![
        Line::from(Span::styled(
            "认证信息:",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(format!("  账号: {}  |  密码: {}", username, password)),
    ]);
    f.render_widget(auth, chunks[2]);

    // 提示
    let tips = Paragraph::new("按 Ctrl+C 停止服务")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    f.render_widget(tips, chunks[3]);
}

/// 绘制关闭界面
pub fn draw_shutdown(f: &mut Frame) {
    let area = centered_rect(40, 20, f.area());
    f.render_widget(Clear, area);

    let shutdown_msg = Paragraph::new("服务已停止，感谢使用！")
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(shutdown_msg, area);
}
