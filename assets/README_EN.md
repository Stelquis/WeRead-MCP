# 微信文章阅读 MCP 🚀

[![Version](https://img.shields.io/badge/version-1.0.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![License](https://img.shields.io/badge/license-MIT-green)](../LICENSE)
[![CI](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml/badge.svg)](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml)

> 基于 Rust 的 MCP 服务器，用于读取微信公众号文章。
> 纯 HTTP 请求，无需浏览器，自动下载图片到本地，输出结构化 Markdown。

[中文](../README.md) · [报告 Bug](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md) · [提出功能](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md)

---

## 📦 安装

### 前置条件

- Rust 工具链（`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`）
- OpenSSL 开发库（Ubuntu: `apt install libssl-dev`，macOS: 已内置）

### 编译

```bash
git clone https://github.com/Stelquis/WeRead-MCP.git
cd WeRead-MCP
cargo build --release
```

编译产物位于 `./target/release/weixin-mcp-rs`。

---

## 🚀 快速开始

### 1. 配置 MCP 客户端

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/path/to/weixin-mcp-rs"
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
├── article.md         ← 结构化 Markdown
└── images/            ← 本地图片
    ├── image_0.jpg
    └── ...
```

---

## ✨ 功能特性

| 特性 | 说明 |
|------|------|
| 🔌 **无需浏览器** | 纯 HTTP 请求，二进制 ~20MB，启动毫秒级 |
| 📝 **完整 Markdown** | 标题、粗体、列表、引用、代码块、表格、公式全部保留 |
| 🖼️ **图片本地化** | 自动下载图片，替换引用为本地路径 |
| 📂 **自动归档** | 每篇文章独立保存到 `./output/<标题>/` |
| 🔗 **MCP 标准** | 兼容 Claude Desktop / Codex / Cursor / OpenCode |
| 🤖 **AI 工作流** | 内置爬取→通读→润色→写回流程指引 |

---

## 🏗️ 架构

```
AI Client (Claude Desktop / Codex / Cursor)
    ↕ stdio JSON-RPC (MCP 协议)
main.rs → server.rs → scraper.rs → parser.rs
                          ↕ HTTP
                   mp.weixin.qq.com
```

| 模块 | 职责 |
|------|------|
| `main.rs` | 入口，初始化日志，启动 MCP 服务 |
| `server.rs` | MCP 工具注册，URL 校验，AI 工作流提示词 |
| `scraper.rs` | HTTP 请求 + 图片下载 + 本地文件写入 |
| `parser.rs` | HTML 解析 + Markdown/表格/公式转换 |
| `error.rs` | 统一错误类型 |

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
  "content": "纯文本正文",
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

## 📖 关于 MCP

MCP (Model Context Protocol) 是由 **Anthropic 主导的开放协议**，为 AI 模型提供标准化的工具调用接口。目前支持的客户端：

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
| MCP 启动后无响应 | stdout 被日志污染 | 日志已配置输出到 stderr，检查是否有其他 print 语句 |
| 文章内容为空 | 微信反爬虫 | 降低请求频率，检查 URL 是否有效 |
| 图片下载失败 | 部分图片 URL 为空或格式异常 | 已自动跳过，不影响文章正文 |
| 编译报错 | 缺少 OpenSSL 或 Rust 版本过低 | 安装 `libssl-dev`，运行 `rustup update` |

---

## 📄 许可证

MIT © 2026 Wechat Read MCP