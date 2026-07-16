#!/usr/bin/env python3
"""MCP stdio (NDJSON) client for testing weixin-mcp-rs"""
import subprocess
import json
import sys
import os
import time

BINARY = "/workspace/Repo/WeRead-MCP/target/release/weread-mcp"
URL = "https://mp.weixin.qq.com/s/wm_LM83gyLM-auidBxprZw"


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


print("=" * 60)
print("Starting weixin-mcp-rs MCP server (NDJSON)...")
print("=" * 60)

proc = subprocess.Popen(
    [BINARY],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,  # line-buffered
)

time.sleep(0.5)
if proc.poll() is not None:
    print(f"Server died on startup! Exit: {proc.returncode}")
    print("Stderr:", proc.stderr.read()[:2000])
    sys.exit(1)

# Step 1: Initialize
print("1. Sending: initialize ...")
send_msg(proc, {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2025-11-25",
        "capabilities": {},
        "clientInfo": {"name": "test-client", "version": "1.0"}
    }
})

resp = read_msg(proc, timeout=10)
if resp:
    print(f"   ✅ Initialized: server={resp['result']['serverInfo']['name']}")
else:
    print("   ❌ No response"); sys.exit(1)

# Step 2: Initialized notification
print("2. Sending: notifications/initialized ...")
send_msg(proc, {
    "jsonrpc": "2.0",
    "method": "notifications/initialized"
})

# Step 3: tools/list
print("3. Sending: tools/list ...")
send_msg(proc, {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/list",
    "params": {}
})
resp = read_msg(proc, timeout=10)
tools = resp.get("result", {}).get("tools", []) if resp else []
tool_names = [t["name"] for t in tools]
print(f"   ✅ Tools: {tool_names}")

# Step 4: Call read_weixin_article
print(f"\n4. Calling: read_weixin_article ...")
print(f"   URL: {URL}")
send_msg(proc, {
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "read_weixin_article",
        "arguments": {"url": URL}
    }
})

print("   ⏳ Fetching article, please wait ...")
sys.stdout.flush()

resp = read_msg(proc, timeout=300)
if not resp:
    print("   ❌ No response from server")
    sys.exit(1)

result = resp.get("result", {})
# MCP 返回格式: content 是 [{"type":"text","text":"{...}"}]
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
print("📋 RESULTS")
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

md = data.get("content_markdown", "")
if md:
    print(f"\n📝 Markdown 正文 (前 2500 字符):")
    print("─" * 50)
    print(md[:2500])
    if len(md) > 2500:
        print(f"   ... (共 {len(md)} 字符，已截断)")

print("\n" + "=" * 60)
print("✅ DONE! Full result saved to output.json")
print("=" * 60)

os.makedirs("/workspace/output/test-mcp", exist_ok=True)
with open("/workspace/output/test-mcp/output.json", "w") as f:
    json.dump(data, f, ensure_ascii=False, indent=2)

proc.terminate()
proc.wait(timeout=5)
