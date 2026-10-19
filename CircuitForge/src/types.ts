export type ComponentType =
  | 'resistor'
  | 'capacitor'
  | 'inductor'
  | 'diode'
  | 'bjtNpn'
  | 'bjtPnp'
  | 'nmos'
  | 'pmos'
  | 'opAmp'
  | 'potentiometer'
  | 'zenerDiode'
  | 'led'
  | 'probeVoltage'
  | 'probeVoltagePos'
  | 'probeVoltageNeg'
  | 'probeCurrent'
  | 'vdc'
  | 'vac'
  | 'vsin'
  | 'vpulse'
  | 'idc'
  | 'iac'
  | 'switch'
  | 'transformer'
  | 'vreg'
  | 'ground';

export interface Point {
  x: number;
  y: number;
}

export interface ComponentPin {
  id: string;
  name: string;
  offsetX: number;
  offsetY: number;
}

export interface Component {
  id: string;
  designator: string;
  componentType: ComponentType;
  value: string;
  x: number;
  y: number;
  rotation: number;
  mirror: boolean;
  properties: Record<string, string>;
}

export interface Wire {
  id: string;
  points: Point[];
}

export interface NetLabel {
  name: string;
  x: number;
  y: number;
}

export interface Circuit {
  components: Component[];
  wires: Wire[];
  junctions: Point[];
  netLabels: NetLabel[];
}

export type AnalysisType = 'operatingPoint' | 'transient' | 'ac';

export type AcSweepType = 'decade' | 'octave' | 'linear';

export interface TransientAnalysis {
  step: string;
  stop: string;
  start?: string;
  maxStep?: string;
}

export interface AcAnalysis {
  sweepType: AcSweepType;
  points: number;
  startFreq: string;
  stopFreq: string;
}

export interface SimulationConfig {
  analysisType: AnalysisType;
  transient?: TransientAnalysis;
  ac?: AcAnalysis;
  dcOp: boolean;
}

export interface Project {
  schemaVersion: string;
  name: string;
  description?: string;
  created: string;
  modified: string;
  simulation: SimulationConfig;
  circuit: Circuit;
}

export interface ValidationError {
  code: string;
  message: string;
  componentId?: string;
  isFatal: boolean;
}

export interface ComplexNum {
  real: number;
  imag: number;
}

export interface RawVariable {
  name: string;
  varType: string;
}

export interface SimulationPlot {
  title: string;
  plotName: string;
  variables: RawVariable[];
  realData: Record<string, number[]>;
  complexData: Record<string, ComplexNum[]>;
}

export interface SimulationResult {
  success: boolean;
  stdout: string;
  stderr: string;
  exitCode?: number;
  plots: SimulationPlot[];
  opTable: Record<string, number>;
  errorMessage?: string;
}
