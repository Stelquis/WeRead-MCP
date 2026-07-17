#!/usr/bin/env python3
"""MCP stdio (NDJSON) client for testing weixin-mcp-rs

Usage:
    python3 test_mcp.py <URL>
    python3 test_mcp.py --binary /path/to/weread-mcp <URL>

Examples:
    python3 test_mcp.py https://mp.weixin.qq.com/s/xxx
    python3 test_mcp.py --binary ./target/release/weread-mcp https://mp.weixin.qq.com/s/xxx
"""
import subprocess
import json
import sys
import os
import time
import argparse
import signal
from pathlib import Path


# 全局子进程引用，供信号处理器清理用
_proc: subprocess.Popen | None = None


def _cleanup_proc():
    """终止子进程（如果存在）"""
    global _proc
    if _proc is not None and _proc.poll() is None:
        try:
            _proc.terminate()
            _proc.wait(timeout=5)
        except Exception:
            _proc.kill()
            _proc.wait(timeout=5)
        _proc = None


def _signal_handler(signum, frame):
    """捕获 SIGINT/SIGTERM，清理子进程后退出"""
    print(f"\n⚠️  收到信号 {signum}，正在清理...")
    _cleanup_proc()
    sys.exit(128 + signum)


def send_msg(proc, msg):
    """Send a JSON-RPC message as a single line (NDJSON format)"""
    line = json.dumps(msg, ensure_ascii=False) + "\n"
    proc.stdin.write(line)
    proc.stdin.flush()


def read_msg(proc, timeout=300):
    """Read one JSON-RPC response line"""
    start = time.time()
    line = ""
    while time.time() - start < timeout:
        ch = proc.stdout.read(1)
        if not ch:
            raise EOFError("stdin closed")
        line += ch
        if ch == "\n":
            break
    else:
        raise TimeoutError("timeout")

    line = line.strip()
    if not line:
        return None
    return json.loads(line)


def parse_args():
    parser = argparse.ArgumentParser(
        description="WeRead MCP 协议测试工具 — 调用 MCP 服务器爬取微信文章",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
示例:
  %(prog)s https://mp.weixin.qq.com/s/xxx
  %(prog)s --binary ./target/release/weread-mcp https://mp.weixin.qq.com/s/xxx
  %(prog)s --binary ./target/release/weread-mcp --output ./my-output https://mp.weixin.qq.com/s/xxx
        """,
    )
    parser.add_argument(
        "--binary",
        default=os.environ.get(
            "WEREAD_MCP_BINARY",
            "/workspace/Repo/WeRead-MCP/target/release/weread-mcp",
        ),
        help="MCP 服务器二进制路径（默认: %(default)s，也可通过环境变量 WEREAD_MCP_BINARY 设置）",
    )
    parser.add_argument(
        "--output",
        default=os.environ.get("WEREAD_MCP_OUTPUT", ""),
        help="结果 JSON 输出路径（默认: 不保存文件）",
    )
    parser.add_argument(
        "url",
        nargs="?",
        default="https://mp.weixin.qq.com/s/wm_LM83gyLM-auidBxprZw",
        help="微信文章 URL（默认: 内置测试 URL）",
    )
    return parser.parse_args()


def main():
    args = parse_args()
    binary = args.binary
    url = args.url
    output_path = args.output

    # 注册信号处理器，确保异常退出时清理子进程
    signal.signal(signal.SIGINT, _signal_handler)
    signal.signal(signal.SIGTERM, _signal_handler)

    print("=" * 60)
    print(f"🚀 WeRead MCP 测试工具")
    print(f"   Binary: {binary}")
    print(f"   URL:    {url}")
    print("=" * 60)

    # 检查二进制是否存在
    if not os.path.isfile(binary):
        print(f"❌ 二进制文件不存在: {binary}")
        print("   请先编译: cargo build --release")
        sys.exit(1)

    global _proc
    _proc = subprocess.Popen(
        [binary],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )

    time.sleep(0.5)
    if _proc.poll() is not None:
        print(f"❌ 服务器启动失败! Exit: {_proc.returncode}")
        print("Stderr:", _proc.stderr.read()[:2000])
        _cleanup_proc()
        sys.exit(1)

    try:
        # Step 1: Initialize
        print("1. 初始化 MCP 连接...")
        send_msg(_proc, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "test-client", "version": "1.0"}
            }
        })
        resp = read_msg(_proc, timeout=10)
        if resp:
            print(f"   ✅ 已连接: {resp['result']['serverInfo']['name']}")
        else:
            print("   ❌ 无响应")
            _cleanup_proc()
            sys.exit(1)

        # Step 2: Initialized notification
        print("2. 发送初始化通知...")
        send_msg(_proc, {"jsonrpc": "2.0", "method": "notifications/initialized"})

        # Step 3: tools/list
        print("3. 获取工具列表...")
        send_msg(_proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        })
        resp = read_msg(_proc, timeout=10)
        tools = resp.get("result", {}).get("tools", []) if resp else []
        tool_names = [t["name"] for t in tools]
        print(f"   ✅ 可用工具: {tool_names}")

        # Step 4: Call read_weixin_article
        print(f"\n4. 调用 read_weixin_article...")
        print(f"   URL: {url}")
        send_msg(_proc, {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "read_weixin_article",
                "arguments": {"url": url}
            }
        })

        print("   ⏳ 正在爬取，请稍候...")
        sys.stdout.flush()

        resp = read_msg(_proc, timeout=300)
        if not resp:
            print("   ❌ 无响应")
            _cleanup_proc()
            sys.exit(1)

        result = resp.get("result", {})
        content_list = result.get("content", [])
        if content_list and isinstance(content_list, list):
            raw_text = content_list[0].get("text", "{}")
        else:
            raw_text = str(content_list)

        try:
            data = json.loads(raw_text)
        except (json.JSONDecodeError, TypeError):
            data = {"raw": str(raw_text)[:500]}

        # Print results
        print("\n" + "=" * 60)
        print("📋 结果")
        print("=" * 60)

        title = data.get("title", "N/A")
        author = data.get("author", "N/A")
        pub_time = data.get("publish_time", "N/A")
        print(f"\n📰 标题: {title}")
        print(f"✍️  作者: {author}")
        print(f"📅 时间: {pub_time}")

        images = data.get("images", [])
        print(f"\n📸 图片 ({len(images)} 张):")
        for i, img in enumerate(images[:5], 1):
            print(f"   [{i}] {img[:90]}...")
        if len(images) > 5:
            print(f"   ... 还有 {len(images)-5} 张")

        output_info = data.get("output", {})
        if output_info.get("markdown_path"):
            print(f"\n📁 输出目录: {os.path.dirname(output_info['markdown_path'])}")
            print(f"   article.md: {output_info['markdown_path']}")
            print(f"   images: {output_info['images_dir']}")
            print(f"   下载图片: {len(output_info.get('downloaded_images', []))} 张")

        md = data.get("content_markdown", "")
        if md:
            print(f"\n📝 Markdown 正文 (前 2500 字符):")
            print("─" * 50)
            print(md[:2500])
            if len(md) > 2500:
                print(f"   ... (共 {len(md)} 字符，已截断)")

        # 保存结果到 JSON 文件
        if output_path:
            os.makedirs(os.path.dirname(output_path) or ".", exist_ok=True)
            with open(output_path, "w") as f:
                json.dump(data, f, ensure_ascii=False, indent=2)
            print(f"\n💾 结果已保存: {output_path}")
        else:
            out_dir = os.path.dirname(output_info.get("markdown_path", ""))
            if out_dir:
                json_path = os.path.join(out_dir, "result.json")
                with open(json_path, "w") as f:
                    json.dump(data, f, ensure_ascii=False, indent=2)
                print(f"\n💾 结果已保存: {json_path}")

        print("\n" + "=" * 60)
        print("✅ 完成！")
        print("=" * 60)
    finally:
        _cleanup_proc()


if __name__ == "__main__":
    main()