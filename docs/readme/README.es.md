# co-sh

[English](../../README.md) | [Português](./README.pt-BR.md) | **Español** | [Русский](./README.ru.md) | [中文](./README.zh-CN.md) | [日本語](./README.ja.md)

> [!WARNING]
> **WhatsApp está en desarrollo.** La función de WhatsApp (`cosh-whatsapp`) aún es experimental y **no se recomienda su uso**.

Cosh es un coding agent de propósito general para la terminal, con herramientas de ingeniería de software y *computer tools* que le permiten interactuar con aplicaciones gráficas.

## ¿Por qué co-sh?

co-sh no pretende reemplazar a los coding agents consolidados — está diseñado para trabajar junto a ellos como una capa de orquestación. Estas son algunas razones para usarlo:

- **Colaboración multi-agente** — Ejecuta distintos agentes de última generación (Claude, Gemini, Codex, Cline, etc.) directamente en la misma TUI, creando un espacio de trabajo compartido donde pueden comunicarse y colaborar a través de una única interfaz.
- **Ideal para experimentar** — Perfecto para desarrolladores a los que les gusta probar agentes diferentes y combinar sus fortalezas en lugar de comprometerse con un único ecosistema.
- **Modo gestor** — Sigue usando tus harnesses de agentes existentes mientras añades orquestación encima, especialmente útil cuando trabajas en varias tareas pequeñas a la vez.
- **Experiencia TUI nativa** — Configura providers, modelos y modos de operación directamente en la interfaz con una configuración manual mínima.
- **Arquitectura modular** — Crates independientes y reutilizables para herramientas de desarrollo, componentes de TUI, SDK y recall de memoria.

## En qué se diferencia de otros orquestadores TUI

co-sh se distingue de otros orquestadores de agentes basados en terminal en varios aspectos clave:

| Recurso | co-sh | Otros orquestadores TUI |
|---------|-------|-------------------------|
| **Paneles divididos multi-agente** | Ejecuta varios agentes externos a la vez en paneles divididos, cada uno con su propia sesión, modelo y git worktree opcional | Suelen enfocarse en un solo agente o requieren cambiar de contexto |
| **Computer tools vía accessibility tree** | Lee y opera interfaces gráficas mediante una representación estructurada, recurriendo a capturas solo cuando es necesario | Normalmente dependen de capturas de pantalla o carecen de interacción con la GUI |
| **TUI basada en Rust** | Construida desde cero con Ratatui, inspirada en la UX de OpenCode | A menudo terminales web o interfaces menos optimizadas |
| **Configuración en la app** | Providers, modelos y modos configurados directamente en la TUI | Suelen requerir archivos de configuración externos o flags de CLI |
| **Crates modulares** | Herramientas de desarrollo, widgets de TUI, SDK y recall de memoria como bibliotecas independientes y reutilizables | Arquitectura monolítica con componentes fuertemente acoplados |
| **Espacio de trabajo compartido** | Varios agentes colaboran a través de una única interfaz con despacho coordinado de tareas | Los agentes normalmente trabajan aislados |

Aspectos destacados:

- **Modo multi-agente (`/agents`)** — ejecuta agentes externos (Gemini, Claude, Codex, Cline, …) junto con el agente principal en paneles divididos, todos alcanzables desde el mismo prompt. Cada agente mantiene su propia sesión, modelo y git worktree opcional, y el agente principal los coordina — despacha tareas y recopila informes como un equipo.
- **Computer tools vía accessibility tree** — el agente lee y opera interfaces gráficas mediante una representación estructurada y compacta, y solo recurre a capturas de pantalla como respaldo. Menos imágenes procesadas, menos tokens consumidos.
- **TUI construida desde cero en Rust** (Ratatui), inspirada en la UX de OpenCode. La configuración de providers, modelos y modos de operación se hace directamente en la interfaz — casi cero configuración manual.
- **Crates independientes** — las herramientas de desarrollo viven en crates desacoplados de la aplicación, reutilizables como bibliotecas fuera de Cosh.

## Instalación

Probado en Linux y Windows. macOS aún no ha sido probado — si algo falla, [abre un issue](https://github.com/aidreamy/co-sh/issues) con el mayor detalle posible.

**Linux y macOS** (también funciona en Windows con Git Bash / WSL):

```sh
curl -fsSL https://raw.githubusercontent.com/aidreamy/co-sh/main/download.sh | bash
```

**Windows (PowerShell):**

```powershell
irm https://raw.githubusercontent.com/aidreamy/co-sh/main/download.ps1 | iex
```

> Nota: el comando `curl | bash` **no** funciona en PowerShell ni CMD — Windows no tiene `bash` nativo. Usa el script de PowerShell de arriba (Git Bash y WSL también funcionan).

Variables opcionales: `COSH_BIN_DIR` (directorio de instalación), `COSH_VERSION` (versión específica) y `COSH_VARIANT` (`slim`, predeterminado, o `slim-embed`, con RAG local vía fastembed). En PowerShell, defínelas como `$env:COSH_VERSION = "v0.1.0"` antes de ejecutar el comando.

Después de instalar, ejecuta `cosh` en la terminal. Si el directorio de instalación no está en tu `PATH`, el propio script muestra el comando para agregarlo. Ante cualquier dificultad en la instalación, abre un issue — ¡con gusto te ayudaré!

## Crates

| Crate | Descripción |
|---|---|
| [`cosh-tools`](../../crates/cosh-tools/README.md) | Herramientas de desarrollo y computer tools |
| [`cosh-tui`](../../crates/cosh-tui/README.md) | Biblioteca de widgets de TUI (renderización de markdown/diff, layout) — los componentes sobre los que se construye la interfaz de terminal de la app |
| [`cosh-sdk`](../../crates/cosh-sdk/README.md) | SDK para construir agentes sobre cosh |
| [`cosh-recall`](../../crates/cosh-recall/README.md) | Memoria y recall de contexto |
| [`cosh-whatsapp`](../../crates/cosh-whatsapp/README.md) | Integración con WhatsApp Web (multidispositivo): emparejamiento, eventos normalizados y envíos tipados |
| [`cosh-onnx`](../../crates/cosh-onnx/README.md) | Motor de decisiones ONNX, habilitado por la feature `onnx` |

## Roadmap

- **Motor de decisiones Laya/ONNX** — feature opcional `onnx`, proporcionada por el crate `cosh-onnx`; consulta la [guía del crate](../onnx/onnx.md).
- **Orquestación de harnesses en grafo** — un nuevo modo de sesión para coordinar múltiples harnesses.

## Licencia

Apache-2.0.
