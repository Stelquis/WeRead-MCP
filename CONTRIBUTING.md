# Contributing to WeRead MCP

Thank you for your interest in contributing! We welcome contributions from the community.

## 🐛 Reporting Bugs

1. Check if the issue has already been reported
2. Open a [GitHub Issue](https://github.com/Stelquis/WeRead-MCP/issues/new?template=bug_report.md)
3. Include:
   - Rust version (`rustc --version`)
   - OS and environment
   - Steps to reproduce
   - Expected vs actual behavior
   - Full error logs (if any)

## 💡 Feature Requests

Open a [Feature Request](https://github.com/Stelquis/WeRead-MCP/issues/new?template=feature_request.md) with:

- Clear description of the feature
- Use case / motivation
- Proposed implementation (optional)

## 🛠️ Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/WeRead-MCP.git
cd WeRead-MCP

# Build
cargo build

# Run tests
cargo test

# Test MCP protocol
python3 test_mcp.py
```

## 📝 Code Style

- Run `cargo fmt` before committing
- Run `cargo clippy` and fix warnings
- Keep functions focused and well-documented
- Use `tracing` for logging (not `println!`)
- Log to stderr only (stdout is reserved for MCP protocol)

## 📤 Pull Request Process

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/your-feature`)
3. Commit your changes with clear messages
4. Push to your fork and open a PR
5. Ensure CI passes
6. Wait for review

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.