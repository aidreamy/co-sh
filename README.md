# Morpheus

**English** | [Português](./docs/readme/README.pt-BR.md) | [Español](./docs/readme/README.es.md) | [Русский](./docs/readme/README.ru.md) | [中文](./docs/readme/README.zh-CN.md) | [日本語](./docs/readme/README.ja.md)

<img width="1910" height="1064" alt="image(3)" src="https://github.com/user-attachments/assets/a80fcff1-068a-44f8-b43d-3e7d7697ecd6" />

> [!WARNING]
> **WhatsApp is under development.** The WhatsApp feature (`morpheus-whatsapp`) is still experimental and is **not recommended for use** yet.

Morpheus is a general-purpose coding agent for the terminal, with software engineering tools and *computer tools* that let it interact with graphical applications.

## Why Morpheus?

Morpheus isn't meant to replace established coding agents — it's designed to work alongside them as an orchestration layer. Here's why you might want to use it:

- **Multi-agent collaboration** — Run different state-of-the-art agents (Claude, Gemini, Codex, Cline, etc.) directly within the same TUI, creating a shared workspace where they can communicate and collaborate through a single interface.
- **Experimentation-friendly** — Perfect for developers who like trying different agents and combining their strengths rather than committing to a single ecosystem.
- **Manager mode** — Keep using your existing agent harnesses while adding orchestration on top, especially useful when working on multiple smaller features or tasks simultaneously.
- **Native TUI experience** — Configure providers, models, and operating modes directly in the interface with minimal manual setup.
- **Modular architecture** — Independent, reusable crates for development tools, TUI components, SDK, and memory recall.

## How it differs from other TUI orchestrators

Morpheus stands out from other terminal-based agent orchestrators in several key ways:

| Feature | Morpheus | Other TUI Orchestrators |
|---------|-------|------------------------|
| **Multi-agent split panes** | Run multiple external agents simultaneously in split panes, each with its own session, model, and optional git worktree | Typically single-agent focused or require switching contexts |
| **Computer tools via accessibility tree** | Reads and operates graphical interfaces through structured representation, falling back to screenshots only when needed | Usually rely on screenshots or lack GUI interaction entirely |
| **Rust-based TUI** | Built from scratch with Ratatui, inspired by OpenCode's UX | Often web-based terminals or less optimized interfaces |
| **Configured in-app** | Providers, models, and modes configured directly in the TUI | Often require external config files or CLI flags |
| **Modular crates** | Development tools, TUI widgets, SDK, and memory recall as independent, reusable libraries | Monolithic architecture with tightly coupled components |
| **Shared workspace** | Multiple agents collaborate through a single interface with coordinated task dispatching | Agents typically work in isolation |

Highlights:

- **Multi-agent mode (`/agents`)** — run external agents (Gemini, Claude, Codex, Cline, …) alongside the main agent in split panes, all reachable from the same prompt. Each agent keeps its own session, model and optional git worktree, and the main agent coordinates them — dispatching tasks and collecting reports as one team.
- **Computer tools via accessibility tree** — the agent reads and operates graphical interfaces through a structured, compact representation, falling back to screenshots only when needed. Fewer images processed, fewer tokens consumed.
- **TUI built from scratch in Rust** (Ratatui), inspired by OpenCode's UX. Providers, models and operating modes are configured right in the interface — almost zero manual setup.
- **Independent crates** — the development tools live in crates decoupled from the application, reusable as libraries outside morpheus.

## Installation

Tested on Linux and Windows. macOS hasn't been tested yet — if anything fails, [open an issue](https://github.com/aidreamy/co-sh/issues) with as much detail as possible.

**Linux & macOS** (also works on Windows via Git Bash / WSL):

```sh
curl -fsSL https://raw.githubusercontent.com/aidreamy/co-sh/main/download.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/aidreamy/co-sh/main/download.ps1 | iex
```

> Note: the `curl | bash` command does **not** work in PowerShell or CMD — Windows has no native `bash`. Use the PowerShell script above instead (Git Bash and WSL also work).

Optional variables: `COSH_BIN_DIR` (install directory), `COSH_VERSION` (specific version) and `COSH_VARIANT` (`slim`, the default, or `slim-embed`, with local RAG via fastembed). On PowerShell, set them as `$env:COSH_VERSION = "v0.1.0"` before running the command.

After installing, run `morpheus` in your terminal. If the install directory isn't on your `PATH`, the script prints the command to add it. If anything goes wrong during installation, open an issue — I'll be glad to help!

## Crates

| Crate | Description |
|---|---|
| [`morpheus-tools`](./crates/morpheus-tools/README.md) | Development tools and computer tools |
| [`morpheus-tui`](./crates/morpheus-tui/README.md) | TUI widget library (markdown/diff rendering, layout) — the components the app's terminal interface is built on |
| [`morpheus-sdk`](./crates/morpheus-sdk/README.md) | SDK for building agents on top of morpheus |
| [`morpheus-recall`](./crates/morpheus-recall/README.md) | Memory and context recall |
| [`morpheus-whatsapp`](./crates/morpheus-whatsapp/README.md) | WhatsApp Web (multi-device) integration: pairing, normalized events and typed sends |
| [`morpheus-onnx`](./crates/morpheus-onnx/README.md) | ONNX decision engine (behind the `onnx` feature) |

## Roadmap

- **Laya/ONNX decision engine** — optional `onnx` feature backed by the `morpheus-onnx` crate; see the [crate guide](./docs/onnx/onnx.md).
- **Graph-based harness orchestration** — a new session mode for coordinating multiple harnesses.

## License

Apache-2.0.
