/*!
# 客户端模块

本模块实现了聊天客户端功能，支持：
- 连接服务器并注册（发送用户名）
- 启动独立任务实时接收服务器转发的消息
- 主循环中读取用户输入，构造消息并发送到服务器
- 支持退出（输入 "exit" 即退出程序）

详细说明请参见各函数注释。
*/

use crate::{ArcString, Message};
use colored::*;
use serde_json;
use std::io::{self, Write};
use std::net::SocketAddr;
use std::process;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::spawn;

/// 聊天客户端结构体
#[derive(Debug)]
pub struct Client {
    /// 客户端用户名，使用 `ArcString` 封装以避免重复克隆
    name: ArcString,
}

impl Client {
    /// 创建新的客户端实例
    ///
    /// # 参数
    /// - `name`: 客户端用户名
    ///
    /// # 返回值
    /// 返回 `Client` 实例
    pub fn new(name: String) -> Self {
        Self {
            name: ArcString::new(name.trim().to_string()),
        }
    }

    /// 启动客户端：连接服务器、注册用户、并同时处理发送和接收消息
    pub async fn run(&self, addr: String) -> Result<(), Box<dyn std::error::Error>> {
        // 强制转换为IPv4地址
        let addr: SocketAddr = addr.parse()?;
        // 连接到服务器
        let stream = TcpStream::connect(addr).await?;
        println!("{}", "成功连接到服务器".green().bold());

        // 使用 split 分离读写任务
        let (mut reader, mut writer) = stream.into_split();

        // 发送注册消息，内容为用户名字符串
        let reg_msg = self.name.get();
        writer.write_all(reg_msg.as_bytes()).await?;

        // 启动接收任务，处理来自服务器转发的消息
        let _recv_task = spawn(async move {
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf).await {
                    Ok(0) => {
                        print!("\r\x1b[K"); // \r 回到行首，\x1b[K 清除该行

                        println!("{}", "服务器关闭了连接".red().bold());
                        process::exit(1);
                    }
                    Ok(n) => {
                        let json_str = String::from_utf8_lossy(&buf[..n]);
                        match serde_json::from_str::<Message>(&json_str) {
                            Ok(message) => {
                                // **清除当前输入行并刷新终端**
                                print!("\r\x1b[K"); // \r 回到行首，\x1b[K 清除该行
                                                    // 打印接收到的消息（显示发送者和内容）
                                println!(
                                    "\n[{}] {}: {}",
                                    message.time_stamp().bright_black(),
                                    message.from().cyan().bold(),
                                    message.content().yellow()
                                );

                                // **重新显示输入提示**
                                print!("{}", "请输入接收方: ".cyan().bold());
                                io::stdout().flush().unwrap();
                            }
                            Err(e) => {
                                eprintln!("{}: {:?}", "解析服务器消息失败".red().bold(), e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("{}: {:?}", "读取服务器消息失败".red().bold(), e);
                        break;
                    }
                }
            }
        });

        // 主循环：交互式读取用户输入并发送消息
        loop {
            // 提示输入目标接收方
            print!("{}", "请输入接收方: ".cyan().bold());
            io::stdout().flush()?;
            let mut recipient = String::new();
            io::stdin().read_line(&mut recipient)?;
            let recipient = recipient.trim().to_string();
            let mut content = String::new();

            if recipient == "/exit" {
                println!("{}", "再见！感谢使用 ChatApp!".green().bold());
                process::exit(0);
            } else if recipient == reg_msg {
                println!("{}", "无法发送消息给自己".yellow().bold());
                continue;
            } else if recipient == "/list" {
                content = String::from("");
            } else {
                // 提示输入消息内容
                print!("{}", "请输入消息内容: ".purple().bold());
                io::stdout().flush()?;
                io::stdin().read_line(&mut content)?;
            }

            // 构造消息对象，from 为自身用户名，to 为用户输入的接收方
            let msg = Message::new(
                self.name.clone(),
                recipient.trim().to_string(),
                content.trim().to_string(),
            );
            let json_msg = serde_json::to_string(&msg)?;
            // 将消息发送到服务器
            if let Err(e) = writer.write_all(json_msg.as_bytes()).await {
                eprintln!("发送消息失败: {:?}", e);
                return Ok(());
            }
        }

        // 一般不会到达此处，等待接收任务结束
        //recv_task.await?;
        //Ok(())
    }
}
