# CircuitForge Development Progress

## Current Milestone: M6 (Distribution) — ✅ ALL MILESTONES COMPLETE & ENHANCED

### Milestone Status Breakdown

| Milestone | Status | Details |
|---|---|---|
| **M1: Environment and Feasibility** | **implemented and tested** | Verified toolchains, ngspice-38 engine, clean-room legal EULA check, REA skills, Tauri smoke test. |
| **M2: Design Plan & Foundation** | **implemented and tested** | Design system (`docs/DESIGN.md`), Decisions (`docs/DECISIONS.md`), Journal (`docs/DESIGN_JOURNAL.md`), Schema (`docs/PROJECT_FORMAT.md`), Canvas schematic capture engine (`src/main.ts`), DSU netlist generator (`src-tauri/src/netlist.rs`). |
| **M3: Real Simulation** | **implemented and tested** | End-to-end ngspice batch execution, ASCII rawfile parsing for real & complex vectors, validation diagnostics, robust AC analysis syntax handling. |
| **M4: Engineering Workflow** | **implemented and tested** | Waveform oscilloscope canvas with logarithmic AC Bode frequency scale and cursor readouts, DC bias table, netlist viewer, CSV export, example project files (`examples/*.forge`). |
| **M5: Quality and Polish** | **implemented and tested** | Pure white clean laboratory aesthetic, Times New Roman typography throughout, eliminated circular source stray wire artifacts, all 7 analytical test benchmarks passing. |
| **M6: Distribution** | **implemented and tested** | Standalone production binary `circuitforge.exe` (9.92 MB) and verified NSIS Windows installer `CircuitForge_0.1.0_x64-setup.exe` (7.22 MB). |

---

### Recent Fixes & Engineering Enhancements (Verified)

1. **Eliminated "Weird Diagonal Wire" Artifact on Circular Sources:**
   - **Root Cause:** In HTML5 Canvas 2D API, calling `.arc(cx, cy, r, ...)` after `lineTo(0, 40)` draws an implicit chord connecting the current subpath endpoint `(0, 40)` to the starting angle `(18, 0)` of the arc.
   - **Fix:** Created `drawCircle(ctx, cx, cy, r)` which explicitly performs `ctx.moveTo(cx + r, cy)` before `ctx.arc(cx, cy, r, ...)`. Applied to all circular sources (`vac`, `vsin`, `vpulse`, `idc`, `iac`, `bjtNpn`, `bjtPnp`, probes).

2. **Dual & Differential Voltage Probes ($V_+ - V_-$):**
   - **Design:** Redesigned into realistic test probes with needle tips at `(0, 0)`, ergonomic probe handles, and bold circular badge markers:
     - `probeVoltagePos` (`V+`): Vivid crimson `#CF222E` with `+` marker.
     - `probeVoltageNeg` (`V-`): Slate blue `#0550AE` with `-` marker.
   - **Measurement:** `NetlistGenerator::resolve_probes` resolves positive and negative test nodes. `SimulationEngine` computes differential vectors $V_{\text{diff}} = V(+) - V(-)$ for both transient time series and complex AC frequency data, automatically populating the oscilloscope channel list.

3. **Expanded Component Library:**
   - **Switch (SPST):** Two-terminal mechanical/toggle contact with $R_{\text{on}} = 1\text{m}\Omega$ and $R_{\text{off}} = 100\text{M}\Omega$.
   - **Transformer:** Magnetically coupled inductors ($L_1 = 10\text{mH}$, $L_2 = 10\text{mH}$, $K = 0.999$).
   - **AC Current Source (`iac`):** Independent sinusoidal current generator with AC frequency excitation.
   - **Voltage Regulator (`vreg`):** 3-terminal fixed 5V regulator with subcircuit representation.

4. **Reliable AC Simulation Analysis:**
   - Fixed source syntax bug where default `c.value` string `"SIN(0 5 1k)"` or `"PULSE(...)"` caused corrupted nested expressions like `SIN(0 SIN(0 5 1k) 1k)` in SPICE netlists.
   - Implemented regex/string parameter extraction so `SIN(...)`, `PULSE(...)`, `AC <mag> <phase>`, and bare numeric values generate correct SPICE directives.
   - Added automatic AC perturbation stimulus (`AC 1 0`) to independent sources during AC frequency sweeps.
   - Enhanced oscilloscope waveform viewer with logarithmic decade frequency axis (`10Hz`, `100Hz`, `1kHz`, `10kHz`, `100kHz`, `1MHz`) and engineering unit formatting (`formatEng`).

5. **Interface Theme & Typography:**
   - Pure white interface theme (`#FFFFFF` background, `#F6F8FA` sidebars, `#EAEFF5` headers) with high-contrast text (`#1F2328`).
   - Pure **Times New Roman** serif typography rendered across all window labels, buttons, inspectors, statusbars, canvas component labels, and oscilloscope axis markers.

---

### Verified Simulation Benchmarks Summary

1. **Benchmark 1 (Ohm's Law DC):**
   - Setup: $V_1 = 10\text{V}$, $R_1 = 1\text{k}\Omega$.
   - Theoretical current: $I = \frac{10\text{V}}{1000\Omega} = 0.01000\text{ A}$ ($10\text{ mA}$).
   - Status: **implemented and tested** (PASS).

2. **Benchmark 2 (Resistive Voltage Divider):**
   - Setup: $V_{\text{in}} = 12\text{V}$, $R_1 = 3\text{k}\Omega$, $R_2 = 1\text{k}\Omega$.
   - Theoretical output: $V_{\text{out}} = 12\text{V} \cdot \frac{1\text{k}}{3\text{k} + 1\text{k}} = 3.0000\text{V}$.
   - Status: **implemented and tested** (PASS).

3. **Benchmark 3 (RC Step Transient):**
   - Setup: Step pulse $0 \to 10\text{V}$, $R = 1\text{k}\Omega$, $C = 1\mu\text{F}$ ($\tau = RC = 1\text{ ms}$).
   - Theoretical voltage at $t = \tau$: $V(\tau) = 10(1 - e^{-1}) = 6.3212\text{V}$.
   - Status: **implemented and tested** (PASS).

4. **Benchmark 4 (RC Low-Pass AC):**
   - Setup: $V_{\text{ac}} = 1\text{V}$, $R = 1\text{k}\Omega$, $C = 159.155\text{ nF}$.
   - Theoretical cutoff: $f_c = \frac{1}{2\pi RC} \approx 1000\text{ Hz}$, with $|V_{\text{out}}| = 0.707\text{V}$ ($-3\text{ dB}$).
   - Status: **implemented and tested** (PASS).

5. **Benchmark 5 (Differential Voltage Probing):**
   - Setup: $V_{\text{in}} = 12\text{V}$, $R_1 = 3\text{k}\Omega$, $R_2 = 1\text{k}\Omega$. $V_+$ placed at input ($12\text{V}$), $V_-$ placed at divider junction ($3\text{V}$).
   - Theoretical differential voltage: $V(V_+ - V_-) = 12\text{V} - 3\text{V} = 9.0000\text{V}$.
   - Status: **implemented and tested** (PASS).

6. **Circuit Validation Diagnostics:**
   - Ground missing: `NO_GROUND` fatal error (PASS).
   - Duplicate designator: `DUPLICATE_DESIGNATOR` fatal error (PASS).
   - Empty values: `EMPTY_VALUE` fatal error (PASS).
