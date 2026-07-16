# WeRead MCP — WeChat Article Reader 🚀

[![Version](https://img.shields.io/badge/version-1.0.0-blue)]()
[![Rust](https://img.shields.io/badge/rust-2024-edition?logo=rust)](https://www.rust-lang.org/)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![License](https://img.shields.io/badge/license-MIT-green)](../LICENSE)
[![CI](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml/badge.svg)](https://github.com/Stelquis/WeRead-MCP/actions/workflows/ci.yml)

> A Rust-based MCP server for reading WeChat Official Account articles.
> Pure HTTP, no browser needed, auto-downloads images, outputs structured Markdown.

[中文](../README.md) · [Report Bug](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md) · [Request Feature](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md)

---

## 📖 Introduction

**WeRead MCP** is an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server for AI clients to read WeChat Official Account articles.

Traditional solutions rely on Headless Chrome — bulky, slow to start, and easily blocked. WeRead MCP uses **pure HTTP requests** to fetch article content directly, making it lightweight and fast.

### Pipeline

```
User provides URL → HTTP fetch HTML → Parse to Markdown → Download images → Save to local directory
```

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🔌 **No Browser** | Pure HTTP, ~20MB binary, instant startup |
| 📝 **Full Markdown** | Headings, bold, lists, quotes, code blocks, tables, formulas |
| 🖼️ **Local Images** | Auto-downloads all images, replaces URLs with local paths |
| 📂 **Auto Archiving** | Each article saved to `./output/<title>/` |
| 🔗 **MCP Standard** | Compatible with Claude Desktop, Codex, Cursor, OpenCode |
| 🤖 **AI Workflow** | Built-in fetch→read→polish→write pipeline instructions |

---

## 📦 Installation

### Prerequisites

- **Rust toolchain** (if not installed):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **OpenSSL dev libraries**:
  - Ubuntu/Debian: `apt install libssl-dev pkg-config`
  - macOS: built-in
  - Windows: see [openssl docs](https://docs.rs/openssl/latest/openssl/)

### Build

```bash
git clone https://github.com/Stelquis/WeRead-MCP.git
cd WeRead-MCP
cargo build --release
```

Binary: `./target/release/weread-mcp`

---

## 🚀 Quick Start

### 1. Configure MCP Client

Add the following to your AI client's MCP configuration:

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/absolute/path/to/weread-mcp"
    }
  }
}
```

**Claude Code / Codex** (`.mcp.json` in project root):

```json
{
  "mcpServers": {
    "weixin-reader": {
      "command": "/absolute/path/to/weread-mcp"
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
├── article.md         ← Structured Markdown (with local image paths)
└── images/            ← Local image files
    ├── image_0.jpg
    ├── image_1.png
    └── ...
```

---

## 🏗️ Architecture

```
AI Client (Claude Desktop / Codex / Cursor)
    ↕ stdio JSON-RPC (MCP protocol)
weread-mcp
├── main.rs      → Entry point, logging, MCP service startup
├── server.rs    → Tool registration, URL validation, AI workflow prompts
├── scraper.rs   → HTTP requests, image download, local file output
├── parser.rs    → HTML parsing, Markdown/table/formula conversion
└── error.rs     → Unified error types
          ↕ HTTP
   mp.weixin.qq.com
```

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `main.rs` | Entry point, initialize tracing, start MCP service via stdio |
| `server.rs` | Register `read_weixin_article` tool, validate URL, build response, manage output directory |
| `scraper.rs` | HTTP client wrapper, fetch HTML, download images, write files |
| `parser.rs` | Parse HTML with cssparser + scraper, extract title/author/content, convert to Markdown |
| `error.rs` | Unified `AppError` type with `Display` and `Error` traits |

---

## 🛠️ API Reference

### `read_weixin_article`

Read a WeChat Official Account article.

**Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | `string` | ✅ | WeChat article URL, must start with `https://mp.weixin.qq.com/s/` |

**Response:**

```json
{
  "success": true,
  "title": "Article Title",
  "author": "Author Name",
  "publish_time": "2024-01-01",
  "content": "Plain text body (no HTML tags)",
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

## 🧪 Local Testing

```bash
# Build
cargo build --release

# Run MCP protocol test (edit URL in test_mcp.py first)
python3 test_mcp.py

# Or test via stdio directly
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | ./target/release/weread-mcp
```

---

## 📖 About MCP

MCP (Model Context Protocol) is an **open protocol** led by Anthropic for standardized AI tool invocation. Supported clients:

| Client | Configuration |
|--------|---------------|
| 🖥️ **Claude Desktop** | Settings → Developer → Edit Config → `claude_desktop_config.json` |
| ⌨️ **Claude Code / Codex** | Project root `.mcp.json` or `~/.claude/settings.json` |
| 🖱️ **Cursor** | Settings → MCP → Add New MCP Server |
| 🔧 **OpenCode** | Project root `.mcp.json` |

---

## 🔧 Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| MCP unresponsive | stdout polluted by log output | Logs go to stderr; check for stray `println!` statements |
| Empty article content | WeChat anti-scraping | Reduce request frequency, verify URL validity |
| Some images not downloaded | Invalid or empty image URLs | Auto-skipped, does not affect article body |
| Build error | Missing OpenSSL or outdated Rust | Install `libssl-dev`, run `rustup update` |

---

## 📄 License

MIT © 2026 Stelquis