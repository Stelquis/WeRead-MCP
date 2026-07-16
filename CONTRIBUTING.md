# 贡献指南

感谢你有兴趣为 WeRead MCP 贡献代码！欢迎社区的每一位参与者。

## 🐛 报告 Bug

1. 先检查是否已有相同 Issue
2. 创建 [GitHub Issue](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md)
3. 请提供：
   - Rust 版本（`rustc --version`）
   - 操作系统和环境
   - 复现步骤
   - 期望行为 vs 实际行为
   - 完整错误日志（stderr 输出）

## 💡 功能建议

创建 [Feature Request](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md)，包含：

- 清晰的特性描述
- 使用场景 / 动机
- 实现思路（可选）

## 🛠️ 开发环境

```bash
# 克隆你的 fork
git clone https://github.com/你的用户名/WeRead-MCP.git
cd WeRead-MCP

# 构建
cargo build

# 运行测试
cargo test

# 测试 MCP 协议
python3 test_mcp.py
```

## 📝 代码规范

- 提交前运行 `cargo fmt`
- 运行 `cargo clippy` 并修复警告
- 保持函数专注且有文档注释
- 使用 `tracing` 记录日志（不要用 `println!`）
- 日志只输出到 stderr（stdout 保留给 MCP 协议通信）

## 📤 提交 PR 流程

1. Fork 本仓库
2. 创建功能分支（`git checkout -b feat/你的功能`）
3. 提交变更，写明提交信息
4. 推送到你的 fork 并创建 PR
5. 确保 CI 通过
6. 等待审核

## 📄 许可证

提交贡献即表示你同意你的贡献将在 MIT 许可证下发布。