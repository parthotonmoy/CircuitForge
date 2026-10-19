import { api } from './api';
import type {
  Component,
  ComponentPin,
  ComponentType,
  Point,
  Project,
  SimulationPlot,
  SimulationResult,
  Wire,
} from './types';
import { open, save } from '@tauri-apps/plugin-dialog';

// -------------------------------------------------------------
// Component pin geometry matching Rust model.rs
// -------------------------------------------------------------
function getComponentPins(c: Component): ComponentPin[] {
  const rad = (c.rotation * Math.PI) / 180;
  const cosR = Math.round(Math.cos(rad));
  const sinR = Math.round(Math.sin(rad));

  let rawPins: [string, string, number, number][] = [];
  switch (c.componentType) {
    case 'resistor':
    case 'capacitor':
    case 'inductor':
      rawPins = [
        ['p1', '1', -40, 0],
        ['p2', '2', 40, 0],
      ];
      break;
    case 'diode':
    case 'zenerDiode':
    case 'led':
      rawPins = [
        ['anode', '+', -40, 0],
        ['cathode', '-', 40, 0],
      ];
      break;
    case 'potentiometer':
      rawPins = [
        ['term1', '1', -40, 0],
        ['wiper', 'W', 0, -30],
        ['term2', '2', 40, 0],
      ];
      break;
    case 'probeVoltage':
    case 'probeVoltagePos':
    case 'probeVoltageNeg':
      rawPins = [['probe', 'V', 0, 0]];
      break;
    case 'probeCurrent':
      rawPins = [
        ['in', 'I+', -20, 0],
        ['out', 'I-', 20, 0],
      ];
      break;
    case 'switch':
      rawPins = [
        ['p1', '1', -40, 0],
        ['p2', '2', 40, 0],
      ];
      break;
    case 'transformer':
      rawPins = [
        ['p1', 'P+', -30, -30],
        ['p2', 'P-', -30, 30],
        ['s1', 'S+', 30, -30],
        ['s2', 'S-', 30, 30],
      ];
      break;
    case 'vreg':
      rawPins = [
        ['in', 'IN', -30, 0],
        ['gnd', 'GND', 0, 30],
        ['out', 'OUT', 30, 0],
      ];
      break;
    case 'vdc':
    case 'vac':
    case 'vsin':
    case 'vpulse':
    case 'idc':
    case 'iac':
      rawPins = [
        ['pos', '+', 0, -40],
        ['neg', '-', 0, 40],
      ];
      break;
    case 'ground':
      rawPins = [['gnd', '0', 0, -20]];
      break;
    case 'bjtNpn':
    case 'bjtPnp':
      rawPins = [
        ['base', 'B', -30, 0],
        ['collector', 'C', 20, -30],
        ['emitter', 'E', 20, 30],
      ];
      break;
    case 'nmos':
    case 'pmos':
      rawPins = [
        ['gate', 'G', -30, 0],
        ['drain', 'D', 20, -30],
        ['source', 'S', 20, 30],
      ];
      break;
    case 'opAmp':
      rawPins = [
        ['in_inv', '-', -40, -20],
        ['in_noninv', '+', -40, 20],
        ['out', 'OUT', 40, 0],
      ];
      break;
  }

  return rawPins.map(([id, name, ox, oy]) => {
    const mx = c.mirror ? -ox : ox;
    const rx = mx * cosR - oy * sinR;
    const ry = mx * sinR + oy * cosR;
    return {
      id,
      name,
      offsetX: rx,
      offsetY: ry,
    };
  });
}

// -------------------------------------------------------------
// State Management
// -------------------------------------------------------------
class WorkbenchState {
  project: Project = {
    schemaVersion: '1.0.0',
    name: 'Untitled Circuit',
    created: new Date().toISOString(),
    modified: new Date().toISOString(),
    simulation: {
      analysisType: 'transient',
      transient: {
        step: '10u',
        stop: '5m',
        start: '0',
        maxStep: '10u',
      },
      ac: {
        sweepType: 'decade',
        points: 50,
        startFreq: '10',
        stopFreq: '1Meg',
      },
      dcOp: true,
    },
    circuit: {
      components: [],
      wires: [],
      junctions: [],
      netLabels: [],
    },
  };

  selectedComponentId: string | null = null;
  selectedWireId: string | null = null;
  activeTool: 'select' | 'wire' | 'place' = 'select';
  pendingPlacementType: ComponentType | null = null;

  // Viewport transform
  panX = 100;
  panY = 100;
  zoom = 1.0;
  gridPitch = 20;

  // Wire creation state
  wireStartPoint: Point | null = null;
  wireCurrentPoint: Point | null = null;

  // History stack for Undo/Redo
  undoStack: string[] = [];
  redoStack: string[] = [];

  // Active Simulation Result
  simResult: SimulationResult | null = null;

  // Oscilloscope State
  scopeActiveChannels: Set<string> = new Set();
  scopeMouseX: number | null = null;
  scopeMouseY: number | null = null;
  scopeZoomX: number = 1.0;
  scopePanX: number = 0;
  scopeGridMode: 'linear' | 'log' = 'linear';

  recordSnapshot() {
    this.undoStack.push(JSON.stringify(this.project.circuit));
    this.redoStack = [];
    if (this.undoStack.length > 50) this.undoStack.shift();
  }

  undo() {
    if (this.undoStack.length === 0) return;
    this.redoStack.push(JSON.stringify(this.project.circuit));
    const previous = this.undoStack.pop()!;
    this.project.circuit = JSON.parse(previous);
    this.selectedComponentId = null;
    this.selectedWireId = null;
  }

  redo() {
    if (this.redoStack.length === 0) return;
    this.undoStack.push(JSON.stringify(this.project.circuit));
    const next = this.redoStack.pop()!;
    this.project.circuit = JSON.parse(next);
    this.selectedComponentId = null;
    this.selectedWireId = null;
  }
}

const state = new WorkbenchState();

// -------------------------------------------------------------
// UI Element References
// -------------------------------------------------------------
const schematicCanvas = document.getElementById('schematic-canvas') as HTMLCanvasElement;
const plotCanvas = document.getElementById('plot-canvas') as HTMLCanvasElement;
const ctxSchematic = schematicCanvas.getContext('2d')!;
const ctxPlot = plotCanvas.getContext('2d')!;

const propDesignator = document.getElementById('prop-designator') as HTMLInputElement;
const propValue = document.getElementById('prop-value') as HTMLInputElement;
const propRotation = document.getElementById('prop-rotation') as HTMLInputElement;

const simTypeSelect = document.getElementById('sim-type') as HTMLSelectElement;
const simTranOptions = document.getElementById('sim-tran-options') as HTMLElement;
const simAcOptions = document.getElementById('sim-ac-options') as HTMLElement;
const simTranStop = document.getElementById('sim-tran-stop') as HTMLInputElement;
const simTranStep = document.getElementById('sim-tran-step') as HTMLInputElement;
const simAcStart = document.getElementById('sim-ac-start') as HTMLInputElement;
const simAcStop = document.getElementById('sim-ac-stop') as HTMLInputElement;
const simAcPoints = document.getElementById('sim-ac-points') as HTMLInputElement;

const statusMessage = document.getElementById('status-message') as HTMLElement;
const statusGrid = document.getElementById('status-grid') as HTMLElement;
const statusEngineDot = document.getElementById('status-engine-dot') as HTMLElement;
const netlistViewer = document.getElementById('netlist-viewer') as HTMLElement;
const consoleViewer = document.getElementById('console-viewer') as HTMLElement;
const opTableBody = document.getElementById('op-table-body') as HTMLElement;

// -------------------------------------------------------------
// Coordinate & Geometry Utilities
// -------------------------------------------------------------
function snapToGrid(val: number, pitch: number): number {
  return Math.round(val / pitch) * pitch;
}

function screenToWorld(sx: number, sy: number): Point {
  return {
    x: snapToGrid((sx - state.panX) / state.zoom, state.gridPitch),
    y: snapToGrid((sy - state.panY) / state.zoom, state.gridPitch),
  };
}

function worldToScreen(wx: number, wy: number): Point {
  return {
    x: wx * state.zoom + state.panX,
    y: wy * state.zoom + state.panY,
  };
}

// -------------------------------------------------------------
// Rendering: Schematic Canvas
// -------------------------------------------------------------
function resizeCanvases() {
  const container = schematicCanvas.parentElement;
  if (container) {
    schematicCanvas.width = container.clientWidth;
    schematicCanvas.height = container.clientHeight;
  }
  const plotContainer = plotCanvas.parentElement;
  if (plotContainer) {
    plotCanvas.width = plotContainer.clientWidth;
    plotCanvas.height = plotContainer.clientHeight;
  }
  renderSchematic();
  renderWaveforms();
}

window.addEventListener('resize', resizeCanvases);

function renderGrid() {
  const width = schematicCanvas.width;
  const height = schematicCanvas.height;
  const pitch = state.gridPitch * state.zoom;

  ctxSchematic.fillStyle = '#FFFFFF';
  ctxSchematic.fillRect(0, 0, width, height);

  ctxSchematic.fillStyle = '#D0D7DE';
  const startX = (state.panX % pitch);
  const startY = (state.panY % pitch);

  for (let x = startX; x < width; x += pitch) {
    for (let y = startY; y < height; y += pitch) {
      ctxSchematic.fillRect(x - 0.75, y - 0.75, 1.5, 1.5);
    }
  }
}

function drawCircle(ctx: CanvasRenderingContext2D, cx: number, cy: number, r: number) {
  ctx.moveTo(cx + r, cy);
  ctx.arc(cx, cy, r, 0, Math.PI * 2);
}

function renderComponentSymbol(c: Component) {
  const isSelected = c.id === state.selectedComponentId;
  const screenPos = worldToScreen(c.x, c.y);

  ctxSchematic.save();
  ctxSchematic.translate(screenPos.x, screenPos.y);
  ctxSchematic.scale(state.zoom, state.zoom);
  ctxSchematic.rotate((c.rotation * Math.PI) / 180);
  if (c.mirror) {
    ctxSchematic.scale(-1, 1);
  }

  ctxSchematic.strokeStyle = isSelected ? '#CF222E' : '#0550AE';
  ctxSchematic.lineWidth = isSelected ? 2.5 : 1.75;
  ctxSchematic.lineJoin = 'round';
  ctxSchematic.lineCap = 'round';

  ctxSchematic.beginPath();

  switch (c.componentType) {
    case 'resistor': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-24, 0);
      const points = [-20, -10, -12, 10, -4, -10, 4, 10, 12, -10, 20, 10, 24, 0];
      for (let i = 0; i < points.length; i += 2) {
        ctxSchematic.lineTo(points[i], points[i + 1]);
      }
      ctxSchematic.lineTo(40, 0);
      break;
    }
    case 'potentiometer': {
      // Resistor body
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-24, 0);
      const points = [-20, -10, -12, 10, -4, -10, 4, 10, 12, -10, 20, 10, 24, 0];
      for (let i = 0; i < points.length; i += 2) {
        ctxSchematic.lineTo(points[i], points[i + 1]);
      }
      ctxSchematic.lineTo(40, 0);
      // Wiper arrow pointing from top
      ctxSchematic.moveTo(0, -30);
      ctxSchematic.lineTo(0, -12);
      ctxSchematic.lineTo(-4, -18);
      ctxSchematic.moveTo(0, -12);
      ctxSchematic.lineTo(4, -18);
      break;
    }
    case 'capacitor': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-8, 0);
      ctxSchematic.moveTo(-8, -16);
      ctxSchematic.lineTo(-8, 16);

      ctxSchematic.moveTo(8, -16);
      ctxSchematic.lineTo(8, 16);
      ctxSchematic.moveTo(8, 0);
      ctxSchematic.lineTo(40, 0);
      break;
    }
    case 'inductor': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-24, 0);
      for (let i = 0; i < 3; i++) {
        ctxSchematic.arc(-16 + i * 16, 0, 8, Math.PI, 0, false);
      }
      ctxSchematic.moveTo(24, 0);
      ctxSchematic.lineTo(40, 0);
      break;
    }
    case 'diode': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-12, 0);
      ctxSchematic.moveTo(12, 0);
      ctxSchematic.lineTo(40, 0);
      ctxSchematic.moveTo(-12, -12);
      ctxSchematic.lineTo(-12, 12);
      ctxSchematic.lineTo(12, 0);
      ctxSchematic.closePath();
      ctxSchematic.moveTo(12, -12);
      ctxSchematic.lineTo(12, 12);
      break;
    }
    case 'zenerDiode': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-12, 0);
      ctxSchematic.moveTo(12, 0);
      ctxSchematic.lineTo(40, 0);
      ctxSchematic.moveTo(-12, -12);
      ctxSchematic.lineTo(-12, 12);
      ctxSchematic.lineTo(12, 0);
      ctxSchematic.closePath();
      // Zener cathode wings
      ctxSchematic.moveTo(8, -15);
      ctxSchematic.lineTo(12, -12);
      ctxSchematic.lineTo(12, 12);
      ctxSchematic.lineTo(16, 15);
      break;
    }
    case 'led': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-12, 0);
      ctxSchematic.moveTo(12, 0);
      ctxSchematic.lineTo(40, 0);
      ctxSchematic.moveTo(-12, -12);
      ctxSchematic.lineTo(-12, 12);
      ctxSchematic.lineTo(12, 0);
      ctxSchematic.closePath();
      ctxSchematic.moveTo(12, -12);
      ctxSchematic.lineTo(12, 12);
      // Emission rays
      ctxSchematic.moveTo(0, -14);
      ctxSchematic.lineTo(8, -22);
      ctxSchematic.moveTo(6, -14);
      ctxSchematic.lineTo(14, -22);
      break;
    }
    case 'ground': {
      ctxSchematic.moveTo(0, -20);
      ctxSchematic.lineTo(0, 0);
      ctxSchematic.moveTo(-16, 0);
      ctxSchematic.lineTo(16, 0);
      ctxSchematic.moveTo(-10, 6);
      ctxSchematic.lineTo(10, 6);
      ctxSchematic.moveTo(-4, 12);
      ctxSchematic.lineTo(4, 12);
      break;
    }
    case 'vdc': {
      // Standard battery/cell DC source symbol
      ctxSchematic.moveTo(0, -40);
      ctxSchematic.lineTo(0, -10);
      ctxSchematic.moveTo(-18, -10);
      ctxSchematic.lineTo(18, -10); // Long positive bar
      ctxSchematic.moveTo(-10, 0);
      ctxSchematic.lineTo(10, 0);   // Short negative bar
      ctxSchematic.moveTo(-18, 10);
      ctxSchematic.lineTo(18, 10);  // Long bar
      ctxSchematic.moveTo(-10, 20);
      ctxSchematic.lineTo(10, 20);  // Short bar
      ctxSchematic.moveTo(0, 20);
      ctxSchematic.lineTo(0, 40);
      // Polarity signs
      ctxSchematic.moveTo(-8, -24);
      ctxSchematic.lineTo(-2, -24);
      ctxSchematic.moveTo(-5, -27);
      ctxSchematic.lineTo(-5, -21);
      ctxSchematic.moveTo(-8, 30);
      ctxSchematic.lineTo(-2, 30);
      break;
    }
    case 'vac':
    case 'vsin': {
      ctxSchematic.moveTo(0, -40);
      ctxSchematic.lineTo(0, -18);
      ctxSchematic.moveTo(0, 18);
      ctxSchematic.lineTo(0, 40);
      drawCircle(ctxSchematic, 0, 0, 18);
      // Sine wave inside
      ctxSchematic.moveTo(-10, 0);
      ctxSchematic.quadraticCurveTo(-5, -9, 0, 0);
      ctxSchematic.quadraticCurveTo(5, 9, 10, 0);
      break;
    }
    case 'vpulse': {
      ctxSchematic.moveTo(0, -40);
      ctxSchematic.lineTo(0, -18);
      ctxSchematic.moveTo(0, 18);
      ctxSchematic.lineTo(0, 40);
      drawCircle(ctxSchematic, 0, 0, 18);
      // Square pulse wave inside
      ctxSchematic.moveTo(-9, 5);
      ctxSchematic.lineTo(-4, 5);
      ctxSchematic.lineTo(-4, -5);
      ctxSchematic.lineTo(4, -5);
      ctxSchematic.lineTo(4, 5);
      ctxSchematic.lineTo(9, 5);
      break;
    }
    case 'idc': {
      ctxSchematic.moveTo(0, -40);
      ctxSchematic.lineTo(0, -18);
      ctxSchematic.moveTo(0, 18);
      ctxSchematic.lineTo(0, 40);
      drawCircle(ctxSchematic, 0, 0, 18);
      // Arrow
      ctxSchematic.moveTo(0, 10);
      ctxSchematic.lineTo(0, -10);
      ctxSchematic.lineTo(-4, -4);
      ctxSchematic.moveTo(0, -10);
      ctxSchematic.lineTo(4, -4);
      break;
    }
    case 'iac': {
      ctxSchematic.moveTo(0, -40);
      ctxSchematic.lineTo(0, -18);
      ctxSchematic.moveTo(0, 18);
      ctxSchematic.lineTo(0, 40);
      drawCircle(ctxSchematic, 0, 0, 18);
      // Sine wave inside
      ctxSchematic.moveTo(-10, 0);
      ctxSchematic.quadraticCurveTo(-5, -8, 0, 0);
      ctxSchematic.quadraticCurveTo(5, 8, 10, 0);
      // Directional arrow on the right
      ctxSchematic.moveTo(6, 6);
      ctxSchematic.lineTo(6, -6);
      ctxSchematic.lineTo(3, -2);
      ctxSchematic.moveTo(6, -6);
      ctxSchematic.lineTo(9, -2);
      break;
    }
    case 'switch': {
      ctxSchematic.moveTo(-40, 0);
      ctxSchematic.lineTo(-18, 0);
      drawCircle(ctxSchematic, -16, 0, 2.5);
      drawCircle(ctxSchematic, 16, 0, 2.5);
      ctxSchematic.moveTo(18, 0);
      ctxSchematic.lineTo(40, 0);
      const isClosed = c.properties.state === 'closed' || c.properties.state === 'on';
      if (isClosed) {
        ctxSchematic.moveTo(-13.5, 0);
        ctxSchematic.lineTo(13.5, 0);
      } else {
        ctxSchematic.moveTo(-13.5, 0);
        ctxSchematic.lineTo(12, -14);
      }
      break;
    }
    case 'transformer': {
      // Primary coil
      ctxSchematic.moveTo(-30, -30);
      ctxSchematic.lineTo(-18, -30);
      for (let i = 0; i < 3; i++) {
        ctxSchematic.moveTo(-18, -20 + i * 16);
        ctxSchematic.arc(-18, -12 + i * 16, 8, -Math.PI / 2, Math.PI / 2, false);
      }
      ctxSchematic.moveTo(-18, 30);
      ctxSchematic.lineTo(-30, 30);

      // Core lines
      ctxSchematic.moveTo(-4, -26);
      ctxSchematic.lineTo(-4, 26);
      ctxSchematic.moveTo(4, -26);
      ctxSchematic.lineTo(4, 26);

      // Secondary coil
      ctxSchematic.moveTo(30, -30);
      ctxSchematic.lineTo(18, -30);
      for (let i = 0; i < 3; i++) {
        ctxSchematic.moveTo(18, -20 + i * 16);
        ctxSchematic.arc(18, -12 + i * 16, 8, -Math.PI / 2, Math.PI / 2, true);
      }
      ctxSchematic.moveTo(18, 30);
      ctxSchematic.lineTo(30, 30);
      break;
    }
    case 'vreg': {
      ctxSchematic.moveTo(-30, 0);
      ctxSchematic.lineTo(-20, 0);
      ctxSchematic.moveTo(20, 0);
      ctxSchematic.lineTo(30, 0);
      ctxSchematic.moveTo(0, 16);
      ctxSchematic.lineTo(0, 30);
      ctxSchematic.strokeRect(-20, -16, 40, 32);
      break;
    }
    case 'probeVoltage':
    case 'probeVoltagePos': {
      // Red Positive Test Point Probe (needle tip at (0, 0), badge with +)
      ctxSchematic.moveTo(0, 0);
      ctxSchematic.lineTo(-5, -8);
      ctxSchematic.moveTo(-5, -8);
      ctxSchematic.lineTo(-14, -22);
      drawCircle(ctxSchematic, -22, -34, 12);
      // Bold '+' cross inside badge
      ctxSchematic.moveTo(-26, -34);
      ctxSchematic.lineTo(-18, -34);
      ctxSchematic.moveTo(-22, -38);
      ctxSchematic.lineTo(-22, -30);
      break;
    }
    case 'probeVoltageNeg': {
      // Blue/Dark Negative Reference Probe (needle tip at (0, 0), badge with -)
      ctxSchematic.moveTo(0, 0);
      ctxSchematic.lineTo(-5, -8);
      ctxSchematic.moveTo(-5, -8);
      ctxSchematic.lineTo(-14, -22);
      drawCircle(ctxSchematic, -22, -34, 12);
      // Bold '-' bar inside badge
      ctxSchematic.moveTo(-26, -34);
      ctxSchematic.lineTo(-18, -34);
      break;
    }
    case 'probeCurrent': {
      ctxSchematic.moveTo(-20, 0);
      ctxSchematic.lineTo(-12, 0);
      ctxSchematic.moveTo(12, 0);
      ctxSchematic.lineTo(20, 0);
      drawCircle(ctxSchematic, 0, 0, 12);
      ctxSchematic.moveTo(-6, 0);
      ctxSchematic.lineTo(6, 0);
      ctxSchematic.lineTo(2, -4);
      ctxSchematic.moveTo(6, 0);
      ctxSchematic.lineTo(2, 4);
      break;
    }
    case 'bjtNpn': {
      drawCircle(ctxSchematic, 0, 0, 22);
      ctxSchematic.moveTo(-30, 0);
      ctxSchematic.lineTo(-10, 0);
      ctxSchematic.moveTo(-10, -16);
      ctxSchematic.lineTo(-10, 16);
      ctxSchematic.moveTo(-10, -8);
      ctxSchematic.lineTo(20, -30);
      ctxSchematic.moveTo(-10, 8);
      ctxSchematic.lineTo(20, 30);
      ctxSchematic.lineTo(12, 28);
      ctxSchematic.moveTo(20, 30);
      ctxSchematic.lineTo(16, 20);
      break;
    }
    case 'bjtPnp': {
      drawCircle(ctxSchematic, 0, 0, 22);
      ctxSchematic.moveTo(-30, 0);
      ctxSchematic.lineTo(-10, 0);
      ctxSchematic.moveTo(-10, -16);
      ctxSchematic.lineTo(-10, 16);
      ctxSchematic.moveTo(-10, -8);
      ctxSchematic.lineTo(20, -30);
      ctxSchematic.moveTo(-10, 8);
      ctxSchematic.lineTo(20, 30);
      // Inward arrow
      ctxSchematic.moveTo(0, 15);
      ctxSchematic.lineTo(-5, 11);
      ctxSchematic.lineTo(1, 7);
      break;
    }
    case 'nmos': {
      ctxSchematic.moveTo(-30, 0);
      ctxSchematic.lineTo(-12, 0);
      ctxSchematic.moveTo(-12, -18);
      ctxSchematic.lineTo(-12, 18);
      ctxSchematic.moveTo(-6, -16);
      ctxSchematic.lineTo(-6, 16);
      ctxSchematic.moveTo(-6, -14);
      ctxSchematic.lineTo(20, -30);
      ctxSchematic.moveTo(-6, 14);
      ctxSchematic.lineTo(20, 30);
      break;
    }
    case 'pmos': {
      ctxSchematic.moveTo(-30, 0);
      ctxSchematic.lineTo(-16, 0);
      ctxSchematic.arc(-14, 0, 2, 0, Math.PI * 2);
      ctxSchematic.moveTo(-12, -18);
      ctxSchematic.lineTo(-12, 18);
      ctxSchematic.moveTo(-6, -16);
      ctxSchematic.lineTo(-6, 16);
      ctxSchematic.moveTo(-6, -14);
      ctxSchematic.lineTo(20, -30);
      ctxSchematic.moveTo(-6, 14);
      ctxSchematic.lineTo(20, 30);
      break;
    }
    case 'opAmp': {
      ctxSchematic.moveTo(-40, -30);
      ctxSchematic.lineTo(-40, 30);
      ctxSchematic.lineTo(30, 0);
      ctxSchematic.closePath();
      ctxSchematic.moveTo(-40, -20);
      ctxSchematic.lineTo(-26, -20);
      ctxSchematic.moveTo(-40, 20);
      ctxSchematic.lineTo(-26, 20);
      ctxSchematic.moveTo(30, 0);
      ctxSchematic.lineTo(40, 0);
      ctxSchematic.moveTo(-34, -20);
      ctxSchematic.lineTo(-28, -20);
      ctxSchematic.moveTo(-34, 20);
      ctxSchematic.lineTo(-28, 20);
      ctxSchematic.moveTo(-31, 17);
      ctxSchematic.lineTo(-31, 23);
      break;
    }
    default:
      ctxSchematic.strokeRect(-20, -20, 40, 40);
      break;
  }
  ctxSchematic.stroke();

  // Pins indicators
  const pins = getComponentPins(c);
  ctxSchematic.restore();

  // Render pin anchor dots
  for (const pin of pins) {
    const pWorld = { x: c.x + pin.offsetX, y: c.y + pin.offsetY };
    const pScreen = worldToScreen(pWorld.x, pWorld.y);
    ctxSchematic.beginPath();
    ctxSchematic.fillStyle = '#9A6700';
    ctxSchematic.arc(pScreen.x, pScreen.y, 3 * Math.max(0.6, state.zoom), 0, Math.PI * 2);
    ctxSchematic.fill();
  }

  // Draw Designator and Value text with Times New Roman font
  ctxSchematic.save();
  ctxSchematic.font = `bold ${Math.max(11, 13 * state.zoom)}px "Times New Roman", Times, serif`;
  ctxSchematic.fillStyle = isSelected ? '#CF222E' : '#1F2328';
  const textScreen = worldToScreen(c.x + 12, c.y - 14);
  ctxSchematic.fillText(c.designator, textScreen.x, textScreen.y);
  if (
    c.componentType !== 'ground' &&
    c.componentType !== 'probeVoltage' &&
    c.componentType !== 'probeVoltagePos' &&
    c.componentType !== 'probeVoltageNeg'
  ) {
    ctxSchematic.font = `${Math.max(10, 12 * state.zoom)}px "Times New Roman", Times, serif`;
    ctxSchematic.fillStyle = '#59636E';
    ctxSchematic.fillText(c.value, textScreen.x, textScreen.y + 14 * state.zoom);
  }
  ctxSchematic.restore();
}

function renderWires() {
  ctxSchematic.save();
  ctxSchematic.lineCap = 'round';
  ctxSchematic.lineJoin = 'round';

  for (const w of state.project.circuit.wires) {
    if (w.points.length < 2) continue;
    const isSelected = w.id === state.selectedWireId;
    ctxSchematic.strokeStyle = isSelected ? '#CF222E' : '#0550AE';
    ctxSchematic.lineWidth = (isSelected ? 2.5 : 1.75) * state.zoom;

    ctxSchematic.beginPath();
    const p0 = worldToScreen(w.points[0].x, w.points[0].y);
    ctxSchematic.moveTo(p0.x, p0.y);
    for (let i = 1; i < w.points.length; i++) {
      const p = worldToScreen(w.points[i].x, w.points[i].y);
      ctxSchematic.lineTo(p.x, p.y);
    }
    ctxSchematic.stroke();
  }

  // Draw in-progress wire
  if (state.activeTool === 'wire' && state.wireStartPoint && state.wireCurrentPoint) {
    ctxSchematic.strokeStyle = '#0969DA';
    ctxSchematic.lineWidth = 1.75 * state.zoom;
    ctxSchematic.setLineDash([4 * state.zoom, 4 * state.zoom]);

    const pStart = worldToScreen(state.wireStartPoint.x, state.wireStartPoint.y);
    const pEnd = worldToScreen(state.wireCurrentPoint.x, state.wireCurrentPoint.y);

    // Orthogonal routing (horizontal first, then vertical)
    const pCorner = worldToScreen(state.wireCurrentPoint.x, state.wireStartPoint.y);

    ctxSchematic.beginPath();
    ctxSchematic.moveTo(pStart.x, pStart.y);
    ctxSchematic.lineTo(pCorner.x, pCorner.y);
    ctxSchematic.lineTo(pEnd.x, pEnd.y);
    ctxSchematic.stroke();
    ctxSchematic.setLineDash([]);
  }

  // Render Junction nodes
  for (const j of state.project.circuit.junctions) {
    const sj = worldToScreen(j.x, j.y);
    ctxSchematic.beginPath();
    ctxSchematic.fillStyle = '#0550AE';
    ctxSchematic.arc(sj.x, sj.y, 4 * state.zoom, 0, Math.PI * 2);
    ctxSchematic.fill();
  }

  ctxSchematic.restore();
}

function renderSchematic() {
  ctxSchematic.clearRect(0, 0, schematicCanvas.width, schematicCanvas.height);
  renderGrid();
  renderWires();
  for (const comp of state.project.circuit.components) {
    renderComponentSymbol(comp);
  }
}

// -------------------------------------------------------------
// Interactive Waveform & Oscilloscope Engine
// -------------------------------------------------------------
function formatEng(val: number, unit: string = ''): string {
  if (Math.abs(val) < 1e-15) return '0' + unit;
  const abs = Math.abs(val);
  const sign = val < 0 ? '-' : '';
  if (abs >= 1e9) return sign + (abs / 1e9).toPrecision(3) + 'G' + unit;
  if (abs >= 1e6) return sign + (abs / 1e6).toPrecision(3) + 'M' + unit;
  if (abs >= 1e3) return sign + (abs / 1e3).toPrecision(3) + 'k' + unit;
  if (abs >= 1) return sign + abs.toPrecision(3) + unit;
  if (abs >= 1e-3) return sign + (abs * 1e3).toPrecision(3) + 'm' + unit;
  if (abs >= 1e-6) return sign + (abs * 1e6).toPrecision(3) + 'u' + unit;
  if (abs >= 1e-9) return sign + (abs * 1e9).toPrecision(3) + 'n' + unit;
  return sign + abs.toPrecision(3) + unit;
}

function updateScopeChannelPills(keys: string[]) {
  const container = document.getElementById('scope-channel-pills');
  if (!container) return;

  // Initialize active channels and ensure differential probe vectors are active
  if (state.scopeActiveChannels.size === 0) {
    keys.forEach((k) => state.scopeActiveChannels.add(k));
  } else {
    keys.forEach((k) => {
      if (k.startsWith('V(VP')) {
        state.scopeActiveChannels.add(k);
      }
    });
  }

  container.innerHTML = '';
  const traceColors = ['#0969DA', '#CF222E', '#1A7F37', '#8250DF', '#D4A72C'];

  keys.forEach((key, idx) => {
    const pill = document.createElement('button');
    const isActive = state.scopeActiveChannels.has(key);
    const color = traceColors[idx % traceColors.length];
    pill.style.padding = '1px 8px';
    pill.style.fontSize = '11px';
    pill.style.borderRadius = '3px';
    pill.style.fontFamily = 'var(--font-serif)';
    pill.style.display = 'inline-flex';
    pill.style.alignItems = 'center';
    pill.style.gap = '4px';

    if (isActive) {
      pill.style.backgroundColor = `${color}18`;
      pill.style.borderColor = color;
      pill.style.color = color;
      pill.style.fontWeight = 'bold';
    } else {
      pill.style.backgroundColor = '#FAFAFA';
      pill.style.borderColor = '#D0D7DE';
      pill.style.color = '#8C959F';
    }

    pill.innerHTML = `<span style="display:inline-block; width:7px; height:7px; border-radius:50%; background:${isActive ? color : '#8C959F'};"></span> ${key}`;
    pill.addEventListener('click', () => {
      if (state.scopeActiveChannels.has(key)) {
        if (state.scopeActiveChannels.size > 1) {
          state.scopeActiveChannels.delete(key);
        }
      } else {
        state.scopeActiveChannels.add(key);
      }
      updateScopeChannelPills(keys);
      renderWaveforms();
    });
    container.appendChild(pill);
  });
}

function renderWaveforms() {
  const width = plotCanvas.width;
  const height = plotCanvas.height;

  // Pure white lab oscilloscope background
  ctxPlot.fillStyle = '#FFFFFF';
  ctxPlot.fillRect(0, 0, width, height);

  if (!state.simResult || !state.simResult.plots || state.simResult.plots.length === 0) {
    ctxPlot.fillStyle = '#59636E';
    ctxPlot.font = '14px "Times New Roman", Times, serif';
    ctxPlot.textAlign = 'center';
    ctxPlot.fillText(
      'Run simulation to generate and inspect waveform vectors.',
      width / 2,
      height / 2
    );
    return;
  }

  const plot: SimulationPlot = state.simResult.plots[0];
  const allKeys = Object.keys(plot.realData);
  if (allKeys.length < 2) return;

  const xKey = allKeys[0]; // Independent variable: time or frequency
  const rawXValues = plot.realData[xKey];
  const allYKeys = allKeys.slice(1);

  // Update filter pills
  updateScopeChannelPills(allYKeys);

  const activeYKeys = allYKeys.filter((k) => state.scopeActiveChannels.has(k));
  if (activeYKeys.length === 0) return;

  const padding = { left: 75, right: 35, top: 20, bottom: 35 };
  const plotW = width - padding.left - padding.right;
  const plotH = height - padding.top - padding.bottom;

  let minX = Math.min(...rawXValues);
  let maxX = Math.max(...rawXValues);
  if (minX === maxX) maxX += 1;

  const isLogX = (xKey === 'frequency' || state.scopeGridMode === 'log') && minX > 0 && maxX > minX;
  const logMinX = isLogX ? Math.log10(minX) : 0;
  const logMaxX = isLogX ? Math.log10(maxX) : 1;

  const getNormX = (x: number) => {
    if (isLogX) {
      return (Math.log10(Math.max(x, minX)) - logMinX) / (logMaxX - logMinX);
    }
    return (x - minX) / (maxX - minX);
  };

  let minY = Infinity;
  let maxY = -Infinity;
  for (const k of activeYKeys) {
    const vals = plot.realData[k];
    for (const v of vals) {
      if (v < minY) minY = v;
      if (v > maxY) maxY = v;
    }
  }

  if (minY === maxY) {
    minY -= 1;
    maxY += 1;
  }

  // Draw Subtle Oscilloscope Division Grid
  ctxPlot.strokeStyle = '#E1E4E8';
  ctxPlot.lineWidth = 1;
  ctxPlot.strokeRect(padding.left, padding.top, plotW, plotH);

  // Minor sub-grid dots
  ctxPlot.fillStyle = '#E8ECEF';
  const subDivX = 20;
  const subDivY = 10;
  for (let sx = 0; sx <= subDivX; sx++) {
    for (let sy = 0; sy <= subDivY; sy++) {
      const gx = padding.left + (plotW * sx) / subDivX;
      const gy = padding.top + (plotH * sy) / subDivY;
      ctxPlot.fillRect(gx - 0.5, gy - 0.5, 1, 1);
    }
  }

  // Horizontal Grid Lines & Y-Axis Labels
  ctxPlot.fillStyle = '#59636E';
  ctxPlot.font = '11px "Times New Roman", Times, serif';
  ctxPlot.textAlign = 'right';

  const gridRows = 6;
  for (let r = 0; r <= gridRows; r++) {
    const yVal = minY + (maxY - minY) * (1 - r / gridRows);
    const yPos = padding.top + (plotH * r) / gridRows;

    ctxPlot.beginPath();
    ctxPlot.strokeStyle = '#E1E4E8';
    ctxPlot.moveTo(padding.left, yPos);
    ctxPlot.lineTo(padding.left + plotW, yPos);
    ctxPlot.stroke();

    ctxPlot.fillText(formatEng(yVal, 'V'), padding.left - 8, yPos + 4);
  }

  // Vertical Grid Lines & X-Axis Labels
  ctxPlot.textAlign = 'center';
  ctxPlot.font = '11px "Times New Roman", Times, serif';

  if (isLogX) {
    // Logarithmic decade markers
    const startDecade = Math.floor(logMinX);
    const endDecade = Math.ceil(logMaxX);
    for (let dec = startDecade; dec <= endDecade; dec++) {
      const freqVal = Math.pow(10, dec);
      if (freqVal >= minX * 0.99 && freqVal <= maxX * 1.01) {
        const xNorm = getNormX(freqVal);
        const xPos = padding.left + xNorm * plotW;

        ctxPlot.beginPath();
        ctxPlot.strokeStyle = '#D0D7DE';
        ctxPlot.lineWidth = 1.25;
        ctxPlot.moveTo(xPos, padding.top);
        ctxPlot.lineTo(xPos, padding.top + plotH);
        ctxPlot.stroke();

        ctxPlot.fillText(formatEng(freqVal, 'Hz'), xPos, padding.top + plotH + 18);
      }
    }
  } else {
    const gridCols = 8;
    for (let c = 0; c <= gridCols; c++) {
      const xVal = minX + (maxX - minX) * (c / gridCols);
      const xPos = padding.left + (plotW * c) / gridCols;

      ctxPlot.beginPath();
      ctxPlot.strokeStyle = '#E1E4E8';
      ctxPlot.moveTo(xPos, padding.top);
      ctxPlot.lineTo(xPos, padding.top + plotH);
      ctxPlot.stroke();

      ctxPlot.fillText(formatEng(xVal, xKey === 'time' ? 's' : ''), xPos, padding.top + plotH + 18);
    }
  }

  // Trace Colors
  const traceColors = ['#0969DA', '#CF222E', '#1A7F37', '#8250DF', '#D4A72C'];

  // Plot Traces
  activeYKeys.forEach((k) => {
    const originalIndex = allYKeys.indexOf(k);
    const vals = plot.realData[k];
    const color = traceColors[originalIndex % traceColors.length];

    ctxPlot.strokeStyle = color;
    ctxPlot.lineWidth = 2.0;
    ctxPlot.beginPath();

    for (let i = 0; i < rawXValues.length; i++) {
      const xNorm = getNormX(rawXValues[i]);
      const yNorm = (vals[i] - minY) / (maxY - minY);

      const px = padding.left + xNorm * plotW;
      const py = padding.top + (1 - yNorm) * plotH;

      if (i === 0) ctxPlot.moveTo(px, py);
      else ctxPlot.lineTo(px, py);
    }
    ctxPlot.stroke();
  });

  // Oscilloscope Interactive Cursor Tracking
  if (state.scopeMouseX !== null && state.scopeMouseX >= padding.left && state.scopeMouseX <= padding.left + plotW) {
    const mx = state.scopeMouseX;
    const xRatio = (mx - padding.left) / plotW;
    const cursorXVal = isLogX
      ? Math.pow(10, logMinX + xRatio * (logMaxX - logMinX))
      : minX + xRatio * (maxX - minX);

    // Vertical cursor line
    ctxPlot.save();
    ctxPlot.strokeStyle = '#CF222E';
    ctxPlot.lineWidth = 1;
    ctxPlot.setLineDash([4, 3]);
    ctxPlot.beginPath();
    ctxPlot.moveTo(mx, padding.top);
    ctxPlot.lineTo(mx, padding.top + plotH);
    ctxPlot.stroke();
    ctxPlot.restore();

    // Find nearest point
    let nearestIdx = 0;
    let minDiff = Infinity;
    for (let i = 0; i < rawXValues.length; i++) {
      const diff = Math.abs(rawXValues[i] - cursorXVal);
      if (diff < minDiff) {
        minDiff = diff;
        nearestIdx = i;
      }
    }

    // Draw point markers and compute readout text
    let readoutParts = [`${xKey}=${formatEng(rawXValues[nearestIdx], xKey === 'time' ? 's' : xKey === 'frequency' ? 'Hz' : '')}`];

    activeYKeys.forEach((k) => {
      const originalIndex = allYKeys.indexOf(k);
      const color = traceColors[originalIndex % traceColors.length];
      const yVal = plot.realData[k][nearestIdx];
      const yNorm = (yVal - minY) / (maxY - minY);
      const py = padding.top + (1 - yNorm) * plotH;

      ctxPlot.beginPath();
      ctxPlot.fillStyle = color;
      ctxPlot.arc(mx, py, 4, 0, Math.PI * 2);
      ctxPlot.fill();
      ctxPlot.strokeStyle = '#FFFFFF';
      ctxPlot.lineWidth = 1.5;
      ctxPlot.stroke();

      readoutParts.push(`${k}: ${formatEng(yVal, 'V')}`);
    });

    const readoutElement = document.getElementById('scope-cursor-readout');
    if (readoutElement) {
      readoutElement.textContent = `Cursor [${readoutParts.join(' | ')}]`;
    }
  }
}

// -------------------------------------------------------------
// Component Selection, Property Sync, & Operations
// -------------------------------------------------------------
function syncSelectedComponentToInspector() {
  if (!state.selectedComponentId) {
    propDesignator.value = '';
    propValue.value = '';
    propRotation.value = '';
    return;
  }

  const comp = state.project.circuit.components.find((c) => c.id === state.selectedComponentId);
  if (!comp) return;

  propDesignator.value = comp.designator;
  propValue.value = comp.value;
  propRotation.value = `${comp.rotation}°`;
}

function rotateSelectedComponent() {
  if (!state.selectedComponentId) return;
  const comp = state.project.circuit.components.find((c) => c.id === state.selectedComponentId);
  if (!comp) return;

  state.recordSnapshot();
  comp.rotation = (comp.rotation + 90) % 360;
  syncSelectedComponentToInspector();
  renderSchematic();
}

function deleteSelection() {
  if (state.selectedComponentId) {
    state.recordSnapshot();
    state.project.circuit.components = state.project.circuit.components.filter(
      (c) => c.id !== state.selectedComponentId
    );
    state.selectedComponentId = null;
    syncSelectedComponentToInspector();
    renderSchematic();
  } else if (state.selectedWireId) {
    state.recordSnapshot();
    state.project.circuit.wires = state.project.circuit.wires.filter(
      (w) => w.id !== state.selectedWireId
    );
    state.selectedWireId = null;
    renderSchematic();
  }
}

// -------------------------------------------------------------
// Benchmark Presets
// -------------------------------------------------------------
function loadBenchmark(name: string) {
  state.recordSnapshot();
  state.project.circuit.components = [];
  state.project.circuit.wires = [];
  state.project.circuit.junctions = [];
  state.project.circuit.netLabels = [];

  switch (name) {
    case 'ex-ohm': {
      state.project.name = "Benchmark 1: Ohm's Law";
      state.project.simulation.analysisType = 'operatingPoint';
      state.project.circuit.components = [
        {
          id: 'v1',
          designator: 'V1',
          componentType: 'vdc',
          value: '10V',
          x: 200,
          y: 200,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'r1',
          designator: 'R1',
          componentType: 'resistor',
          value: '1k',
          x: 320,
          y: 160,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'gnd',
          designator: '0',
          componentType: 'ground',
          value: '0',
          x: 200,
          y: 260,
          rotation: 0,
          mirror: false,
          properties: {},
        },
      ];
      state.project.circuit.wires = [
        {
          id: 'w1',
          points: [
            { x: 200, y: 160 },
            { x: 280, y: 160 },
          ],
        },
        {
          id: 'w2',
          points: [
            { x: 360, y: 160 },
            { x: 400, y: 160 },
            { x: 400, y: 240 },
            { x: 200, y: 240 },
          ],
        },
      ];
      state.project.circuit.junctions = [{ x: 200, y: 240 }];
      break;
    }
    case 'ex-divider': {
      state.project.name = 'Benchmark 2: Voltage Divider';
      state.project.simulation.analysisType = 'operatingPoint';
      state.project.circuit.components = [
        {
          id: 'v1',
          designator: 'V1',
          componentType: 'vdc',
          value: '12V',
          x: 160,
          y: 200,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'r1',
          designator: 'R1',
          componentType: 'resistor',
          value: '3k',
          x: 260,
          y: 160,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'r2',
          designator: 'R2',
          componentType: 'resistor',
          value: '1k',
          x: 360,
          y: 200,
          rotation: 90,
          mirror: false,
          properties: {},
        },
        {
          id: 'gnd',
          designator: '0',
          componentType: 'ground',
          value: '0',
          x: 160,
          y: 260,
          rotation: 0,
          mirror: false,
          properties: {},
        },
      ];
      state.project.circuit.wires = [
        {
          id: 'w1',
          points: [
            { x: 160, y: 160 },
            { x: 220, y: 160 },
          ],
        },
        {
          id: 'w2',
          points: [
            { x: 300, y: 160 },
            { x: 360, y: 160 },
          ],
        },
        {
          id: 'w3',
          points: [
            { x: 360, y: 240 },
            { x: 160, y: 240 },
          ],
        },
      ];
      state.project.circuit.junctions = [{ x: 160, y: 240 }];
      break;
    }
    case 'ex-rc-step': {
      state.project.name = 'Benchmark 3: RC Step Transient';
      state.project.simulation.analysisType = 'transient';
      state.project.simulation.transient = {
        step: '10u',
        stop: '5m',
        start: '0',
        maxStep: '10u',
      };
      state.project.circuit.components = [
        {
          id: 'v1',
          designator: 'V1',
          componentType: 'vpulse',
          value: 'PULSE(0 10 0 1u 1u 10m 20m)',
          x: 160,
          y: 200,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'r1',
          designator: 'R1',
          componentType: 'resistor',
          value: '1k',
          x: 260,
          y: 160,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'c1',
          designator: 'C1',
          componentType: 'capacitor',
          value: '1u',
          x: 360,
          y: 200,
          rotation: 90,
          mirror: false,
          properties: {},
        },
        {
          id: 'gnd',
          designator: '0',
          componentType: 'ground',
          value: '0',
          x: 160,
          y: 260,
          rotation: 0,
          mirror: false,
          properties: {},
        },
      ];
      state.project.circuit.wires = [
        {
          id: 'w1',
          points: [
            { x: 160, y: 160 },
            { x: 220, y: 160 },
          ],
        },
        {
          id: 'w2',
          points: [
            { x: 300, y: 160 },
            { x: 360, y: 160 },
          ],
        },
        {
          id: 'w3',
          points: [
            { x: 360, y: 240 },
            { x: 160, y: 240 },
          ],
        },
      ];
      state.project.circuit.junctions = [{ x: 160, y: 240 }];
      break;
    }
    case 'ex-rc-ac': {
      state.project.name = 'Benchmark 4: RC Low-Pass AC';
      state.project.simulation.analysisType = 'ac';
      state.project.simulation.ac = {
        sweepType: 'decade',
        points: 50,
        startFreq: '10',
        stopFreq: '1Meg',
      };
      state.project.circuit.components = [
        {
          id: 'v1',
          designator: 'V1',
          componentType: 'vac',
          value: 'AC 1',
          x: 160,
          y: 200,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'r1',
          designator: 'R1',
          componentType: 'resistor',
          value: '1k',
          x: 260,
          y: 160,
          rotation: 0,
          mirror: false,
          properties: {},
        },
        {
          id: 'c1',
          designator: 'C1',
          componentType: 'capacitor',
          value: '159n',
          x: 360,
          y: 200,
          rotation: 90,
          mirror: false,
          properties: {},
        },
        {
          id: 'gnd',
          designator: '0',
          componentType: 'ground',
          value: '0',
          x: 160,
          y: 260,
          rotation: 0,
          mirror: false,
          properties: {},
        },
      ];
      state.project.circuit.wires = [
        {
          id: 'w1',
          points: [
            { x: 160, y: 160 },
            { x: 220, y: 160 },
          ],
        },
        {
          id: 'w2',
          points: [
            { x: 300, y: 160 },
            { x: 360, y: 160 },
          ],
        },
        {
          id: 'w3',
          points: [
            { x: 360, y: 240 },
            { x: 160, y: 240 },
          ],
        },
      ];
      state.project.circuit.junctions = [{ x: 160, y: 240 }];
      break;
    }
  }

  // Sync simulation config to UI
  simTypeSelect.value = state.project.simulation.analysisType;
  updateSimOptionsVisibility();
  renderSchematic();
  statusMessage.textContent = `Loaded ${state.project.name}`;
}

function updateSimOptionsVisibility() {
  const type = simTypeSelect.value;
  simTranOptions.style.display = type === 'transient' ? 'block' : 'none';
  simAcOptions.style.display = type === 'ac' ? 'block' : 'none';
}

// -------------------------------------------------------------
// Simulation & Validation Workflow
// -------------------------------------------------------------
async function runSimulation() {
  statusMessage.textContent = 'Running SPICE simulation...';
  try {
    // Collect Sim Config
    const type = simTypeSelect.value as 'operatingPoint' | 'transient' | 'ac';
    state.project.simulation.analysisType = type;

    if (type === 'transient') {
      state.project.simulation.transient = {
        stop: simTranStop.value.trim() || '5m',
        step: simTranStep.value.trim() || '10u',
        start: '0',
        maxStep: simTranStep.value.trim() || '10u',
      };
    } else if (type === 'ac') {
      state.project.simulation.ac = {
        sweepType: 'decade',
        points: parseInt(simAcPoints.value) || 50,
        startFreq: simAcStart.value.trim() || '10',
        stopFreq: simAcStop.value.trim() || '1Meg',
      };
    }

    // Generate Netlist Preview
    const netlist = await api.generateNetlist(state.project);
    netlistViewer.textContent = netlist;

    // Run Engine
    const result = await api.runSimulation(state.project);
    state.simResult = result;

    if (result.success) {
      statusMessage.textContent = 'Simulation completed successfully.';
      consoleViewer.textContent = `[SPICE RUN OK]\n${result.stdout}`;

      // Populate DC Operating Point Table
      opTableBody.innerHTML = '';
      const opKeys = Object.keys(result.opTable);
      if (opKeys.length > 0) {
        for (const k of opKeys) {
          const row = document.createElement('tr');
          row.style.borderBottom = '1px solid var(--border-subtle)';
          row.innerHTML = `<td style="padding: 6px;">${k}</td><td style="padding: 6px; font-weight: 600;">${result.opTable[k].toPrecision(6)}</td>`;
          opTableBody.appendChild(row);
        }
      } else {
        opTableBody.innerHTML = '<tr><td colspan="2" style="padding: 10px; color: var(--text-muted);">No DC node voltages reported.</td></tr>';
      }

      renderWaveforms();
    } else {
      statusMessage.textContent = 'Simulation failed.';
      consoleViewer.textContent = `[SPICE ERROR]\n${result.errorMessage || ''}\n\nSTDOUT:\n${result.stdout}\n\nSTDERR:\n${result.stderr}`;
    }
  } catch (err: any) {
    statusMessage.textContent = `Error: ${err}`;
    consoleViewer.textContent = `[SYSTEM ERROR]: ${err}`;
  }
}

async function validateCircuit() {
  try {
    const errs = await api.validateCircuit(state.project.circuit);
    if (errs.length === 0) {
      statusMessage.textContent = 'Circuit validation passed: No errors detected.';
      consoleViewer.textContent = '[VALIDATION SUCCESS]: Ground verified, designators unique, all values populated.';
    } else {
      statusMessage.textContent = `Validation failed: ${errs.length} issues found.`;
      consoleViewer.textContent = `[VALIDATION ISSUES]:\n` + errs.map((e) => `- [${e.code}] ${e.message}`).join('\n');
    }
  } catch (err: any) {
    statusMessage.textContent = `Validation error: ${err}`;
  }
}

// -------------------------------------------------------------
// Canvas Event Handlers (Pan, Zoom, Place, Select, Wire)
// -------------------------------------------------------------
let isPanning = false;
let panStartX = 0;
let panStartY = 0;

let isDraggingComponent = false;
let dragStartX = 0;
let dragStartY = 0;

schematicCanvas.addEventListener('mousedown', (e) => {
  const rect = schematicCanvas.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;

  // Middle click or Alt-drag = Pan
  if (e.button === 1 || (e.button === 0 && e.altKey)) {
    isPanning = true;
    panStartX = mouseX - state.panX;
    panStartY = mouseY - state.panY;
    return;
  }

  const worldPos = screenToWorld(mouseX, mouseY);

  // Tool: Place component
  if (state.activeTool === 'place' && state.pendingPlacementType) {
    state.recordSnapshot();
    const type = state.pendingPlacementType;
    const count = state.project.circuit.components.filter((c) => c.componentType === type).length + 1;
    const prefix =
      type === 'resistor' ? 'R' :
      type === 'potentiometer' ? 'POT' :
      type === 'capacitor' ? 'C' :
      type === 'inductor' ? 'L' :
      type === 'switch' ? 'SW' :
      type === 'transformer' ? 'T' :
      type === 'vreg' ? 'U_REG' :
      type === 'diode' ? 'D' :
      type === 'zenerDiode' ? 'DZ' :
      type === 'led' ? 'LED' :
      type === 'ground' ? '0' :
      type === 'bjtNpn' ? 'Q' :
      type === 'bjtPnp' ? 'Q' :
      type === 'nmos' ? 'M' :
      type === 'pmos' ? 'M' :
      type === 'opAmp' ? 'U' :
      type === 'probeVoltagePos' ? 'VP+' :
      type === 'probeVoltageNeg' ? 'VP-' :
      type === 'probeVoltage' ? 'VP' :
      type === 'probeCurrent' ? 'IPROBE' :
      type === 'idc' ? 'I' :
      type === 'iac' ? 'I_AC' : 'V';

    const defaultVal =
      type === 'resistor' ? '1k' :
      type === 'potentiometer' ? '10k' :
      type === 'capacitor' ? '1u' :
      type === 'inductor' ? '10m' :
      type === 'switch' ? '0.001' :
      type === 'transformer' ? '10m' :
      type === 'vreg' ? '5V' :
      type === 'diode' ? '1N4148' :
      type === 'zenerDiode' ? '5.1V' :
      type === 'led' ? 'RED' :
      type === 'vdc' ? '5V' :
      type === 'vac' ? 'AC 1' :
      type === 'vsin' ? 'SIN(0 5 1k)' :
      type === 'vpulse' ? 'PULSE(0 5 0 1u 1u 5m 10m)' :
      type === 'idc' ? '1m' :
      type === 'iac' ? '1m' :
      type === 'probeVoltagePos' ? 'V+' :
      type === 'probeVoltageNeg' ? 'V-' :
      type === 'probeVoltage' ? 'V' :
      type === 'probeCurrent' ? 'I' :
      type === 'ground' ? '0' : '1';

    const newComp: Component = {
      id: `comp_${Date.now()}`,
      designator: type === 'ground' ? '0' : `${prefix}${count}`,
      componentType: type,
      value: defaultVal,
      x: worldPos.x,
      y: worldPos.y,
      rotation: 0,
      mirror: false,
      properties: {},
    };

    state.project.circuit.components.push(newComp);
    state.selectedComponentId = newComp.id;
    state.activeTool = 'select';
    state.pendingPlacementType = null;
    document.querySelectorAll('.palette-item').forEach((el) => el.classList.remove('active'));
    syncSelectedComponentToInspector();
    renderSchematic();
    return;
  }

  // Tool: Wire
  if (state.activeTool === 'wire') {
    if (!state.wireStartPoint) {
      state.wireStartPoint = worldPos;
      state.wireCurrentPoint = worldPos;
    } else {
      state.recordSnapshot();
      // Commit orthogonal wire segment
      const p1 = state.wireStartPoint;
      const p2 = { x: worldPos.x, y: p1.y };
      const p3 = worldPos;

      const newWire: Wire = {
        id: `wire_${Date.now()}`,
        points: [p1, p2, p3],
      };
      state.project.circuit.wires.push(newWire);
      state.wireStartPoint = null;
      state.wireCurrentPoint = null;
      renderSchematic();
    }
    return;
  }

  // Tool: Select / Move
  if (state.activeTool === 'select') {
    // Check hit on components
    let hitComp: Component | null = null;
    for (const c of state.project.circuit.components) {
      const dist = Math.hypot(c.x - worldPos.x, c.y - worldPos.y);
      if (dist <= 30) {
        hitComp = c;
        break;
      }
    }

    if (hitComp) {
      state.selectedComponentId = hitComp.id;
      state.selectedWireId = null;
      isDraggingComponent = true;
      dragStartX = worldPos.x - hitComp.x;
      dragStartY = worldPos.y - hitComp.y;
      syncSelectedComponentToInspector();
      renderSchematic();
      return;
    }

    // Check hit on wires
    let hitWire: Wire | null = null;
    for (const w of state.project.circuit.wires) {
      for (let i = 0; i < w.points.length - 1; i++) {
        const pa = w.points[i];
        const pb = w.points[i + 1];
        // Line distance check
        const lineDist = pointToSegmentDistance(worldPos, pa, pb);
        if (lineDist < 10) {
          hitWire = w;
          break;
        }
      }
      if (hitWire) break;
    }

    if (hitWire) {
      state.selectedWireId = hitWire.id;
      state.selectedComponentId = null;
      syncSelectedComponentToInspector();
      renderSchematic();
      return;
    }

    // Clear selection
    state.selectedComponentId = null;
    state.selectedWireId = null;
    syncSelectedComponentToInspector();
    renderSchematic();
  }
});

function pointToSegmentDistance(p: Point, a: Point, b: Point): number {
  const l2 = (b.x - a.x) ** 2 + (b.y - a.y) ** 2;
  if (l2 === 0) return Math.hypot(p.x - a.x, p.y - a.y);
  let t = ((p.x - a.x) * (b.x - a.x) + (p.y - a.y) * (b.y - a.y)) / l2;
  t = Math.max(0, Math.min(1, t));
  return Math.hypot(p.x - (a.x + t * (b.x - a.x)), p.y - (a.y + t * (b.y - a.y)));
}

schematicCanvas.addEventListener('mousemove', (e) => {
  const rect = schematicCanvas.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;

  if (isPanning) {
    state.panX = mouseX - panStartX;
    state.panY = mouseY - panStartY;
    renderSchematic();
    return;
  }

  const worldPos = screenToWorld(mouseX, mouseY);

  if (isDraggingComponent && state.selectedComponentId) {
    const comp = state.project.circuit.components.find((c) => c.id === state.selectedComponentId);
    if (comp) {
      comp.x = snapToGrid(worldPos.x - dragStartX, state.gridPitch);
      comp.y = snapToGrid(worldPos.y - dragStartY, state.gridPitch);
      renderSchematic();
    }
    return;
  }

  if (state.activeTool === 'wire' && state.wireStartPoint) {
    state.wireCurrentPoint = worldPos;
    renderSchematic();
  }
});

schematicCanvas.addEventListener('mouseup', () => {
  if (isDraggingComponent) {
    isDraggingComponent = false;
    state.recordSnapshot();
  }
  isPanning = false;
});

schematicCanvas.addEventListener('wheel', (e) => {
  e.preventDefault();
  const zoomFactor = e.deltaY < 0 ? 1.1 : 0.9;
  const newZoom = Math.max(0.4, Math.min(3.0, state.zoom * zoomFactor));

  const rect = schematicCanvas.getBoundingClientRect();
  const mouseX = e.clientX - rect.left;
  const mouseY = e.clientY - rect.top;

  state.panX = mouseX - (mouseX - state.panX) * (newZoom / state.zoom);
  state.panY = mouseY - (mouseY - state.panY) * (newZoom / state.zoom);
  state.zoom = newZoom;

  statusGrid.textContent = `Grid: 20px | Zoom: ${Math.round(state.zoom * 100)}%`;
  renderSchematic();
});

// -------------------------------------------------------------
// UI Bindings & Keyboard Shortcuts
// -------------------------------------------------------------
document.getElementById('btn-run')?.addEventListener('click', runSimulation);
document.getElementById('btn-validate')?.addEventListener('click', validateCircuit);
document.getElementById('btn-rotate')?.addEventListener('click', rotateSelectedComponent);
document.getElementById('btn-delete')?.addEventListener('click', deleteSelection);

document.getElementById('btn-wire')?.addEventListener('click', () => {
  state.activeTool = state.activeTool === 'wire' ? 'select' : 'wire';
  state.wireStartPoint = null;
  state.wireCurrentPoint = null;
  statusMessage.textContent = state.activeTool === 'wire' ? 'Wire Tool Active: Click pin to start wire' : 'Select Tool Active';
});

// Component palette click handler
document.querySelectorAll('.palette-item').forEach((item) => {
  item.addEventListener('click', () => {
    document.querySelectorAll('.palette-item').forEach((el) => el.classList.remove('active'));
    item.classList.add('active');
    state.activeTool = 'place';
    state.pendingPlacementType = item.getAttribute('data-type') as ComponentType;
    statusMessage.textContent = `Placement active: Click canvas to position ${state.pendingPlacementType}`;
  });
});

// Inspector Property Editing Sync
propDesignator.addEventListener('change', () => {
  if (!state.selectedComponentId) return;
  const comp = state.project.circuit.components.find((c) => c.id === state.selectedComponentId);
  if (comp) {
    state.recordSnapshot();
    comp.designator = propDesignator.value.trim();
    renderSchematic();
  }
});

propValue.addEventListener('change', () => {
  if (!state.selectedComponentId) return;
  const comp = state.project.circuit.components.find((c) => c.id === state.selectedComponentId);
  if (comp) {
    state.recordSnapshot();
    comp.value = propValue.value.trim();
    renderSchematic();
  }
});

// Analysis Deck Tabs
document.querySelectorAll('.deck-tab').forEach((tab) => {
  tab.addEventListener('click', () => {
    document.querySelectorAll('.deck-tab').forEach((t) => t.classList.remove('active'));
    tab.classList.add('active');
    const tabTarget = tab.getAttribute('data-tab');

    document.getElementById('tab-waveforms')!.style.display = tabTarget === 'waveforms' ? 'block' : 'none';
    document.getElementById('tab-op')!.style.display = tabTarget === 'op' ? 'block' : 'none';
    document.getElementById('tab-netlist')!.style.display = tabTarget === 'netlist' ? 'block' : 'none';
    document.getElementById('tab-console')!.style.display = tabTarget === 'console' ? 'block' : 'none';

    if (tabTarget === 'waveforms') {
      renderWaveforms();
    }
  });
});

// Oscilloscope Canvas Mouse Interactions
plotCanvas.addEventListener('mousemove', (e) => {
  const rect = plotCanvas.getBoundingClientRect();
  state.scopeMouseX = e.clientX - rect.left;
  state.scopeMouseY = e.clientY - rect.top;
  renderWaveforms();
});

plotCanvas.addEventListener('mouseleave', () => {
  state.scopeMouseX = null;
  state.scopeMouseY = null;
  const readout = document.getElementById('scope-cursor-readout');
  if (readout) readout.textContent = 'Cursor: Move mouse over plot';
  renderWaveforms();
});

document.getElementById('btn-scope-autoscale')?.addEventListener('click', () => {
  state.scopeZoomX = 1.0;
  state.scopePanX = 0;
  renderWaveforms();
});

document.getElementById('btn-scope-mode')?.addEventListener('click', () => {
  state.scopeGridMode = state.scopeGridMode === 'linear' ? 'log' : 'linear';
  const btn = document.getElementById('btn-scope-mode');
  if (btn) btn.textContent = `Grid: ${state.scopeGridMode === 'linear' ? 'Linear' : 'Logarithmic'}`;
  renderWaveforms();
});

// Simulation Type Select
simTypeSelect.addEventListener('change', updateSimOptionsVisibility);

// Example Circuit Select
const exampleSelect = document.getElementById('select-example') as HTMLSelectElement;
exampleSelect.addEventListener('change', () => {
  if (exampleSelect.value) {
    loadBenchmark(exampleSelect.value);
  }
});

// Save Project Action
document.getElementById('btn-save')?.addEventListener('click', async () => {
  try {
    const selectedPath = await save({
      filters: [{ name: 'CircuitForge Project', extensions: ['forge', 'json'] }],
      defaultPath: `${state.project.name.replace(/\s+/g, '_').toLowerCase()}.forge`,
    });
    if (selectedPath) {
      await api.saveProject(selectedPath, state.project);
      statusMessage.textContent = `Project saved: ${selectedPath}`;
    }
  } catch (err: any) {
    statusMessage.textContent = `Save error: ${err}`;
  }
});

// Open Project Action
document.getElementById('btn-open')?.addEventListener('click', async () => {
  try {
    const selectedPath = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'CircuitForge Project', extensions: ['forge', 'json'] }],
    });
    if (selectedPath && typeof selectedPath === 'string') {
      const proj = await api.openProject(selectedPath);
      state.project = proj;
      simTypeSelect.value = proj.simulation.analysisType;
      updateSimOptionsVisibility();
      renderSchematic();
      statusMessage.textContent = `Opened: ${proj.name}`;
    }
  } catch (err: any) {
    statusMessage.textContent = `Open error: ${err}`;
  }
});

// New Project
document.getElementById('btn-new')?.addEventListener('click', () => {
  state.recordSnapshot();
  state.project.name = 'Untitled Circuit';
  state.project.circuit = { components: [], wires: [], junctions: [], netLabels: [] };
  state.selectedComponentId = null;
  state.selectedWireId = null;
  syncSelectedComponentToInspector();
  renderSchematic();
  statusMessage.textContent = 'New blank circuit created.';
});

// CSV Export
document.getElementById('btn-export-csv')?.addEventListener('click', async () => {
  if (!state.simResult || !state.simResult.plots || state.simResult.plots.length === 0) {
    statusMessage.textContent = 'No simulation results to export.';
    return;
  }
  const plot = state.simResult.plots[0];
  const keys = Object.keys(plot.realData);
  if (keys.length === 0) return;

  let csvContent = keys.join(',') + '\n';
  const rowCount = plot.realData[keys[0]].length;
  for (let i = 0; i < rowCount; i++) {
    const row = keys.map((k) => plot.realData[k][i]);
    csvContent += row.join(',') + '\n';
  }

  try {
    const savePath = await save({
      filters: [{ name: 'CSV File', extensions: ['csv'] }],
      defaultPath: 'waveforms.csv',
    });
    if (savePath) {
      const blob = new Blob([csvContent], { type: 'text/csv' });
      const a = document.createElement('a');
      a.href = URL.createObjectURL(blob);
      a.download = 'waveforms.csv';
      a.click();
      statusMessage.textContent = 'Waveform CSV exported.';
    }
  } catch (err: any) {
    statusMessage.textContent = `Export error: ${err}`;
  }
});

// Keyboard shortcuts
window.addEventListener('keydown', (e) => {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;

  if (e.key === 'F5') {
    e.preventDefault();
    runSimulation();
  } else if (e.key === 'r' || e.key === 'R') {
    rotateSelectedComponent();
  } else if (e.key === 'Delete' || e.key === 'Backspace') {
    deleteSelection();
  } else if (e.key === 'w' || e.key === 'W') {
    document.getElementById('btn-wire')?.click();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
    e.preventDefault();
    if (e.shiftKey) state.redo();
    else state.undo();
    renderSchematic();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
    e.preventDefault();
    state.redo();
    renderSchematic();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
    e.preventDefault();
    document.getElementById('btn-save')?.click();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'o') {
    e.preventDefault();
    document.getElementById('btn-open')?.click();
  } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'n') {
    e.preventDefault();
    document.getElementById('btn-new')?.click();
  }
});

// Initial boot
async function init() {
  resizeCanvases();
  try {
    const engineReady = await api.getEngineStatus();
    if (engineReady) {
      statusEngineDot.className = 'status-dot';
      statusMessage.textContent = 'Engine verified (ngspice-38). Ready.';
    } else {
      statusEngineDot.className = 'status-dot error';
      statusMessage.textContent = 'Engine offline: ngspice binary not detected.';
    }
  } catch {
    statusEngineDot.className = 'status-dot warning';
  }

  // Load Benchmark 1 by default
  loadBenchmark('ex-ohm');
}

init();
