# co-sh

[English](../../README.md) | [Português](./README.pt-BR.md) | [Español](./README.es.md) | [Русский](./README.ru.md) | [中文](./README.zh-CN.md) | **日本語**

> [!WARNING]
> **WhatsApp 機能は開発中です。** WhatsApp 機能（`morpheus-whatsapp`）はまだ実験的であり、**使用は推奨されません**。

Cosh はターミナル上で動作する汎用コーディングエージェントです。ソフトウェア開発ツールに加え、*computer tools* を備えており、グラフィカルアプリケーションとも対話できます。

## なぜ co-sh なのか？

co-sh は確立された coding agent を置き換えることを目的としていません。むしろ、オーケストレーション層としてそれらと並んで動作するよう設計されています。利用する理由は次のとおりです：

- **マルチエージェント協調** — さまざまな最先端エージェント（Claude、Gemini、Codex、Cline など）を同じ TUI 内で直接実行し、単一のインターフェースを通じてコミュニケーションと協働ができる共有ワークスペースを作ります。
- **実験しやすい** — 単一のエコシステムに縛られるのではなく、さまざまなエージェントを試して強みを組み合わせたい開発者に最適です。
- **マネージャーモード** — 既存のエージェント harness を使い続けながら、その上にオーケストレーションを追加できます。複数の小さなタスクを同時に進める際に特に便利です。
- **ネイティブな TUI 体験** — プロバイダー、モデル、動作モードをインターフェース上で直接設定でき、手動設定は最小限で済みます。
- **モジュール式アーキテクチャ** — 開発ツール、TUI コンポーネント、SDK、メモリ recall のための独立した再利用可能な crates。

## 他の TUI オーケストレーターとの違い

co-sh は、ターミナルベースの他のエージェントオーケストレーターとはいくつかの重要な点で異なります：

| 機能 | co-sh | 他の TUI オーケストレーター |
|------|-------|-----------------------------|
| **マルチエージェント分割ペイン** | 複数の外部エージェントを分割ペインで同時に実行し、それぞれが独自のセッション、モデル、オプションの git worktree を持つ | 通常は単一エージェントに集中するか、コンテキストの切り替えが必要 |
| **accessibility tree による computer tools** | 構造化された表現を通じてグラフィカル UI を読み取り操作し、必要な場合のみスクリーンショットにフォールバック | 通常はスクリーンショットに依存するか、GUI 操作をまったく持たない |
| **Rust 製の TUI** | Ratatui でゼロから構築され、OpenCode の UX に着想 | 多くの場合 Web ベースのターミナル、または最適化が不十分なインターフェース |
| **アプリ内設定** | プロバイダー、モデル、モードを TUI 内で直接設定 | 多くの場合、外部設定ファイルや CLI フラグが必要 |
| **モジュール式 crates** | 開発ツール、TUI ウィジェット、SDK、メモリ recall を独立した再利用可能なライブラリとして提供 | コンポーネントが密結合したモノリシックなアーキテクチャ |
| **共有ワークスペース** | 複数のエージェントが単一のインターフェースを通じて協働し、タスク配分を調整 | エージェントは通常それぞれ孤立して動作 |

主な特徴:

- **マルチエージェントモード（`/agents`）** — 外部エージェント（Gemini、Claude、Codex、Cline など）をメインエージェントと一緒に分割ペインで実行でき、すべて同じプロンプトから操作できます。各エージェントは自分のセッション・モデル・オプションの git worktree を持ち、メインエージェントがチームとしてタスクを振り分け、レポートを回収します。
- **accessibility tree による computer tools** — エージェントは構造化されたコンパクトな表現を通じてグラフィカル UI を読み取り操作し、スクリーンショットはフォールバックとしてのみ使用します。処理する画像が減り、トークン消費も削減されます。
- **Rust でゼロから構築された TUI**（Ratatui）。UX は OpenCode に着想を得ています。プロバイダー、モデル、動作モードの設定はすべてインターフェース上で完結 — 手動設定はほぼ不要です。
- **独立した crates** — 開発ツールはアプリ本体から切り離された crates として提供され、Cosh の外でもライブラリとして再利用できます。

## インストール

Linux と Windows でテスト済み。macOS は未テストです — 問題が発生した場合は、できるだけ詳細を添えて [issue を作成](https://github.com/aidreamy/co-sh/issues)してください。

**Linux および macOS**（Windows では Git Bash / WSL でも動作します）:

```sh
curl -fsSL https://raw.githubusercontent.com/aidreamy/co-sh/main/download.sh | bash
```

**Windows（PowerShell）:**

```powershell
irm https://raw.githubusercontent.com/aidreamy/co-sh/main/download.ps1 | iex
```

> 注意: `curl | bash` コマンドは PowerShell や CMD では**動作しません** — Windows にはネイティブの `bash` がありません。上記の PowerShell スクリプトを使用してください（Git Bash と WSL でも動作します）。

オプション変数: `COSH_BIN_DIR`（インストール先ディレクトリ）、`COSH_VERSION`（特定バージョン）、`COSH_VARIANT`（デフォルトは `slim`、fastembed によるローカル RAG 付きは `slim-embed`）。PowerShell では、コマンド実行前に `$env:COSH_VERSION = "v0.1.0"` のように設定してください。

インストール後、ターミナルで `cosh` を実行してください。インストールディレクトリが `PATH` にない場合は、スクリプトが追加用のコマンドを表示します。インストールで問題があれば issue を作成してください — 喜んでお手伝いします！

## Crates

| Crate | 説明 |
|---|---|
| [`morpheus-tools`](../../crates/morpheus-tools/README.md) | 開発ツールと computer tools |
| [`morpheus-tui`](../../crates/morpheus-tui/README.md) | TUI ウィジェットライブラリ（markdown/diff のレンダリング、レイアウト）— アプリのターミナル UI を構成するコンポーネント群 |
| [`morpheus-sdk`](../../crates/morpheus-sdk/README.md) | morpheus 上にエージェントを構築するための SDK |
| [`morpheus-recall`](../../crates/morpheus-recall/README.md) | メモリとコンテキストの再呼び出し |
| [`morpheus-whatsapp`](../../crates/morpheus-whatsapp/README.md) | WhatsApp Web（マルチデバイス）統合：ペアリング、正規化イベント、型付き送信 |
| [`morpheus-onnx`](../../crates/morpheus-onnx/README.md) | `onnx` feature で有効にする ONNX 意思決定エンジン |

## ロードマップ

- **Laya/ONNX 意思決定エンジン** — `morpheus-onnx` crate が提供する任意の `onnx` feature。[crate ガイド](../onnx/onnx.md)を参照してください。
- **グラフによる harness オーケストレーション** — 複数の harness を協調させる新しいセッションモード。

## ライセンス

Apache-2.0.
