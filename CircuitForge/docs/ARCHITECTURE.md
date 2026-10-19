# CircuitForge Architecture Specification

## 1. System Overview

CircuitForge is architected as an offline, single-binary desktop engineering environment utilizing a hybrid native-web stack:
- **Presentation Layer**: HTML5 Canvas + TypeScript running in WebView2.
- **IPC & Native Core**: Rust (Tauri v2) providing IPC command handling, topological netlist generation, rawfile data parsing, and OS-level file operations.
- **Simulation Kernel**: Integrated Berkeley SPICE 3f5 / ngspice-38 64-bit engine executed as an isolated batch subprocess.

```
+-----------------------------------------------------------------------+
|                         CircuitForge UI (Vite + TS)                    |
|                                                                       |
|  [Command Bar]  [Component Palette]  [Schematic Canvas]  [Inspector]   |
|  [Analysis Deck: Waveforms / DC Bias Table / Netlist / Diagnostics]   |
+-----------------------------------------------------------------------+
                                   |  Tauri IPC (Typed JSON)
                                   v
+-----------------------------------------------------------------------+
|                         Tauri Rust Native Core                         |
|                                                                       |
|   commands::*           -> model::Circuit::validate()                 |
|   netlist::NetlistGen   -> Disjoint-Set Union (DSU) Node Extractor    |
|   engine::SimEngine     -> Subprocess manager with tempdir sandbox    |
|   rawfile::Parser       -> ASCII rawfile parser (real & complex data) |
+-----------------------------------------------------------------------+
                                   |  Process Spawn (CLI flags: -b -r -o)
                                   v
+-----------------------------------------------------------------------+
|                    ngspice 38 64-bit Batch Engine                     |
+-----------------------------------------------------------------------+
```

---

## 2. Netlist Generation & Node Topological Resolution

A common challenge in schematic editors is determining electrical nodes from arbitrarily placed wires and pins. CircuitForge solves this using a **Disjoint-Set Union (DSU)** algorithm with integer grid quantization ($20\text{px}$ pitch):

1. **Quantization**: Every component pin coordinate $(x + \text{offset}_x, y + \text{offset}_y)$ and wire vertex is mapped to a discrete integer coordinate `GridPoint(round(x), round(y))`.
2. **Topological Merging**:
   - For every wire segment connecting vertex $A$ to vertex $B$, their sets in the DSU are unified.
   - For T-junctions, if a point lies collinear on a segment, its set is unified with the segment endpoints.
3. **Ground Resolution**: All pins belonging to components of type `Ground` are mapped to the canonical SPICE reference node `0`.
4. **Designator & Subcircuit Synthesis**:
   - Resistors: `R_<designator> <n1> <n2> <value>`
   - Capacitors: `C_<designator> <n1> <n2> <value>`
   - Inductors: `L_<designator> <n1> <n2> <value>`
   - Diodes: `D_<designator> <na> <nc> <model>` with synthetic `.MODEL` definitions.
   - Sources: DC (`VDC`), AC (`VAC`), Sine (`VSIN`), and Pulse (`VPULSE`) with full parameter support.
   - Op-Amps: Synthesized via standard ideal voltage-controlled voltage source subcircuit (`.SUBCKT OPAMP_IDEAL`).

---

## 3. Subprocess Isolation & Simulation Flow

1. **Validation**: The Rust backend executes `Circuit::validate()` checking for:
   - Presence of at least one Ground reference node (`NO_GROUND`).
   - Unique reference designators (`DUPLICATE_DESIGNATOR`).
   - Non-empty component values (`EMPTY_VALUE`).
2. **Sandbox Creation**: A temporary directory (`circuitforge_sim_XXXXXX`) is initialized via `tempfile::Builder`.
3. **Batch Control Block**: A `.control` block is injected before `.END`:
   ```spice
   .control
   set filetype=ascii
   run
   write "<sandbox>/output.raw"
   .endc
   ```
4. **Execution**: `ngspice_con.exe` is invoked directly without spawning a shell (`-b -r output.raw -o output.log circuit.cir`).
5. **Telemetry & Parsing**: Standard output and `output.log` are captured; `output.raw` is parsed into structured Rust objects with both real time-series vectors and complex AC vectors.
6. **Teardown**: The sandbox directory is safely deallocated.

---

## 4. Anti-Template Compliance

All UI components strictly adhere to the workbench design specifications in [docs/DESIGN.md](docs/DESIGN.md):
- High-contrast visual hierarchy ($12\text{px}/13\text{px}$ typography, Monospace figures for electrical values).
- No placeholder UI elements; all controls and menus are directly wired to simulation or workspace state.
- Offline-first execution: all dependencies, icons, fonts, and binaries are packaged locally without external network requests.
