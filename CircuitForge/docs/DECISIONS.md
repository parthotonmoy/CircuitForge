# Architecture & Technical Decisions Log

This document records key architectural, engineering, and legal decisions made during the design and development of CircuitForge, following the priority order established in the project specification.

---

## Decision 1: Clean-Room Implementation & Exclusion of Binary Decompilation

- **Date:** 2026-10-19
- **Context:** PSpice 9.2 is installed on the host system. Section 2 and Section 5.4 require reviewing the installed EULA before performing any binary disassembly with Ghidra.
- **Evidence:** The EULA found in `C:\Program Files\Orcad\Capture\ENDUSER.DOC` and network licensing terms prohibit reverse engineering, decompilation, and disassembly.
- **Decision:** Ghidra binary decompilation and disassembly of proprietary binaries are strictly skipped. CircuitForge is implemented completely clean-room from public Berkeley SPICE 3f5 syntax, ngspice documentation, and open electrical engineering standards.
- **Impact:** Zero proprietary intellectual property contamination, complete legal compliance, clean open-source licensing.

---

## Decision 2: Simulation Engine Selection — ngspice

- **Date:** 2026-10-19
- **Context:** Reliable, mature SPICE simulation backend needed with Windows x64 support and permissible redistribution.
- **Alternatives Considered:**
  1. *Writing a custom SPICE solver in Rust:* High risk of numerical instability, massive scope for non-linear transistor/diode models, and divergence from industry-standard results.
  2. *Bundled official ngspice 64-bit console executable:* Industry standard, decades of battle-tested numerical routines, robust BSIM/Gummel-Poon models, clean Modified BSD license.
- **Decision:** Bundle the official ngspice 64-bit console binary (`ngspice_con.exe`) as a native application resource with fallback discovery (`bundled -> user configured path -> system PATH`).
- **Impact:** Accurate, verified simulation results matching standard academic and industrial benchmarks.

---

## Decision 3: Application Architecture — Rust Core + Tauri v2 + Vanilla TypeScript Canvas

- **Date:** 2026-10-19
- **Context:** Desktop workbench requiring responsive schematic drawing, instant numerical response, memory safety, and cross-platform native capabilities.
- **Architecture:**
  - **Rust Backend:** Source of truth for circuit model, topology graph, netlist compilation, ngspice child process supervisor, rawfile parsing, and file I/O.
  - **Tauri v2:** High-efficiency IPC bridge without heavy Electron overhead, strict Content Security Policy, zero cloud telemetry.
  - **Frontend:** TypeScript + Vite, using an HTML5 Canvas for the schematic editor to ensure 60fps rendering, smooth pan/zoom, sub-pixel grid alignment, and crisp vector linework.
- **Impact:** Lightweight desktop binary, fast startup, robust data validation, and responsive interactive experience.

---

## Decision 4: Project Storage Format — Versioned JSON with Atomic Writes

- **Date:** 2026-10-19
- **Context:** Circuit projects require robust serialization resistant to crashes or disk write corruption.
- **Decision:** Store circuits in human-readable, schema-versioned JSON (`.forge` / `.json`). Saves are executed atomically via temporary file write followed by atomic rename.
- **Impact:** Prevents corrupt saves, provides clear migration paths for future schema versions, and allows easy version control inspection.
