/*!
# 服务器模块

本模块实现了聊天服务器，支持：
- 客户端注册（通过发送用户名）
- 异步消息接收与转发（利用 mpsc 通道解耦读写）
- 根据消息中的目标接收者查找在线用户，并将消息转发至对应客户端
- 当目标用户不在线时，返回提示信息给发送者

详细实现请参见各函数注释。
*/

use crate::{ArcString, Message};
use dashmap::DashMap;
use serde_json;
use std::process;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{tcp::OwnedReadHalf, TcpListener, TcpStream};
use tokio::sync::mpsc;

type ReadStream<'a> = &'a mut OwnedReadHalf;

/// 服务器结构体，管理所有在线用户及其消息发送通道
#[derive(Debug)]
pub struct Server {
    /// 在线用户映射：键为用户名（ArcString），值为对应的 mpsc 发送者
    online_users: Arc<DashMap<ArcString, mpsc::Sender<Message>>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
impl Server {
    /// 创建一个新的 `Server` 实例
    pub fn new() -> Self {
        Self {
            online_users: Arc::new(DashMap::new()),
        }
    }

    /// 启动服务器，监听指定地址，并处理所有新连接
    pub async fn run(&self, addr: &String) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(addr).await?;
        println!("服务器正在监听 {}", addr);

        let server = self.clone();

        let _shutdown_task = tokio::spawn(async move {
            tokio::signal::ctrl_c().await.expect("无法监听 Ctrl+C");
            println!("\n接收到 Ctrl+C，正在关闭服务器...");

            // **通知所有在线用户**
            let users = server.online_users.iter();
            for entry in users {
                let username = entry.key();
                let sender = entry.value();
                let notify_msg = Message::new(
                    ArcString::new("Server".to_string()),
                    username.get(),
                    "服务器即将关闭，所有用户已断开连接".to_string(),
                );
                let _ = sender.send(notify_msg).await;
            }

            // **清空在线用户列表**
            server.online_users.clear();
            println!("所有用户连接已释放，服务器退出。");
            process::exit(0);
        });
        loop {
            // 异步接受新连接
            match listener.accept().await {
                Ok((stream, addr)) => {
                    println!("接收到来自 {} 的新连接", addr);
                    // 克隆当前 Server 实例（低成本克隆内部 Arc）
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            eprintln!("处理来自 {} 的连接时出错: {:?}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("接受连接失败: {:?}", e);
                }
            }
        }
    }

    async fn handle_connection(
        &self,
        mut stream: TcpStream,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];

        // 读取客户端的注册信息（用户名）
        let len = stream.read(&mut buf).await?;
        if len == 0 {
            return Ok(());
        }

        let name = String::from_utf8_lossy(&buf[..len]).trim().to_string();
        let username = ArcString::new(name);

        // 创建 `mpsc` 通道用于消息转发
        let (tx, mut rx) = mpsc::channel::<Message>(10);
        self.online_users.insert(username.clone(), tx);
        println!("用户 {} 已注册", username.get());

        // **解决方法：使用 `into_split()` 分割 `TcpStream`**
        let (mut reader, mut writer) = stream.into_split();

        // **写任务（发送消息给客户端）**
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Ok(json_msg) = serde_json::to_string(&msg) {
                    if let Err(e) = writer.write_all(json_msg.as_bytes()).await {
                        eprintln!("发送消息失败: {:?}", e);
                        break;
                    }
                } else {
                    eprintln!("消息序列化失败");
                }
            }
        });

        // **主任务（接收客户端消息并处理）**
        self.handle_receive(username, &mut reader).await
    }

    /// 处理客户端连接中的消息接收，根据消息转发逻辑进行处理
    async fn handle_receive<'a>(
        &self,
        username: ArcString,
        stream: ReadStream<'a>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buf = [0u8; 1024];

        loop {
            let len = stream.read(&mut buf).await?;
            if len == 0 {
                // 客户端关闭连接
                break;
            }

            // 将读取的数据转换为字符串，并尝试解析为 Message
            let json_msg = String::from_utf8_lossy(&buf[..len]);
            match serde_json::from_str::<Message>(&json_msg) {
                Ok(msg) => {
                    println!(
                        "[{}] {} 发送消息给 {}: {}",
                        msg.time_stamp(),
                        msg.from(),
                        msg.to(),
                        msg.content()
                    );

                    if msg.to == "/list" {
                        let online_list: Vec<String> = self
                            .online_users
                            .iter()
                            .map(|entry| entry.key().get())
                            .collect();
                        // 构造美观的响应消息
                        let response = if online_list.is_empty() {
                            "当前无其他在线用户".to_string()
                        } else {
                            format!(
                                "当前在线用户 (共{}人):\n  › {}",
                                online_list.len(),
                                online_list.join("\n  › ") // 用箭头符号美化列表
                            )
                        };

                        // 发送给请求者（原消息发送者）
                        if let Some(sender_tx) = self.online_users.get(&username) {
                            let list_msg = Message::new(
                                ArcString::new("Server".to_string()),
                                username.get(),
                                response, // 使用格式化后的内容
                            );
                            let _ = sender_tx.send(list_msg).await;
                        }
                        continue; // 跳过后续转发逻辑
                    }
                    // 构造目标用户名的 ArcString
                    let recipient = ArcString::new(msg.to().to_string());
                    // 查找目标用户的发送者
                    if let Some(tx) = self.online_users.get(&recipient) {
                        // 将消息发送给目标用户
                        let _ = tx.send(msg).await;
                    } else {
                        // 若目标用户不在线，给发送者返回提示信息
                        if let Some(sender_tx) = self.online_users.get(&username) {
                            let tip = Message::new(
                                ArcString::new("Server".to_string()),
                                username.get(),
                                format!("用户 {} 不在线", msg.to()),
                            );
                            let _ = sender_tx.send(tip).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("解析 JSON 消息失败: {:?}", e);
                }
            }
        }

        println!("用户 {} 断开连接", username.get());
        self.online_users.remove(&username);
        Ok(())
    }
}

/// 为了在任务中低成本克隆 Server，手动实现 Clone（只克隆内部 Arc）
impl Clone for Server {
    fn clone(&self) -> Self {
        Server {
            online_users: Arc::clone(&self.online_users),
        }
    }
}
