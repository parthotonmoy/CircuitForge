# CircuitForge Design Journal

## Entry 1: Visual Theme & Anti-Template Calibration (2026-10-19)

### Considered:
- **Option A (Light Drafting Paper):** Technical off-white (`#F8F9FA`) with blue blueprint ink lines.
  - *Evaluation:* Excellent for paper schematics, but fatiguing when observing multi-trace waveform oscilloscopes and high-intensity frequency graphs.
- **Option B (Dark Instrument Finish):** Calibrated deep charcoal canvas (`#0A0D12`) with steel structural framing (`#161B22`) and phosphor trace colors.
  - *Evaluation:* Matches modern benchtop oscilloscopes (Rigol, Keysight, Tektronix). Maximizes waveform visibility and circuit element distinction.
  - *Decision:* Kept Option B as the default dark instrument workspace.

### Typography Refinement:
- Monospace was strictly quarantined to:
  1. Netlist viewer
  2. Diagnostic console output
  3. Numeric measurements and cursor delta readouts (using `tabular-nums`)
- Standard interface actions, dialogs, and component labels use crisp humanist sans-serif (`Inter, system-ui`), preventing the "fake hacker terminal" trope.

### Schematic Linework & Pin Attachment:
- Tested pin snapping: 10px orthogonal grid.
- Wires render as 2px clean solid vectors with sharp 90-degree orthogonal routes.
- Junction dots (radius 3.5px) render *only* at genuine multi-wire intersections (3 or more connections), never on straight 2-point wire passes.
- Ground is always symbolized by the standard 3-tiered horizontal rake symbol attached to Node 0.
