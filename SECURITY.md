# Security Policy

## Supported versions

| version | supported |
|---------|-----------|
| 0.1.x   | ✅ |

## Reporting a vulnerability

Open a **private** GitHub security advisory (repository → Security →
Report a vulnerability) rather than a public issue. You'll get a response
within 7 days.

## Key handling — what naysay does

- API keys are stored in the OS credential store (Windows Credential
  Manager / macOS Keychain / Linux Secret Service) via the `keyring`
  crate, or provided through an environment variable for scripted use.
- Keys are sent only to the chat endpoint configured in `naysay.toml`.
- Keys are never logged. Session logs (`sessions/*.jsonl`) contain
  conversation text only — but **conversation text can contain secrets if
  you paste them**, so treat session files as sensitive.
- Panic logs (`panic.log`) and debug logs (`session.log`) contain no key
  material.

## If you leaked a key while using naysay

1. Revoke it at your provider immediately.
2. `naysay key delete` removes the copy from the OS keyring.
3. Check `%LOCALAPPDATA%\naysay\sessions\` (or the platform equivalent)
   for session logs that may contain the pasted key, and delete them.

## Scope

The attack surface is intentionally small: one binary, one HTTP client
talking to a user-configured endpoint, local file reads for `@path`
inlining and `explain`. File reads follow the path you type — naysay
makes no attempt to sandbox them, so don't inline directories containing
secrets.
