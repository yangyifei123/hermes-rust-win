# V2EX / 即刻 / 掘金 — 中文版

## 标题
用 Rust 写了一个支持 22+ 大模型供应商的 AI Agent 命令行工具

## 正文

做这个工具的初衷是自己用 AI 写代码时，不想被绑在一家供应商上。

**Hermes** 是一个纯 Rust 写的 AI Agent CLI，特点：

- **22+ 供应商** — OpenAI、Claude、Gemini、DeepSeek、Groq、Ollama（本地）、智谱（GLM）、Kimi（月之暗面）、MiniMax 等一键切换
- **5MB 二进制** — 启动 <50ms，不装 Python/Node 运行时
- **工具执行** — 终端命令、文件读写、网页搜索、MCP 协议
- **会话持久化** — SQLite 存储，Ctrl+C 不丢对话，随时恢复
- **费用追踪** — 每轮显示 token 用量和费用
- **多 Key 轮转** — 同一供应商配多个 Key，自动负载均衡和故障切换

安装：

```bash
cargo install hermes-agent-cli
```

使用：

```bash
hermes auth add openai sk-...
hermes chat

# 用 DeepSeek
hermes chat --provider deepseek

# 用本地模型（Ollama，无需 API Key）
hermes chat --provider ollama

# 用智谱 GLM
hermes chat --provider zai
```

GitHub: https://github.com/yangyifei123/hermes-rust-win

MIT 协议，20K 行代码，467 个测试。欢迎 Star 和 PR。
