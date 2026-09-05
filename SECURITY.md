# Security Policy / 安全策略

**English first · 中文在后**

[English](#english) · [中文](#中文)

---

<a id="english"></a>

## English

### Supported versions

| version | supported |
|---------|-----------|
| 0.1.x   | ✅ |

### Reporting a vulnerability

Open a **private** GitHub security advisory (repository → Security →
Report a vulnerability) rather than a public issue. You'll get a
response within 7 days.

### Key handling — what naysay does

- API keys are stored in the OS credential store (Windows Credential
  Manager / macOS Keychain / Linux Secret Service) via the `keyring`
  crate, or provided through an environment variable for scripted use.
- Keys are sent only to the chat endpoint configured in `naysay.toml`.
- Keys are never logged. Session logs (`sessions/*.jsonl`) contain
  conversation text only — but **conversation text can contain secrets
  if you paste them**, so treat session files as sensitive.
- Panic logs (`panic.log`) and debug logs (`session.log`) contain no
  key material.

### If you leaked a key while using naysay

1. Revoke it at your provider immediately.
2. `naysay key delete` removes the copy from the OS keyring.
3. Check `%LOCALAPPDATA%\naysay\sessions\` (or the platform equivalent)
   for session logs that may contain the pasted key, and delete them.

### Scope

The attack surface is intentionally small: one binary, one HTTP client
talking to a user-configured endpoint, local file reads for `@path`
inlining and `explain`. File reads follow the path you type — naysay
makes no attempt to sandbox them, so don't inline directories
containing secrets.

---

<a id="中文"></a>

## 中文

### 受支持版本

| 版本 | 支持状态 |
|------|----------|
| 0.1.x | ✅ |

### 报告漏洞

请开 **private** 的 GitHub security advisory(仓库 → Security →
Report a vulnerability),不要公开 issue。7 天内必有回复。

### 密钥处理 — naysay 做了什么

- API key 通过 `keyring` crate 存在系统凭据库(Windows Credential
  Manager / macOS Keychain / Linux Secret Service),或通过环境变量
  在脚本化场景使用。
- key 只发往 `naysay.toml` 里配置的 chat endpoint。
- key 永远不会被日志化。Session 日志(`sessions/*.jsonl`)只含对话
  文本——**但你自己粘贴进对话的也算**,所以 session 文件按敏感
  数据对待。
- Panic 日志(`panic.log`)和调试日志(`session.log`)不含 key。

### 如果用 naysay 时泄漏了 key

1. 立即去你的 provider 撤销它。
2. `naysay key delete` 把 OS keyring 里的副本删掉。
3. 检查 `%LOCALAPPDATA%\naysay\sessions\`(或其它平台等价路径)看
   session 日志里有没有粘过那个 key,删掉对应文件。

### 攻击面

刻意保持小:一个二进制,一个 HTTP 客户端跟你配置的 endpoint 通信,
本地读 `@path` 内联和 `explain`。文件读取走你敲的路径——naysay
不做沙箱,所以别把含密钥的目录用 `@dir` 引入。
