/// 终端美化输出模块
/// 使用 Unicode 框线字符 + ANSI 颜色

pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";
pub const CYAN: &str = "\x1b[36m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const RED: &str = "\x1b[31m";
pub const DIM: &str = "\x1b[2m";

// Unicode 框线字符
pub const BOX_TOP_LEFT: &str = "┌";
pub const BOX_TOP_RIGHT: &str = "┐";
pub const BOX_BOTTOM_LEFT: &str = "└";
pub const BOX_BOTTOM_RIGHT: &str = "┘";
pub const BOX_HORIZONTAL: &str = "─";
pub const BOX_VERTICAL: &str = "│";
pub const BOX_T_LEFT: &str = "├";
pub const BOX_T_RIGHT: &str = "┤";
pub const BOX_CROSS: &str = "┼";
pub const BOX_T_TOP: &str = "┬";
pub const BOX_T_BOTTOM: &str = "┴";

/// 打印带边框的标题
pub fn print_banner() {
    let title = " System Monitor Pro ";
    let width = 50;
    let padding = (width - title.len()) / 2;

    println!();
    print_line_top(width);
    print_empty_line(width);
    print_centered_text(width, title);
    print_empty_line(width);
    print_line_bottom(width);
    println!();
}

/// 打印服务器状态面板
pub fn print_server_info(port: u16, interfaces: &[super::network::NetworkInterface]) {
    let width = 65;

    print_line_top(width);
    print_row(width, "服务状态", "运行中", true);
    print_separator(width);
    print_row(width, "监听端口", &format!("{}", port), false);
    print_separator(width);

    // 打印所有可用接口
    if interfaces.is_empty() {
        print_row(width, "访问地址", &format!("http://0.0.0.0:{}", port), true);
    } else {
        // 本机访问
        print_row(
            width,
            "本机访问",
            &format!("http://127.0.0.1:{}", port),
            false,
        );
        print_separator(width);

        // 网络接口
        for (i, iface) in interfaces.iter().enumerate() {
            let type_icon = match iface.interface_type {
                super::network::InterfaceType::WiFi => "📶",
                super::network::InterfaceType::Ethernet => "🔌",
                _ => "🌐",
            };

            let label = if i == 0 { "网络接口" } else { "         " };
            let display = format!("{} {}: http://{}:{}", type_icon, iface.name, iface.ip, port);
            let highlight = matches!(iface.interface_type, super::network::InterfaceType::WiFi);

            print_row(width, label, &display, highlight);

            if i < interfaces.len() - 1 {
                print_separator(width);
            }
        }
    }

    print_line_bottom(width);
    println!();
}

/// 打印认证信息
pub fn print_auth_info(username: &str, password: &str) {
    let width = 60;

    print_line_top(width);
    print_row(width, "默认账号", username, false);
    print_separator(width);
    print_row(width, "默认密码", password, false);
    print_line_bottom(width);
    println!();

    println!("{}提示:{} 首次登录后建议修改密码", YELLOW, RESET);
    println!();
}

/// 打印访问提示
pub fn print_access_tips() {
    let width = 60;

    print_line_top(width);
    print_left_text(width, "📱 支持设备");
    print_left_text(width, "   • 同一 WiFi 下的手机、平板、电脑");
    print_left_text(width, "   • 浏览器直接访问上述地址");
    print_empty_line(width);
    print_left_text(width, "🔒 安全说明");
    print_left_text(width, "   • 所有访问需要 JWT 认证");
    print_left_text(width, "   • Token 24 小时后过期");
    print_line_bottom(width);
    println!();
}

/// 工具函数：打印顶边框
fn print_line_top(width: usize) {
    print!("{}{}", CYAN, BOX_TOP_LEFT);
    for _ in 0..width {
        print!("{}", BOX_HORIZONTAL);
    }
    println!("{}{}", BOX_TOP_RIGHT, RESET);
}

/// 工具函数：打印底边框
fn print_line_bottom(width: usize) {
    print!("{}{}", CYAN, BOX_BOTTOM_LEFT);
    for _ in 0..width {
        print!("{}", BOX_HORIZONTAL);
    }
    println!("{}{}", BOX_BOTTOM_RIGHT, RESET);
}

/// 工具函数：打印分隔线
fn print_separator(width: usize) {
    print!("{}{}", CYAN, BOX_T_LEFT);
    for _ in 0..width {
        print!("{}", BOX_HORIZONTAL);
    }
    println!("{}{}", BOX_T_RIGHT, RESET);
}

/// 工具函数：打印空行
fn print_empty_line(width: usize) {
    println!("{}{:width$}{}", CYAN, "", BOX_VERTICAL, width = width);
}

/// 工具函数：打印居中文本
fn print_centered_text(width: usize, text: &str) {
    let padding = (width - text.len()) / 2;
    let right_padding = width - text.len() - padding;
    println!(
        "{}{}{:padding$}{}{}{:right_padding$}{}{}",
        CYAN,
        BOX_VERTICAL,
        "",
        BOLD,
        text,
        "",
        BOX_VERTICAL,
        RESET,
        padding = padding,
        right_padding = right_padding
    );
}

/// 工具函数：打印左右对齐的行
fn print_row(width: usize, label: &str, value: &str, highlight: bool) {
    let label_width = 12;
    let value_color = if highlight { GREEN } else { "" };
    let reset = if highlight { RESET } else { "" };

    let total_content = label_width + 3 + value.len(); // label + " : " + value
    let right_padding = width.saturating_sub(total_content);

    println!(
        "{}{} {:label_width$} : {}{}{}{:right_padding$}{}{}",
        CYAN,
        BOX_VERTICAL,
        label,
        value_color,
        value,
        reset,
        "",
        BOX_VERTICAL,
        RESET,
        label_width = label_width,
        right_padding = right_padding
    );
}

/// 工具函数：打印左对齐文本
fn print_left_text(width: usize, text: &str) {
    let padding = width.saturating_sub(text.len());
    println!(
        "{}{} {}{:padding$}{}{}",
        CYAN,
        BOX_VERTICAL,
        text,
        "",
        BOX_VERTICAL,
        RESET,
        padding = padding
    );
}

/// 打印关闭提示
pub fn print_shutdown() {
    println!();
    println!(
        "{}┌────────────────────────────────────────┐{}",
        CYAN, RESET
    );
    println!(
        "{}│  服务已停止，感谢使用 System Monitor   │{}",
        CYAN, RESET
    );
    println!(
        "{}└────────────────────────────────────────┘{}",
        CYAN, RESET
    );
    println!();
}
