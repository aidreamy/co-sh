# Morpheus

[English](../../README.md) | [Português](./README.pt-BR.md) | [Español](./README.es.md) | [Русский](./README.ru.md) | **中文** | [日本語](./README.ja.md)

> [!WARNING]
> **WhatsApp 功能仍在开发中。** WhatsApp 功能（`morpheus-whatsapp`）仍处于实验阶段，**暂不推荐使用**。

Morpheus 是一个运行在终端中的通用编码智能体（coding agent），除软件工程工具外，还配备了 *computer tools*，使其能够与图形应用程序交互。

## 为什么选择 Morpheus？

Morpheus 并不打算取代成熟的 coding agent —— 它的定位是作为编排层与它们协同工作。以下是使用它的理由：

- **多智能体协作** — 在同一 TUI 中直接运行不同的前沿智能体（Claude、Gemini、Codex、Cline 等），打造共享工作区，让它们通过单一界面进行沟通与协作。
- **适合实验** — 非常适合喜欢尝试不同智能体并融合各自优势，而不是绑定单一生态的开发者。
- **Manager 模式** — 继续使用你现有的智能体 harness，同时在其之上增加编排能力，尤其适合同时处理多个较小任务时使用。
- **原生 TUI 体验** — 直接在界面中配置 provider、模型和运行模式，几乎无需手动配置。
- **模块化架构** — 面向开发工具、TUI 组件、SDK 和记忆召回的独立、可复用的 crates。

## 与其他 TUI 编排器的区别

Morpheus 在几个关键方面区别于其他基于终端的智能体编排器：

| 特性 | Morpheus | 其他 TUI 编排器 |
|------|-------|-----------------|
| **多智能体分屏** | 在分屏窗格中同时运行多个外部智能体，每个都有自己的会话、模型和可选的 git worktree | 通常聚焦单一智能体，或需要切换上下文 |
| **基于 accessibility tree 的 computer tools** | 通过结构化表示读取并操作图形界面，仅在必要时回退到截图 | 通常依赖截图，或完全缺少 GUI 交互能力 |
| **基于 Rust 的 TUI** | 使用 Ratatui 从零构建，交互设计受 OpenCode 启发 | 往往是基于 Web 的终端或优化不足的界面 |
| **应用内配置** | Provider、模型和模式直接在 TUI 中配置 | 通常需要外部配置文件或 CLI 参数 |
| **模块化 crates** | 开发工具、TUI 组件、SDK 和记忆召回都是独立、可复用的库 | 组件紧密耦合的单体架构 |
| **共享工作区** | 多个智能体通过单一界面协作，并协调任务分派 | 智能体通常各自独立工作 |

主要特性：

- **多智能体模式（`/agents`）** — 让外部智能体（Gemini、Claude、Codex、Cline 等）与主智能体一起在分屏窗格中运行，全部可以通过同一个提示词触达。每个智能体保留自己的会话、模型和可选的 git worktree，由主智能体统一协调 — 像一个团队一样分派任务、收集报告。
- **基于 accessibility tree 的 computer tools** — 智能体通过结构化、紧凑的表示形式读取并操作图形界面，仅在必要时才回退到截图。处理的图像更少，消耗的 token 更少。
- **用 Rust 从零构建的 TUI**（Ratatui），交互设计受 OpenCode 启发。Provider、模型和运行模式都可以直接在界面中配置 — 几乎零手动配置。
- **独立的 crates** — 开发工具被拆分为与应用解耦的 crates，可以作为库在 Morpheus 之外复用。

## 安装

已在 Linux 和 Windows 上测试。macOS 尚未测试 — 如果遇到问题，请[提交 issue](https://github.com/aidreamy/co-sh/issues)，并尽可能详细描述。

**Linux 和 macOS**（在 Windows 上也可通过 Git Bash / WSL 使用）：

```sh
curl -fsSL https://raw.githubusercontent.com/aidreamy/co-sh/main/download.sh | bash
```

**Windows（PowerShell）：**

```powershell
irm https://raw.githubusercontent.com/aidreamy/co-sh/main/download.ps1 | iex
```

> 注意：`curl | bash` 命令在 PowerShell 或 CMD 中**无法**使用 —— Windows 没有原生的 `bash`。请使用上面的 PowerShell 脚本（Git Bash 和 WSL 同样可用）。

可选变量：`COSH_BIN_DIR`（安装目录）、`COSH_VERSION`（指定版本）和 `COSH_VARIANT`（`slim` 为默认，或 `slim-embed`，含基于 fastembed 的本地 RAG）。在 PowerShell 中，请在运行命令前将它们设置为 `$env:COSH_VERSION = "v0.1.0"`。

安装完成后，在终端运行 `morpheus`。如果安装目录不在你的 `PATH` 中，脚本会自动输出添加它的命令。安装过程中遇到任何问题，请提交 issue — 我很乐意帮忙！

## Crates

| Crate | 描述 |
|---|---|
| [`morpheus-tools`](../../crates/morpheus-tools/README.md) | 开发工具与 computer tools |
| [`morpheus-tui`](../../crates/morpheus-tui/README.md) | TUI 组件库（markdown/diff 渲染、布局）— 应用终端界面所基于的组件层 |
| [`morpheus-sdk`](../../crates/morpheus-sdk/README.md) | 用于在 morpheus 之上构建智能体的 SDK |
| [`morpheus-recall`](../../crates/morpheus-recall/README.md) | 记忆与上下文召回 |
| [`morpheus-whatsapp`](../../crates/morpheus-whatsapp/README.md) | WhatsApp Web（多设备）集成：配对、规范化事件和类型化发送 |
| [`morpheus-onnx`](../../crates/morpheus-onnx/README.md) | 由 `onnx` feature 启用的 ONNX 决策引擎 |

## 路线图

- **Laya/ONNX 决策引擎** — 由 `morpheus-onnx` crate 提供的可选 `onnx` feature；参见 [crate 指南](../onnx/onnx.md)。
- **基于图的 harness 编排** — 一种用于协调多个 harness 的新会话模式。

## 许可证

Apache-2.0.
