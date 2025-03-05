/*!
# Chat App 主入口

本程序支持两种模式运行：
- **服务器模式**（server）：启动服务器，监听并处理所有客户端连接
- **客户端模式**（client）：启动客户端，连接服务器后进行消息交互

使用方法：
```sh
# 启动服务器
cargo run -- server

# 启动客户端（可在第二个参数传入用户名，否则提示输入）
cargo run -- client Alice
详细实现请参见各模块的文档注释。 */

use chat::{Task, TaskType};
use std::env;
use std::io::{self, Write};

#[tokio::main]
async fn main() {
    // 从命令行参数获取运行模式
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("请指定运行模式: server 或 client");
        return;
    }
    let mode = args[1].as_str();
    match Task::from_string(mode) {
        Some(TaskType::Server) => {
            println!("启动服务器模式...");
            let addr = if args.len() >= 3 {
                args[2].clone()
            } else {
                String::from("0.0.0.0:7891")
            };

            let server = chat::server::Server::new();
            if let Err(e) = server.run(&addr).await {
                eprintln!("服务器运行出错: {:?}", e);
            }
        }
        Some(TaskType::Client) => {
            println!("启动客户端模式...");
            // 如果命令行传入了服务器IP地址，则使用；否则默认使用本地地址
            let addr = if args.len() >= 3 {
                args[2].clone()
            } else {
                // 默认通过回环地址，链接本地服务器。
                String::from("127.0.0.1:7891")
            };

            print!("请输入用户名 >> ");
            let mut input = String::new();
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut input).unwrap();
            let username = input.trim().to_string();

            let client = chat::client::Client::new(username);
            if let Err(e) = client.run(addr).await {
                eprintln!("客户端运行出错: {:?}", e);
            }
        }
        None => {
            eprintln!("无效的模式，请使用 server 或 client");
        }
    }
}
