# WeRead MCP — 微信文章阅读器 🚀

[![Version](https://img.shields.io/badge/version-1.0.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![CI](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml/badge.svg)](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml)

> 基于 Rust 的 MCP 服务器，纯 HTTP 抓取微信公众号文章，自动下载图片，输出结构化 Markdown。
> 无需 Chrome 浏览器，二进制仅 ~20MB，启动毫秒级。

[English](./assets/README_EN.md) · [报告 Bug](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md) · [提出功能](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md)

---

## 📖 简介

**WeRead MCP** 是一个 [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) 服务器，专为 AI 客户端读取微信公众号文章而设计。

传统的微信文章抓取方案依赖 Headless Chrome，体积大、启动慢、容易被反爬。WeRead MCP 通过**纯 HTTP 请求**直接获取文章内容，轻量高效，毫秒级启动。

### 核心流程

```
用户提供 URL → HTTP 抓取 HTML → 解析为 Markdown → 下载图片 → 输出到本地目录
```

---

## ✨ 功能特性

| 特性 | 说明 |
|------|------|
| 🔌 **无需浏览器** | 纯 HTTP 请求，无 Chrome/Chromium 依赖，二进制 ~20MB |
| 📝 **完整 Markdown** | 标题、粗体、列表、引用、代码块、表格、公式全部保留 |
| 🖼️ **图片本地化** | 自动下载文章中所有图片，替换为本地路径 |
| 📂 **自动归档** | 每篇文章独立保存到 `./output/<标题>/` 目录 |
| 🔗 **MCP 标准协议** | 兼容 Claude Desktop、Codex、Cursor、OpenCode 等客户端 |
| 🤖 **AI 工作流** | 内置爬取→通读→润色→写回的完整流程指引 |

---

## 📦 安装

### 前置条件

- **Rust 工具链**（如未安装）：
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **OpenSSL 开发库**：
  - Ubuntu/Debian：`apt install libssl-dev pkg-config`
  - macOS：已内置
  - Windows：参考 [openssl 文档](https://docs.rs/openssl/latest/openssl/)

### 编译

```bash
git clone https://github.com/Stelquis/WeRead-MCP.git
cd WeRead-MCP
cargo build --release
```

编译产物位于 `./target/release/weread-mcp`。

---

## 🚀 快速开始

### 1. 配置 MCP 客户端

将以下配置添加到你的 AI 客户端 MCP 配置中：

**Claude Desktop**（`claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/绝对路径/to/weread-mcp"
    }
  }
}
```

**Claude Code / Codex**（项目根目录 `.mcp.json`）：

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/绝对路径/to/weread-mcp"
    }
  }
}
```

### 2. 调用工具

在 AI 客户端中调用 `read_weixin_article` 工具，传入微信文章 URL：

```json
{
  "url": "https://mp.weixin.qq.com/s/xxx"
}
```

### 3. 查看输出

```
./output/<文章标题>/
├── article.md         ← 结构化 Markdown（含本地图片路径）
└── images/            ← 本地图片文件
    ├── image_0.jpg
    ├── image_1.png
    └── ...
```

---

## 🏗️ 架构

```
AI Client (Claude Desktop / Codex / Cursor)
    ↕ stdio JSON-RPC (MCP 协议)
weread-mcp
├── main.rs      → 入口，初始化日志，启动 MCP 服务
├── server.rs    → MCP 工具注册，URL 校验，AI 工作流提示词
├── scraper.rs   → HTTP 请求 + 图片下载 + 本地文件写入
├── parser.rs    → HTML 解析 + Markdown/表格/公式转换
└── error.rs     → 统一错误类型
          ↕ HTTP
   mp.weixin.qq.com
```

### 模块职责

| 模块 | 职责 |
|------|------|
| `main.rs` | 程序入口，初始化 tracing 日志，通过 stdio 启动 MCP 服务 |
| `server.rs` | 注册 `read_weixin_article` 工具，URL 校验，构造响应，管理输出目录 |
| `scraper.rs` | 封装 HTTP 客户端，发送请求获取 HTML，下载图片，写入本地文件 |
| `parser.rs` | 使用 cssparser + scraper 解析 HTML，提取标题/作者/正文，转换 Markdown |
| `error.rs` | 统一 `AppError` 错误类型，支持 `Display` 和 `Error` trait |

---

## 🛠️ API 参考

### `read_weixin_article`

读取微信公众号文章内容。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `url` | `string` | ✅ | 微信文章链接，必须以 `https://mp.weixin.qq.com/s/` 开头 |

**响应：**

```json
{
  "success": true,
  "title": "文章标题",
  "author": "作者名",
  "publish_time": "2024-01-01",
  "content": "纯文本正文（不含 HTML 标签）",
  "content_markdown": "Markdown 格式正文（含本地图片路径）",
  "images": ["https://mmbiz.qpic.cn/..."],
  "output": {
    "success": true,
    "markdown_path": "./output/文章标题/article.md",
    "images_dir": "./output/文章标题/images",
    "downloaded_images": ["image_0.jpg", "image_1.png"]
  }
}
```

---

## 🧪 本地测试

```bash
# 编译
cargo build --release

# 运行 MCP 协议测试（需先修改 test_mcp.py 中的 URL）
python3 test_mcp.py

# 或直接通过 stdio 交互测试
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/weread-mcp
```

---

## 📖 关于 MCP

MCP (Model Context Protocol) 是由 **Anthropic 主导的开放协议**，为 AI 模型提供标准化的工具调用接口。支持的客户端：

| 客户端 | 配置方式 |
|--------|---------|
| 🖥️ **Claude Desktop** | 菜单 → Settings → Developer → Edit Config → `claude_desktop_config.json` |
| ⌨️ **Claude Code / Codex** | 项目根目录 `.mcp.json` 或 `~/.claude/settings.json` |
| 🖱️ **Cursor** | 设置 → MCP → Add New MCP Server |
| 🔧 **OpenCode** | 项目根目录 `.mcp.json` |

---

## 🔧 常见问题

| 问题 | 原因 | 解决 |
|------|------|------|
| MCP 启动后无响应 | stdout 被日志污染 | 日志已配置输出到 stderr，检查代码中是否有 `println!` |
| 文章内容为空 | 微信反爬虫机制 | 降低请求频率，检查 URL 是否有效 |
| 部分图片下载失败 | 图片 URL 为空或格式异常 | 已自动跳过，不影响文章正文 |
| 编译报错 | 缺少 OpenSSL 或 Rust 版本过低 | 安装 `libssl-dev`，运行 `rustup update` |

---

## 📄 许可证

MIT © 2026 Stelquis