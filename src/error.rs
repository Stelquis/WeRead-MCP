//! 统一错误类型定义

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    /// HTTP 请求失败（网络错误、连接超时等）
    #[error("网络请求失败: {0}")]
    Network(String),

    /// HTTP 返回非 200 状态码
    #[error("HTTP {status}: {message}")]
    HttpStatus { status: u16, message: String },

    /// 请求超时
    #[error("请求超时: {0}")]
    Timeout(String),

    /// 读取响应体失败
    #[error("响应读取失败: {0}")]
    ResponseRead(String),

    /// 文件 I/O 操作失败（读写目录、保存图片等）
    #[error("IO 错误: {0}")]
    Io(String),
}
