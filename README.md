# KnotenCore 🦀🤖

[![Version](https://img.shields.io/badge/version-v1.3.0--alpha-orange)](https://github.com/holgerbaer-bl/KnotenCore/releases/tag/v1.3.0-alpha)
[![CI Quality Gates](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml/badge.svg)](https://github.com/holgerbaer-bl/KnotenCore/actions/workflows/ci.yml)
[![Release](https://img.shields.io/badge/release-prerelease-yellow)](https://github.com/holgerbaer-bl/KnotenCore/releases/latest)

*(Noun) /knoːtən kɔːr/*

1. **Not** a relentless underground German hardcore techno subgenre. 
2. A blazing-fast, thread-safe, and deterministic **AI-Native Execution Runtime** — agents feed it structured JSON logic, the AOT compiler turns it into flat bytecode, and the Stack-VM executes it at bare-metal speed. No browser. No GC. No surprises.

**The Deterministic AI-Native Execution Runtime.**

## What is KnotenCore?
**KnotenCore** is a **Deterministic AI-Native Execution Runtime** built entirely in Rust. External AI agents describe programs as structured **JSON-AST nodes** (`.nod` files). The engine compiles these directly into an AOT-optimized flat bytecode stream and executes them on a Register Stack-VM — achieving deterministic, GC-free, bare-metal performance without any intermediate browser or script-engine layer. WGPU-based rendering functions as the **Physical Representation Layer**, allowing agents to express 3D scenes, audio, and UI as pure data via an autonomous, asynchronous Retained-Mode pipeline.

---

## 🤖 AI-Readiness & Architectural Immunity

KnotenCore is purpose-built for autonomous AI agents. Every node and native function is formally specified, machine-validated, and anchored against structural drift:

| Artifact | Path | Purpose |
|----------|------|---------|
| **EBNF Grammar** | [`docs/LANGUAGE_REFERENCE/nod_grammar.ebnf`](docs/LANGUAGE_REFERENCE/nod_grammar.ebnf) | Normative structural grammar of every `.nod` JSON node. Eliminates ambiguity for LLM code generation. |
| **JSON Schema** | [`docs/LANGUAGE_REFERENCE/node_types.json`](docs/LANGUAGE_REFERENCE/node_types.json) | Full Draft-07 JSON Schema with `additionalProperties: false` on every object node. **Hallucinated fields are rejected at runtime.** |
| **Function Registry** | [`docs/LANGUAGE_REFERENCE/native_functions.json`](docs/LANGUAGE_REFERENCE/native_functions.json) | Machine-readable registry of every native FFI function (30+), with parameter types, return types, required permissions, and live AST call examples. |
| **Anti-Pattern Guide** | [`docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod`](docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod) | 10 explicit DO/DON'T patterns for AI agents covering wrong node names, bare scalars, hallucinated functions, and ExternCall misuse. |
| **Error Catalog** | [`docs/LANGUAGE_REFERENCE/error_catalog.json`](docs/LANGUAGE_REFERENCE/error_catalog.json) | Registry of execution fault codes and self-healing hints for AI agents. |
| **Semantic Anchoring** | `#ANCHOR:` | Standardisierte, maschinenlesbare IDs im Quellcode (`CORE_TYPES_SOF`, `GPGPU_ASYNC_CHANNEL`) koordinieren KI-Refactorings synchron ueber den `llm.md` Routing-Hub. |
| **AI Agent Guide** | [`llm.md`](llm.md) | Routing document directing agents to the authoritative references above and documenting all engine constraints. |

---

## 🎯 AI-Readiness Benchmark — AG Baseline: 20/20 (100%)

KnotenCore is the **first DSL project with a public, reproducible AI-Readiness Score** — measuring how reliably external LLMs generate correct `.nod` programs without human correction. The AG agent used **only `llm.md` + `node_types.json` + `native_functions.json`** as context (no Rust source).

---

## ⏱️ Performance & Optimization Benchmarks

KnotenCore features a dual JIT/AOT engine architecture. On a computationally demanding 1,000,000 algorithmic loop constraint using the Leibniz pi estimation heavily encoded with Float primitives (`Mul`, `Add`, `Div`, `While`, `Assign`), tests generated via `bench_knc` resulted in:

- **JIT Evaluator:** ~1914 ms
- **AOT Stack VM:** ~1580 ms (Speedup factor: **1.21x** natively faster out-of-the-box).

### ⚡ AOT Compiler Optimization Engine
* **AST Function Inlining (Sprint 194):** Trivial native FFI math and string calls are resolved and folded directly at compile time.
* **Loop Unrolling & Static Bound Analysis (Sprint 196):** Bounded while-loops ($N \le 8$) are expanded into flat blocks at compile time. Static infinite loops without exit paths are rejected early.
* **Peephole Optimization & Slot Reuse (Sprint 197):** Instruction post-pass eliminates redundant Store-Load chains. Register slot reuse minimizes stack frame sizes.
* **SIMD Auto-Vectorization (Sprint 200):** High-speed optimizer pass collapses sequential float constants into single instruction streams execution-driven by `glam::Vec4` in a single CPU tick.

---

## Runtime Architecture (Three-Crate Workspace)

KnotenCore operates as a strictly separated, circular-dependency-free multi-crate ecosystem:

```
JSON-AST (.nod)  ->  Parser  ->  AST (Node enum inside knoten_core_types)
                                    |
              +---------------------+---------------------+
              |                                           |
        JIT Executor                              AOT VM Compiler
   (Inside aether_compiler)                   (Inside aether_compiler)
              |                                           |
    egui / WGPU Render Layer                      Flat Opcodes
   (Physical Repr. Layer)                     Stack-VM Core (ALU)
```

| Crate / Module | Role |
|---|---|
| **`knoten_core`** | **Fassade** — Duenne Haupt-Crate; fungiert als Re-Export-Fassade nach aussen zur reibungslosen Workspace-Steuerung. |
| **`aether_compiler`** | **Engine Core** — Beheimatet den autonomen JIT-Graph-Executor, den AOT-Bytecode-Compiler sowie die Stack-VM zur allocations-freien ALU-Befehlsabarbeitung. |
| **`knoten_core_types`** | **Sole Source of Truth** — Beheimatet exklusiv die reinen Datenzertifikate (`Node`, `OpCode`, `SimdOp`) frei von Cross-Crate-Logikkopplungen. |
| `src/audio.rs` | **Audio Engine** — Thread-sichere Audio-Pipeline via `rodio`; entkoppelt vom Render-Takt. |
| `src/bin/knoten_lsp.rs` | **Language Server (LSP)** — `tower-lsp` Server fuer Echtzeit-Linter-Validierung und Hover-Diagnostik direkt im Editor. |

---

## Key Features & Showcases

### 🔒 Thread-Safe & Sandboxed Safety
- **Deny-by-Default Execution:** Saemtliche I/O-Operationen werden blockiert, sofern sie nicht explizit via CLI-Flags freigegeben wurden (`--allow-read`, `--allow-write`, `--allow-net`).
- **The Watchdog (Sprint 171):** Ein 50ms Hard-Timeout bricht Amok laufende Endlosschleifen der Stack-VM ressourcenschonend ab.
- **The FFI Shield (Sprint 170/173):** Alle FFI-Grenzen sind via `std::panic::catch_unwind` und Null-Pointer-Validierungen gegen unkontrollierte Engine-Abstuerze gehaertet.

### 🖥️ WGPU Retained-Mode Scene Graph
- **Persistent Geometry:** Scripts spawnen Entitaeten (`registry_spawn_cube`) einmalig; der WGPU-Loop zeichnet den Szenengraphen autonom mit 60 FPS (0% CPU Idle via `ControlFlow::Wait`).
- **Dynamic Blinn-Phong Illumination:** Shader-Unterstuetzung fuer Ambient-Pass plus bis zu 4 parallele Punktlichter mit physikalisch plausibler quadratischer Abschwaechung.

### ⚡ Audited, Mutex-Free GPGPU Streaming (Sprints 213-216)
Die Synchronisation zwischen der Stack-VM und dem WGPU-Render-Triebwerk operiert vollstaendig parallelisiert, isoliert und ohne blockierende Engpaesse:
- **Isolierte Shader-Kanaele:** `COMPUTE_CHANNELS` verwaltet pro `shader_id` ein dediziertes, atomares `crossbeam_channel::bounded(1)`. Datenlecks oder Crosstalk zwischen parallelen Shadern sind unmoeglich.
- **Garantiert ununterbrochenes Rendern:** Der zeitkritische WGPU-Thread verwendet ein strikt nicht-blockierendes `try_send()`. Kanallast oder Verzoegerungen auf der VM-Seite werden geraeuschlos verworfen — der Winit-Eventloop friert niemals ein.
- **Contention-Freies Polling:** Das Auslesen via `registry_compute_readback` klont den Receiver unter einem extrem kurzlebigen Lock-Block. Der Mutex-Guard wird *vor* dem Eintritt in den 1000er `std::hint::spin_loop()` abgeworfen. VM und Render-Thread agieren maximal entkoppelt und parallel.

### 🎧 Native Bare-Metal Audio Engine
KnotenCore verfuegt ueber eine vollstaendig integrierte, thread-sichere asynchrone Audio-Pipeline direkt verknuepft mit dem Runtime-Kern:
- **Zero-Latency Playback:** Fire-and-forget Audioeffekte (`.wav`, `.ogg`) triggern instantan aus Bytecode-Instruktionen via `AudioManager`.
- **Sicherheits-Sandbox:** Alle Audio-Invocations verlangen implizit nach `--allow-read`-Berechtigungen und passieren die unnachgiebige Pfadvalidierung.

### 🧮 Math Standard Library & Determinism
Um komplexe orbitale Mechaniken nativ im VM-Loop zu berechnen, stellt die Engine deterministische Bindungen an Rusts mathematische Kernbibliothek zur Verfuegung:
- **Verfuegbare Funktionen:** `math_sin`, `math_cos`, `math_tan`, `math_sqrt`, `math_abs`, `math_pi`, `math_random`.
- **Typ-Garantie:** Die FFI-Schnittstelle erzwingt strikte `Float`-Signaturen. Parameter-Mischungen loesen sofort eine strukturierte `Fault`-Meldung aus.

### 🛡️ Resource Cleanup & Memory Safety (Sprint 169)
- **Sofortige Entitaets-Zerstoerung:** `registry_destroy_entity(win, id)` entfernt Objekte sofort aus dem Szenengraphen und der Physik-Welt.
- **Stabiler Lebenszyklus:** Shared Geometry (`CachedMesh`) und Texturen verbleiben im Cache fuer andere Entitaeten. Das RAM- und VRAM-Profil verhaelt sich unter unendlichen Spawn/Destroy-Zyklen absolut flach und leckagefrei.

### 🔌 LSP Support — Echtzeit AI-DX
KnotenCore liefert eine vollwertige Erweiterung in `tools/vscode-knotencore/`:
* **Syntax Highlighting:** TextMate-Grammatiken fuer `.knoten` (DSL) und `.nod` (JSON-AST).
* **Code Snippets & LSP:** Direkte Anbindung an den tower-lsp-Server fuer Echtzeit-Fehlermeldungen (`ERR_UNKNOWN_NODE`) und interaktive Funktions-Dokumentation direkt im Editor.

---

### 📊 Application Showcase: The Ultimate Telemetry Dashboard
Ein umfassender Showcase (`examples/telemetry_dashboard.knoten`), der saemtliche Optimierungs- und Datenschichten des Oekosystems vereint:

```javascript
// Neural DSL (.knoten) — NOT JSON-AST. See docs/KNOTEN_SPEC.md
import "core/net.nod";
import "core/fs.nod";
import "core/json.nod";

// Sicheres Abfragen der Container-Metriken mit --allow-net
let response = fetch("https://knotencore.de/api/telemetry");
let payload = json_parse(response);

// 3-Level tiefer, null-sicherer Objektzugriff dank Compiler-Inlining
let cpu_usage = payload.system.metrics.cpu;
let ram_usage = payload.system.metrics.ram;

// Schreiben in das egui-Render-Triebwerk ueber datengebundene UI-Komponenten
ui_init_window(800, 400, "KnotenCore Live Telemetry Monitor");

while (true) {
    UIWindow("Dashboard", "System Status") {
        ui_bar_chart("CPU History (Last 10 Runs)", cpu_usage);
        ui_progress_gauge("RAM Saturation", ram_usage, 0.0, 100.0);
    };
    
    ui_present();
    sleep(16); // CPU-Drosselung auf konstante ~60 FPS
}
```

---

## Compliance & Community Flow

This repository maintains absolute version integrity. Every sprint is planned, rigorously executed, evaluated across local unit/integration tests, explicitly documented within `changelog.md`, and natively pushed to this repository by autonomous agents. 

### Community Guidelines
Open-source contributors and autonomous agents interacting with this framework must strictly abide by our repository documents:
- Review [CONTRIBUTING.md](CONTRIBUTING.md) to understand the AOT Stack Machine and Sandbox constraints before submitting `PULL_REQUEST` templates.
- Consult [SECURITY.md](SECURITY.md) to privately report FileSystem/FFI escapes.
- Follow the [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

Reference `llm.md` for strict machine-readable constraints regarding runtime architecture and OS bindings.

---

**[https://knotencore.de/](https://knotencore.de/) — The Official Engineering & Telemetry Control Hub**
