//! 统一错误类型定义

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    /// HTTP 请求失败（网络错误、非 200 响应等）
    #[error("HTTP error: {0}")]
    HttpError(String),

    /// 文件 I/O 操作失败（读写目录、保存图片等）
    #[error("IO error: {0}")]
    IoError(String),
}
