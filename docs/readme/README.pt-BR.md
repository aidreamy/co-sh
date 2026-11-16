# co-sh

[English](../../README.md) | **Português** | [Español](./README.es.md) | [Русский](./README.ru.md) | [中文](./README.zh-CN.md) | [日本語](./README.ja.md)

> [!WARNING]
> **O WhatsApp está em desenvolvimento.** A funcionalidade de WhatsApp (`cosh-whatsapp`) ainda é experimental e **não é recomendada para uso**.

Cosh é um coding agent de propósito geral para o terminal, com ferramentas de engenharia de software e *computer tools* que permitem interagir com aplicações gráficas.

## Por que co-sh?

co-sh não pretende substituir os coding agents consolidados — foi projetado para trabalhar ao lado deles como uma camada de orquestração. Veja por que você pode querer usá-lo:

- **Colaboração multi-agente** — Rode diferentes agentes de ponta (Claude, Gemini, Codex, Cline, etc.) diretamente na mesma TUI, criando um workspace compartilhado onde eles podem se comunicar e colaborar por uma única interface.
- **Amigável à experimentação** — Perfeito para desenvolvedores que gostam de testar agentes diferentes e combinar seus pontos fortes em vez de se prender a um único ecossistema.
- **Modo gerente** — Continue usando seus harnesses de agentes existentes enquanto adiciona orquestração por cima, especialmente útil ao trabalhar em várias tarefas menores ao mesmo tempo.
- **Experiência TUI nativa** — Configure providers, modelos e modos de operação diretamente na interface, com mínima configuração manual.
- **Arquitetura modular** — Crates independentes e reutilizáveis para ferramentas de desenvolvimento, componentes de TUI, SDK e recall de memória.

## Como se diferencia de outros orquestradores TUI

O co-sh se destaca de outros orquestradores de agentes baseados em terminal em vários pontos-chave:

| Recurso | co-sh | Outros orquestradores TUI |
|---------|-------|---------------------------|
| **Painéis divididos multi-agente** | Rode vários agentes externos ao mesmo tempo em painéis divididos, cada um com sua própria sessão, modelo e git worktree opcional | Geralmente focados em um único agente ou exigem alternar de contexto |
| **Computer tools via accessibility tree** | Lê e opera interfaces gráficas por meio de uma representação estruturada, recorrendo a screenshots só quando necessário | Costumam depender de screenshots ou não têm interação com GUI |
| **TUI em Rust** | Construída do zero com Ratatui, inspirada na UX do OpenCode | Muitas vezes terminais web ou interfaces menos otimizadas |
| **Configuração dentro do app** | Providers, modelos e modos configurados direto na TUI | Costumam exigir arquivos de configuração externos ou flags de CLI |
| **Crates modulares** | Ferramentas de desenvolvimento, widgets de TUI, SDK e recall de memória como bibliotecas independentes e reutilizáveis | Arquitetura monolítica com componentes fortemente acoplados |
| **Workspace compartilhado** | Vários agentes colaboram por uma única interface com despacho coordenado de tarefas | Agentes normalmente trabalham isolados |

Destaques:

- **Modo multi-agente (`/agents`)** — rode agentes externos (Gemini, Claude, Codex, Cline, …) junto com o agente principal em panes divididas, todos alcançáveis pelo mesmo prompt. Cada agente mantém sua própria sessão, modelo e git worktree opcional, e o agente principal os coordena — despachando tarefas e coletando relatórios como um time.
- **Computer tools via accessibility tree** — o agente lê e opera interfaces gráficas por uma representação estruturada e compacta, e só recorre a screenshots como fallback. Menos imagens processadas, menos tokens consumidos.
- **TUI construída do zero em Rust** (Ratatui), inspirada na UX do OpenCode. Configuração de providers, modelos e modos de operação direto na interface — quase zero configuração manual.
- **Crates independentes** — as ferramentas de desenvolvimento vivem em crates desacoplados da aplicação, reutilizáveis como bibliotecas fora do Cosh.

## Instalação

Testado em Linux e Windows. macOS não foi testado ainda — se algo falhar, [abra uma issue](https://github.com/aidreamy/co-sh/issues) com o máximo de detalhes.

**Linux e macOS** (também funciona no Windows via Git Bash / WSL):

```sh
curl -fsSL https://raw.githubusercontent.com/aidreamy/co-sh/main/download.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/aidreamy/co-sh/main/download.ps1 | iex
```

> Nota: o comando `curl | bash` **não** funciona no PowerShell ou CMD — o Windows não tem `bash` nativo. Use o script PowerShell acima (Git Bash e WSL também funcionam).

Variáveis opcionais: `COSH_BIN_DIR` (diretório de instalação), `COSH_VERSION` (versão específica) e `COSH_VARIANT` (`slim`, padrão, ou `slim-embed`, com RAG local via fastembed). No PowerShell, defina-as como `$env:COSH_VERSION = "v0.1.0"` antes de executar o comando.

Depois de instalar, rode `cosh` no terminal. Se o diretório de instalação não estiver no seu `PATH`, o próprio script mostra o comando para adicioná-lo. Qualquer dificuldade na instalação, abra uma issue — ficarei feliz em ajudar!

## Crates

| Crate | Descrição |
|---|---|
| [`cosh-tools`](../../crates/cosh-tools/README.md) | Ferramentas de desenvolvimento e computer tools |
| [`cosh-tui`](../../crates/cosh-tui/README.md) | Biblioteca de widgets de TUI (renderização de markdown/diff, layout) — os componentes sobre os quais a interface de terminal do app é construída |
| [`cosh-sdk`](../../crates/cosh-sdk/README.md) | SDK para construir agentes sobre o cosh |
| [`cosh-recall`](../../crates/cosh-recall/README.md) | Memória e recall de contexto |
| [`cosh-whatsapp`](../../crates/cosh-whatsapp/README.md) | Integração com o WhatsApp Web (multi-device): pareamento, eventos normalizados e envios tipados |
| [`cosh-onnx`](../../crates/cosh-onnx/README.md) | Motor de decisões ONNX, habilitado pela feature `onnx` |

## Roadmap

- **Motor de decisões Laya/ONNX** — feature opcional `onnx`, fornecida pelo crate `cosh-onnx`; veja o [guia do crate](../onnx/onnx.md).
- **Orquestração de harness em grafo** — um novo modo de sessão para coordenar múltiplos harnesses.

## Licença

Apache-2.0.
