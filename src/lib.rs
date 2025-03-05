/*!
# Chat App Library

该库提供了一个简单聊天应用所需的基础类型和工具，包含：

- **ArcString**
  封装 `Arc<String>`，用于避免在多处使用时重复克隆 `String`，提升性能。

- **Message**
  聊天消息结构体，包含发送者、接收者、时间戳和消息内容，支持序列化与反序列化。

- **Task** 与 **TaskType**
  用于区分运行模式（服务器或客户端）。

详细文档请参见各结构体和函数的注释。
*/

use chrono::Local;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// `ArcString` 封装了 `Arc<String>`，用于高效共享字符串，避免不必要的克隆。
///
/// 由于 `ArcString` 内部存储的是 `Arc<String>`，它本身不能直接用作 `HashMap` 或 `DashMap` 的键，
/// 因此需要为其实现 `Hash` 特征，使其能够基于内部 `String` 进行哈希计算。
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ArcString(Arc<String>);

impl ArcString {
    /// 创建一个新的 `ArcString` 实例
    ///
    /// # 参数
    /// - `s`: 要包装的字符串
    ///
    /// # 返回值
    /// - 返回封装后的 `ArcString`
    pub fn new(s: String) -> Self {
        ArcString(Arc::new(s))
    }

    /// 获取内部字符串的克隆
    ///
    /// # 返回值
    /// - 返回 `String` 类型
    pub fn get(&self) -> String {
        self.0.to_string()
    }
}

/// 为 `ArcString` 实现 `Hash` 特征，使其能够作为 `HashMap` 和 `DashMap` 的键。
///
/// 由于 `ArcString` 内部存储的是 `Arc<String>`，而 `Arc<T>` 本身并未实现 `Hash`，
/// 因此这里手动实现 `Hash`，并确保哈希值计算仅基于内部的 `String` 内容。
impl Hash for ArcString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

/// 为 `ArcString` 实现序列化，直接序列化内部字符串引用
impl Serialize for ArcString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// 为 `ArcString` 实现反序列化，将得到的 `String` 包装到 `Arc` 中
impl<'de> Deserialize<'de> for ArcString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(ArcString(Arc::new(s)))
    }
}

impl fmt::Display for ArcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 表示一条聊天消息，包含发送者、接收者、时间戳和内容
#[derive(Deserialize, Serialize)]
pub struct Message {
    from: ArcString,
    to: String,
    time_stamp: String,
    content: String,
}

impl Message {
    /// 创建一条新的消息，自动生成当前时间戳
    ///
    /// # 参数
    /// - `from`: 发送者（封装为 `ArcString`）
    /// - `to`: 接收者
    /// - `content`: 消息内容
    ///
    /// # 返回值
    /// 返回一个 `Message` 实例
    pub fn new(from: ArcString, to: String, content: String) -> Message {
        Message {
            from,
            to,
            content,
            time_stamp: Local::now().format("%H:%M:%S").to_string(),
        }
    }

    /// 获取发送者信息（只读）
    pub fn from(&self) -> &str {
        &self.from.0
    }

    /// 获取接收者信息（只读）
    pub fn to(&self) -> &str {
        &self.to
    }

    /// 获取消息的时间戳（只读）
    pub fn time_stamp(&self) -> &str {
        &self.time_stamp
    }

    /// 获取消息内容（只读）
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// 定义任务类型，用于指定运行模式（服务器或客户端）
#[derive(Debug)]
pub enum TaskType {
    Server,
    Client,
}

/// 辅助类型，用于从字符串转换为 `TaskType`
#[derive(Debug)]
pub struct Task {}

impl Task {
    /// 根据输入字符串返回对应的任务类型
    ///
    /// # 参数
    /// - `task`: 输入字符串（"server" 或 "client"）
    ///
    /// # 返回值
    /// 若匹配成功，返回对应的 `TaskType`，否则返回 `None`
    pub fn from_string(task: &str) -> Option<TaskType> {
        match task.to_lowercase().as_str() {
            "server" => Some(TaskType::Server),
            "client" => Some(TaskType::Client),
            _ => None,
        }
    }
}

/// 声明 client 模块
pub mod client;
/// 声明 server 模块
pub mod server;
