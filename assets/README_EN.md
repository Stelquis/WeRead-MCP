


# Wechat Article Read MCP 🚀

[![Version](https://img.shields.io/badge/version-1.0.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![CI](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml/badge.svg)](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml)

> A Rust-based MCP server for reading WeChat Official Account articles.
> Pure HTTP requests, no browser needed, auto-downloads images, outputs structured Markdown.

[中文](../README.md) · [Report Bug](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md) · [Request Feature](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md)

---

## 📦 Installation

### Prerequisites

- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- OpenSSL dev libraries (Ubuntu: `apt install libssl-dev`, macOS: built-in)

### Build

```bash
git clone https://github.com/Stelquis/WeRead-MCP.git
cd WeRead-MCP
cargo build --release
```

Binary: `./target/release/weixin-mcp-rs`

---

## 🚀 Quick Start

### 1. Configure MCP Client

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/path/to/weixin-mcp-rs"
    }
  }
}
```

### 2. Call the Tool

Invoke `read_weixin_article` with a WeChat article URL:

```json
{
  "url": "https://mp.weixin.qq.com/s/xxx"
}
```

### 3. View Output

```
./output/<article-title>/
├── article.md         ← Structured Markdown
└── images/            ← Local images
    ├── image_0.jpg
    └── ...
```

---

## ✨ Features

| Feature                    | Description                                                  |
| -------------------------- | ------------------------------------------------------------ |
| 🔌**No Browser**     | Pure HTTP, ~20MB binary, instant startup                     |
| 📝**Full Markdown**  | Headings, bold, lists, quotes, code blocks, tables, formulas |
| 🖼️**Local Images** | Auto-downloads images, replaces URLs with local paths        |
| 📂**Auto Archiving** | Each article saved to`./output/<title>/`                   |
| 🔗**MCP Standard**   | Compatible with Claude Desktop / Codex / Cursor / OpenCode   |
| 🤖**AI Workflow**    | Built-in fetch→read→polish→write pipeline                 |

---

## 🏗️ Architecture

```
AI Client (Claude Desktop / Codex / Cursor)
    ↕ stdio JSON-RPC (MCP protocol)
main.rs → server.rs → scraper.rs → parser.rs
                          ↕ HTTP
                   mp.weixin.qq.com
```

| Module         | Responsibility                                                  |
| -------------- | --------------------------------------------------------------- |
| `main.rs`    | Entry point, logging, MCP service startup                       |
| `server.rs`  | MCP tool registration, URL validation, AI workflow instructions |
| `scraper.rs` | HTTP requests, image download, local file output                |
| `parser.rs`  | HTML parsing, Markdown/table/formula conversion                 |
| `error.rs`   | Unified error types                                             |

---

## 🛠️ API Reference

### `read_weixin_article`

Read a WeChat Official Account article.

**Parameters:**

| Param   | Type       | Required | Description                                                        |
| ------- | ---------- | -------- | ------------------------------------------------------------------ |
| `url` | `string` | ✅       | WeChat article URL, must start with`https://mp.weixin.qq.com/s/` |

**Response:**

```json
{
  "success": true,
  "title": "Article Title",
  "author": "Author Name",
  "publish_time": "2024-01-01",
  "content": "Plain text body",
  "content_markdown": "Markdown body (with local image paths)",
  "images": ["https://mmbiz.qpic.cn/..."],
  "output": {
    "success": true,
    "markdown_path": "./output/article-title/article.md",
    "images_dir": "./output/article-title/images",
    "downloaded_images": ["image_0.jpg", "image_1.png"]
  }
}
```

---

## 📖 About MCP

MCP (Model Context Protocol) is an **open protocol** led by Anthropic for standardized AI tool invocation. Supported clients:

| Client                            | Configuration                                                         |
| --------------------------------- | --------------------------------------------------------------------- |
| 🖥️**Claude Desktop**      | Settings → Developer → Edit Config →`claude_desktop_config.json` |
| ⌨️**Claude Code / Codex** | Project root`.mcp.json` or `~/.claude/settings.json`              |
| 🖱️**Cursor**              | Settings → MCP → Add New MCP Server                                 |
| 🔧**OpenCode**              | Project root`.mcp.json`                                             |

---

## 🔧 Troubleshooting

| Issue                 | Cause                            | Solution                                            |
| --------------------- | -------------------------------- | --------------------------------------------------- |
| MCP unresponsive      | stdout polluted by log output    | Logs go to stderr; check for stray print statements |
| Empty article content | WeChat anti-scraping             | Reduce request frequency, verify URL validity       |
| Image download failed | Invalid or empty image URLs      | Auto-skipped, does not affect article body          |
| Build error           | Missing OpenSSL or outdated Rust | Install`libssl-dev`, run `rustup update`        |

---

## 📄 License

MIT © 2026 Wechat Read MCP