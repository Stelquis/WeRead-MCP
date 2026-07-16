//! WeRead MCP — 微信文章阅读器 MCP 服务
//!
//! 基于 Rust 实现，通过纯 HTTP 请求获取微信公众号文章，
//! 解析为结构化 Markdown，下载图片到本地，通过 MCP 协议返回给 AI 客户端。

mod error;
mod parser;
mod scraper;
mod server;

use rmcp::{transport::stdio, ServiceExt};
use server::WeixinServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 日志输出到 stderr，避免干扰 stdout 上的 MCP 协议通信
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("Starting WeRead MCP Server...");

    // 创建服务器实例并通过 stdio 传输启动 MCP 服务
    let service = WeixinServer::new().serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Failed to start MCP server: {}", e);
    })?;

    // 阻塞等待服务结束
    service.waiting().await?;

    tracing::info!("WeRead MCP Server stopped.");
    Ok(())
}
