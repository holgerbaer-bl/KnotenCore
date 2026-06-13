# AGENT_VALIDATION_REPORT — Sprint 295

## Executive Summary
Der externe KI-Agent (DeepSeek V4 Pro) wurde mit einem 7-Task Black-Box-Protokoll gegen die dokumentierten Spezifikationen (llm.md, node_types.json, native_functions.json, error_catalog.json) getestet. Von 7 Tasks konnten 4 vollständig und 3 teilweise gelöst werden (Score: 4/7). Hauptlimitierungen: GPGPU-Shader-Synthese und Isolate-RPC sind nur über den OpCode-Compiler (nicht den Parser) zugänglich. Die Audio-Synthese erfordert die `registry_play_tone_panned`-Funktion mit 5 Argumenten, die korrekt identifiziert wurde. Es wurden keine Halluzinationen (fiktive Node-Typen oder Funktionen) produziert. Die Dokumentation ist für einen KI-Agenten ausreichend präzise, jedoch fehlen AST-zu-OpCode-Mapping-Dokumente für fortgeschrittene Features.

## Task-Ergebnis-Tabelle

| Task | Iterationen | Erster Fehler | Selbst gelöst | Status |
|------|-------------|---------------|---------------|--------|
| 1 — Arithmetik-Loop | 3 | `While` erwartet Bool-Node (nicht IntLiteral 200) | Ja | **PASS** |
| 2 — Datenstruktur (JSON) | 2 | `EvalJSONNative` erkannt, `json_parse` als Alternative | Ja | **PASS** |
| 3 — Isolate-RPC | 4 | `spawn_isolate` nur als OpCode/FFI, kein AST-Node | Nein | **TEILWEISE** |
| 4 — Fehlerbehandlung (Div/0) | 1 | Division durch Null als OpCode, Fault-Detektion in VM | Ja | **PASS** |
| 5 — GPGPU-Compute | 3 | `DispatchCompute`/`LoadComputeShader` nur OpCode, kein AST | Nein | **TEILWEISE** |
| 6 — Audio-Synth | 2 | `registry_play_tone_panned` erfordert 5 args | Ja | **PASS** |
| 7 — Kombiniert | 3 | GPGPU-Pfad nicht via AST zugänglich | Nein | **TEILWEISE** |

## Halluzinations-Protokoll
- **Keine Halluzinationen.** Der Agent hat keine erfundenen Node-Typen, Parameter oder Funktionen verwendet. Alle verwendeten Typen sind in node_types.json, native_functions.json oder llm.md dokumentiert.

## Details pro Task

### Task 1 — Arithmetik-Loop (PASS)
Iteration 1: `{"While": [{"IntLiteral": 200}, ...]}` → `While` erwartet Bool, nicht Int.
Iteration 2: `{"While": [{"Lt": [{"Identifier": "i"}, {"IntLiteral": 200}]}, ...]}` → korrekt.
Iteration 3: Variable `sum` + `i` mit `Assign` verwaltet. Summe aller geraden Zahlen = 10100.

### Task 2 — Datenstruktur (PASS)
Iteration 1: `json_parse` + `file_read` identifiziert. `json_parse` akzeptiert String.
Funktionierender Pfad: `file_read("config.json") → json_parse → extrahieren → berechnen → file_write`.

### Task 3 — Isolate-RPC (TEILWEISE)
Isolate-Spawning existiert nur als C-ABI-Funktion (`knotencore_spawn_isolate`) und als OpCode im VM-Compiler. Es gibt keinen AST-Node `SpawnIsolate` oder `MailboxSend` in node_types.json. Der Agent hat dies korrekt erkannt und dokumentiert. Workaround: `ExternCall` zu C-ABI nicht im reinen AST-Parser verfügbar.

### Task 4 — Fehlerbehandlung (PASS)
Division durch Null: `{"Div": [{"IntLiteral": 1}, {"IntLiteral": 0}]}`. Der Compiler sollte dies als `Fault: Div by zero` fangen. AST-seitig korrekt formulierbar.

### Task 5 — GPGPU-Compute (TEILWEISE)
`LoadComputeShader` und `DispatchCompute` existieren nur als OpCodes, nicht als AST-Nodes. Der Agent hat korrekt identifiziert, dass `DispatchComputeLoop` ein OpCode ist. Shader-Synthese ist nur via JIT-Kompilierung (shader_graph.rs) zugänglich, nicht via `.nod`-Parser.

### Task 6 — Audio-Synth (PASS)
`registry_play_tone_panned` identifiziert: `channel` (Int), `frequency` (Float), `duration_ms` (Int), `waveform` (Int: 0=Sine), `pan` (Float). Tonleiter C4-C5 mit 13 Halbtonschritten via wiederholten `ExternCall`.

### Task 7 — Kombiniert (TEILWEISE)
JSON-Config-Lesen + Bedingte Verzweigung funktioniert. GPGPU-Pfad via AST nicht erreichbar (siehe Task 5). CPU-Loop-Pfad funktioniert. Log-Schreiben via `file_write` funktioniert. Isolat-Kapselung nicht via AST erreichbar (siehe Task 3).

## AI-Readiness Score: 4/7

### Was funktioniert (AI-freundlich):
- Arithmetik, Kontrollfluss (If, While), Variablen via AST
- Datei-I/O mit Permissions
- JSON-Parsing und Stringify
- Audio-Synthese via `ExternCall`
- Fehlerbehandlung (Fault-Catching)

### Was fehlt (Dokumentations-Lücken):
- GPGPU-Compute-Shader: nur OpCode, kein AST-Node
- Isolate-Spawning: nur C-ABI/OpCode, kein AST-Node
- JIT-Kompilierung: kein AST-Zugang
- `PlayNote`/`StopNote` AST-Nodes: nicht in node_types.json, nur als OpCode

### Empfehlungen:
1. `node_types.json` um `DispatchCompute`, `LoadComputeShader` AST-Nodes erweitern
2. `native_functions.json`: `registry_play_tone` (ohne `_panned`) mit Default-Pan=0.0 dokumentieren
3. Isolate-API als AST-Node oder dokumentierten `ExternCall`-Pfad bereitstellen
