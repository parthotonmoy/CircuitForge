# CircuitForge — Clean-Room SPICE Circuit Workbench

CircuitForge is an independent, open-source desktop electronic circuit design and SPICE simulation workbench for Windows, built with Rust, Tauri v2, and TypeScript.

Inspired by classic electrical engineering simulation workflows (schematic capture, simulation profile configuration, simulation execution, and waveform/operating point inspection), CircuitForge provides an engineering-grade interface backed by an integrated [ngspice](https://ngspice.sourceforge.io/) engine.

---

## Features

- **Interactive Schematic Capture**: 20px precision engineering grid, pan/zoom canvas, orthogonal wire routing, rotation, deletion, and undo/redo history.
- **Component Palette**:
  - *Passives*: Resistors, Capacitors, Inductors, Ground (0V reference).
  - *Sources*: DC Voltage (`VDC`), AC Voltage (`VAC`), Sine (`VSIN`), Pulse (`VPULSE`), DC Current (`IDC`).
  - *Semiconductors & Subcircuits*: Diodes, NPN/PNP BJTs, NMOS/PMOS MOSFETs, Ideal Operational Amplifiers.
- **SPICE Engine Integration**: Bundled 64-bit `ngspice-38` engine running in isolated batch mode.
- **Simulation Analyses**:
  - **DC Operating Point (`.OP`)**: Calculates and tabularizes bias voltages and branch currents.
  - **Transient Analysis (`.TRAN`)**: Time-domain simulation with real-time waveform display.
  - **AC Frequency Sweep (`.AC`)**: Decadic, octaval, and linear frequency-domain analysis with magnitude response calculation.
- **Diagnostics & Netlisting**: Complete SPICE netlist preview, live diagnostics console, and electrical design rule validation (ground node requirement, duplicate designator detection, empty parameter validation).
- **Data Export & Persistence**: Waveform vector export to standard CSV; atomic project saving to `.forge` JSON files.

---

## Verified Benchmarks

The simulation engine and netlist compiler are verified against standard analytical EE benchmarks in `src-tauri/tests/`:

1. **Benchmark 1 (Ohm's Law DC)**: 10V DC source across a 1kΩ resistor produces $I = 10\text{ mA}$ ($0.01000\text{ A}$).
2. **Benchmark 2 (Resistive Voltage Divider)**: 12V DC source across a 3kΩ / 1kΩ divider produces $V_{\text{out}} = 3.0000\text{ V}$.
3. **Benchmark 3 (RC Step Transient)**: 10V pulse into $1\text{ k}\Omega$ and $1\mu\text{F}$ ($\tau = 1\text{ ms}$) reaches $V(\tau) = 10(1 - e^{-1}) = 6.3212\text{ V}$.
4. **Benchmark 4 (RC Low-Pass AC)**: $1\text{ k}\Omega$ resistor and $159.155\text{ nF}$ capacitor produce a cutoff frequency $f_c = \frac{1}{2\pi RC} \approx 1000\text{ Hz}$ with $|V_{\text{out}}| = \frac{1}{\sqrt{2}} \approx 0.707\text{ V}$ ($-3\text{ dB}$).

---

## Building and Running

### Prerequisites

- **Windows 10/11 (64-bit)**
- **Rust Toolchain**: `stable-x86_64-pc-windows-msvc` (Rust 1.80+)
- **Node.js**: Node 20+ and `npm`
- **Visual Studio 2022 Build Tools**: with the *Desktop development with C++* workload and Windows SDK
- **WebView2 Runtime**: Evergreen (pre-installed on Windows 10/11)

### Development Mode

```powershell
# Install frontend dependencies
npm install

# Run the Tauri application in development mode
cargo tauri dev
```

### Running Tests

```powershell
# Run the Rust integration test suite and SPICE engine verification
cd src-tauri
cargo test
```

### Building the Production Installer

```powershell
# Compiles frontend assets and packages the standalone NSIS installer (.exe)
cargo tauri build
```

The resulting installer is generated in `src-tauri/target/release/bundle/nsis/`.

---

## Architecture Overview

```
CircuitForge/
├── src/                    # Frontend (Vite + TypeScript + HTML5 Canvas)
│   ├── main.ts             # Schematic editor, renderer, oscilloscope, and UI bindings
│   ├── api.ts              # Typed Tauri invoke IPC bridge
│   ├── types.ts            # Shared circuit, simulation, and data types
│   └── style.css           # Instrument dark theme design tokens & layout
├── src-tauri/              # Native Shell & SPICE Engine Orchestrator (Rust + Tauri v2)
│   ├── src/
│   │   ├── model.rs        # Strongly typed circuit AST & electrical validation rules
│   │   ├── netlist.rs      # Topological DSU netlist generator (SPICE 3f5 / ngspice syntax)
│   │   ├── engine.rs       # Isolated batch simulation runner & process orchestrator
│   │   ├── rawfile.rs      # High-performance ASCII rawfile parser (real & complex vectors)
│   │   └── lib.rs          # Tauri IPC command registration
│   ├── resources/ngspice/  # Bundled 64-bit ngspice engine binary & libraries
│   └── tests/              # End-to-end analytical SPICE benchmark suite
├── examples/               # Standalone .forge benchmark project files
└── docs/                   # Architectural decisions, design system, and project format specs
```

---

## Legal & Licensing

- **License**: MIT License (see [LICENSE](LICENSE)).
- **Third-Party Licenses**: Detailed attribution for dependencies and bundled components in [THIRD_PARTY_LICENSES](THIRD_PARTY_LICENSES).
- **Clean-Room Attestation**: CircuitForge is an independent open-source project. No reverse-engineered proprietary code, assets, or decompiled intellectual property are included.
