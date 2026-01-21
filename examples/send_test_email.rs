//! 测试邮件发送示例
//!
//! 使用 QQ 邮箱 SMTP 服务发送测试邮件
//!
//! 运行方式:
//! ```bash
//! cargo run --example send_test_email -- <收件人邮箱>
//! ```
//!
//! 例如:
//! ```bash
//! cargo run --example send_test_email -- test@example.com
//! ```

use lettre::{
    message::header::ContentType,
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport, Transport,
};
use std::env;

fn main() {
    // QQ 邮箱 SMTP 配置
    let smtp_host = "smtp.qq.com";
    let smtp_port: u16 = 465;
    let sender_email = "472540980@qq.com";
    let display_name = "Links";
    let password = "nihqtzichypycaac"; // QQ邮箱授权码

    // 从命令行参数获取收件人邮箱
    let args: Vec<String> = env::args().collect();
    let recipient = if args.len() > 1 {
        args[1].clone()
    } else {
        println!("用法: cargo run --example send_test_email -- <收件人邮箱>");
        println!("例如: cargo run --example send_test_email -- test@example.com");
        return;
    };

    println!("📧 邮件发送测试");
    println!("================");
    println!("SMTP 服务器: {}:{}", smtp_host, smtp_port);
    println!("发件人: {} <{}>", display_name, sender_email);
    println!("收件人: {}", recipient);
    println!();

    // 构建邮件
    let email = Message::builder()
        .from(format!("{} <{}>", display_name, sender_email).parse().unwrap())
        .to(recipient.parse().unwrap())
        .subject("Links 测试邮件")
        .header(ContentType::TEXT_PLAIN)
        .body(String::from(
            "这是一封来自 Links 信令服务器的测试邮件。\n\n\
             如果您收到此邮件，说明邮件发送功能配置正确。\n\n\
             --\n\
             Links Signaling Server",
        ))
        .expect("Failed to build email");

    // 创建 SMTP 传输（使用 SSL）
    let creds = Credentials::new(sender_email.to_string(), password.to_string());

    let mailer = SmtpTransport::relay(smtp_host)
        .expect("Failed to create SMTP transport")
        .port(smtp_port)
        .credentials(creds)
        .build();

    // 发送邮件
    println!("正在发送邮件...");
    match mailer.send(&email) {
        Ok(_) => {
            println!("✅ 邮件发送成功！");
            println!("请检查收件人邮箱（包括垃圾邮件文件夹）");
        }
        Err(e) => {
            println!("❌ 邮件发送失败: {:?}", e);
        }
    }
}
