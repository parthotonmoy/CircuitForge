# CircuitForge Design System & User Experience Specification

> Written and validated BEFORE frontend implementation in accordance with Section 10 of the Engineering Brief and anti-template contract.

---

## 1. Design Concept

CircuitForge is designed as a precision laboratory workbench instrument—combining the clarity of classical drafting schematics with the tactile responsiveness of modern benchtop oscilloscopes and spectrum analyzers. It eschews generic SaaS dashboard conventions (floating card grids, decorative rounded corners, glowing neon gradients, and empty promotional heroes) in favor of purposeful technical linework, structured tool strips, calibrated SI notations, and legible tabular typography. The workspace prioritizes the schematic drafting canvas as the primary instrument plane, framed by high-contrast parameter inspection rails and a dedicated waveform analysis deck.

---

## 2. Color Palette & Functional Semantics

The palette uses a calibrated dark instrument finish (`#0D1117`) that minimizes eye fatigue during prolonged schematic capture and waveform tracing, paired with precision trace colors derived from laboratory phosphor standards.

| Token | Hex Value | Purpose & Rationale |
|---|---|---|
| `--surface-canvas` | `#0A0D12` | Deep drafting canvas surface, allowing dark grid markers and bright component vectors to stand out cleanly without glare. |
| `--surface-panel` | `#161B22` | Structured instrument frame, panels, and tool headers. Distinct step from canvas surface. |
| `--surface-elevated`| `#21262D` | Active tool buttons, selected component cards, input fields, and modal dialogues. |
| `--border-subtle` | `#30363D` | Structural hairline dividers (1px solid) encoding logical hierarchy between canvas and side rails. |
| `--border-focus` | `#58A6FF` | High-visibility focus indicator (2px solid) meeting WCAG AAA keyboard accessibility standards. |
| `--text-primary` | `#F0F6FC` | High-contrast technical values, pin labels, and menu text (contrast ratio > 12:1). |
| `--text-muted` | `#8B949E` | Secondary metadata, engineering units, disabled states (contrast ratio > 4.5:1). |
| `--schematic-wire` | `#48D0B0` | Electrical schematic linework and connected nets (phosphor cyan/teal, unmistakable against black canvas). |
| `--schematic-pin` | `#E3B341` | Terminal pin attachment points (amber gold highlight when unconnected). |
| `--schematic-sel` | `#58A6FF` | Active component selection outline and highlighted net vectors. |
| `--trace-1` | `#38D9A9` | Primary waveform trace (Green phosphor). |
| `--trace-2` | `#FF6B6B` | Secondary waveform trace (Calibrated coral red). |
| `--trace-3` | `#4DABF7` | Tertiary waveform trace (Signal blue). |
| `--trace-4` | `#FFD43B` | Quaternary waveform trace (Amber yellow). |
| `--status-error` | `#F85149` | Diagnostic error indicators, floating nodes, convergence warnings. |
| `--status-success`| `#2EA043` | Simulation completion, valid netlist confirmation, saved state. |

---

## 3. Typography & Numerical Representation

- **Interface & Label Font:** Inter, Segoe UI, system-ui (clean humanist sans-serif, legible at small UI sizes).
- **Technical, Netlist & Readout Font:** JetBrains Mono, Fira Code, Consolas (monospace with tabular numbers `font-variant-numeric: tabular-nums`).
- **Rules:**
  - Every numeric readout (voltages, currents, cursor time coordinates, frequency values, simulation progress) strictly utilizes tabular figures to prevent jitter during updates.
  - Consistent SI engineering prefixes: `f` ($10^{-15}$), `p` ($10^{-12}$), `n` ($10^{-9}$), `u` / `µ` ($10^{-6}$), `m` ($10^{-3}$), `k` ($10^3$), `Meg` ($10^6$), `G` ($10^9$), `T` ($10^{12}$).
  - Capitalization: Plain English sentence case for UI actions and dialog headers; uppercase strictly reserved for industry standard SPICE device prefixes (`R`, `C`, `L`, `V`, `I`, `Q`, `D`, `M`, `U`).

---

## 4. Spacing, Corner Radii & Elevation Rules

- **Spacing Scale:** 4px grid (`--space-1: 4px`, `--space-2: 8px`, `--space-3: 12px`, `--space-4: 16px`, `--space-6: 24px`).
- **Corner Radii:** Purpose-driven differentiation rather than uniform roundness:
  - Canvas and viewports: `0px` (sharp architectural boundary).
  - Side panels & status bar: `0px` (flush instrument panels).
  - Input fields & buttons: `4px` (subtle bevel providing tactile target).
  - Floating dialogs / modals: `6px` (contained elevation).
  - Pills / Badges: Never use round capsule pills as decorative elements.
- **Borders & Elevation:**
  - Flat panels defined by explicit 1px hairline dividers (`--border-subtle`), not soft fuzzy drop shadows.

---

## 5. Desktop-First Layout Architecture

```
+----------------------------------------------------------------------------------------------------+
|  CircuitForge  [File]  [Edit]  [View]  [Simulate]  [Help]    [ Run (F5) ] [ Stop ] [ Mode: Edit/Sim ]|
+-------------------+---------------------------------------------------------+----------------------+
| COMPONENT PALETTE | SCHEMATIC CANVAS                                        | PROPERTIES & INSPECT |
| [Passives]        |                                                         | Reference: R1        |
| - Resistor (R)    |        V1               R1            C1                | Value:     10k       |
| - Capacitor (C)   |   +---[ + ]---+-------/\/\/\/\-------+-------||---+     | Tolerance: 1%        |
| - Inductor (L)    |   |   5V      |                  |   |       |    |     | Node (+):  NET_1     |
| [Semiconductors]  |   |           |                  |  ---      |    |     | Node (-):  NET_2     |
| - Diode (D)       |  --- GND     --- GND            (P) V(out)  ---  ---    +----------------------+
| - NPN BJT (Q)     |                                         GND  GND    | SIMULATION PROFILE   |
| - NMOS FET (M)    |                                                         | Type: Transient      |
| [Sources]         |                                                         | Stop: 5ms, Step: 10u |
+-------------------+---------------------------------------------------------+----------------------+
| WAVEFORM & DIAGNOSTICS DECK (Resizable Split)                                                      |
| [Waveforms (2 Traces)]  [DC Operating Points]  [SPICE Netlist]  [Diagnostics Console]             |
|   V(V) |                                                                                           |
|    5.0 |--------+````````\                                                                        |
|    2.5 |        |         \___________ C1: 1.25ms, 3.16V | C2: 3.50ms, 0.42V | Delta: 2.25ms, -2.74V   |
|    0.0 +--------+-------------------------------------------------------------                    |
|        0ms     1ms       2ms       3ms       4ms       5ms                                         |
+----------------------------------------------------------------------------------------------------+
| STATUS: Simulation Successful (Transient, 501 points) | Engine: ngspice-38 (bundled) | Grid: 10px  |
+----------------------------------------------------------------------------------------------------+
```

---

## 6. Anti-Template Review & Verification

| Anti-Template Defect | CircuitForge Check & Defense |
|---|---|
| *Polished welcome screen over empty app* | Eliminated. App opens immediately into an operational workspace with quick template choices (Blank Project, Voltage Divider, RC Filter, RLC Resonant). |
| *Identical rounded cards for unrelated items* | Eliminated. Panels are differentiated by structural grid borders, distinct technical density, and clear role separation. |
| *Arbitrary warm cream/terracotta or neon wash* | Eliminated. Professional calibrated instrument slate/dark steel palette with phosphor trace indicators. |
| *Template chrome: tracked out ALL CAPS, eyebrow tags* | Eliminated. Clean sentence case labels; uppercase only for electrical components (`R1`, `V2`, `GND`). |
| *Motion without purpose (fade/slide animations)* | Eliminated. Instant state rendering; zero lag transitions; motion only for real-time cursor dragging and wire drawing. |
| *Fake or dead buttons* | Eliminated. Every single toolbar button, menu entry, and parameter field is wired to executable code. |

---

## 7. The One Memorable Element

The **Interactive Oscilloscope / Logic Graticule Deck**: an authentic laboratory instrument display with calibrated SI grid divisions, dual delta cursors with live readouts, direct probe attachment to schematic nets, and seamless toggling between time-domain and frequency-domain Bode plots.
