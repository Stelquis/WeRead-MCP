# Weixin MCP Server (weixin-mcp-rs) v1.0 🚀

微信公众号文章阅读器 — Rust 实现的 MCP (Model Context Protocol) 服务器。

---

## 📋 项目简介

通过纯 HTTP 请求获取微信公众号文章，解析为结构化 Markdown，下载图片到本地，通过 MCP 协议返回给 AI 客户端。

**核心流程：** 爬取 → 解析 → 下载图片 → 输出 Markdown → AI 润色

---

## 🛠️ 常用命令

| 命令 | 说明 |
|------|------|
| `cargo build --release` | 编译生产版本 |
| `cargo check` | 检查代码编译 |
| `cargo test` | 运行测试 |
| `python3 test_mcp.py` | 本地 MCP 协议测试 |
| `./target/release/weixin-mcp-rs` | 启动 MCP 服务 |

---

## 🏗️ 架构

```
src/
├── main.rs      — 入口：初始化日志，启动 MCP 服务
├── server.rs    — MCP 工具注册，AI 工作流提示词
├── scraper.rs   — HTTP 请求 + 图片下载 + 本地输出
├── parser.rs    — HTML → Markdown（表格/公式/图片提取）
└── error.rs     — 统一错误类型
```

---

## 📦 技术栈

| 依赖 | 版本 | 用途 |
|------|------|------|
| `rmcp` | 0.16 | Anthropic 官方 Rust MCP SDK |
| `reqwest` | 0.12 | HTTP 客户端（纯 HTTP，无需浏览器） |
| `scraper` | 0.22 | HTML 解析 + CSS 选择器 |
| `regex` | 1 | 文本清理 |
| `tokio` | 1 | 异步运行时 |
| `serde` / `schemars` | - | 序列化 + JSON Schema |

---

## ✨ 特性

- ✅ **纯 HTTP** — 无需 Chrome/Chromium 浏览器，二进制 ~20MB
- ✅ **图片本地化** — 自动下载图片，替换 Markdown 引用为本地路径
- ✅ **自动归档** — 输出到 `./output/<文章标题>/` 目录
- ✅ **MCP 标准协议** — 兼容 Claude Desktop / Codex / Cursor / OpenCode
- ✅ **AI 工作流指引** — 内置爬取→通读→润色→写回流程提示词

---

## 🔧 构建与部署

```bash
# 构建
cargo build --release

# 启动
./target/release/weixin-mcp-rs
```

> 需要 OpenSSL 开发库（Ubuntu: `apt install libssl-dev`，macOS: 已内置）