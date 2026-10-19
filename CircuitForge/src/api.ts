import { invoke } from '@tauri-apps/api/core';
import type { Circuit, Project, SimulationResult, ValidationError } from './types';

export const api = {
  validateCircuit: async (circuit: Circuit): Promise<ValidationError[]> => {
    return invoke('validate_circuit', { circuit });
  },

  generateNetlist: async (project: Project): Promise<string> => {
    return invoke('generate_netlist', { project });
  },

  runSimulation: async (project: Project): Promise<SimulationResult> => {
    return invoke('run_simulation', { project });
  },

  saveProject: async (path: string, project: Project): Promise<boolean> => {
    return invoke('save_project', { path, project });
  },

  openProject: async (path: string): Promise<Project> => {
    return invoke('open_project', { path });
  },

  getEngineStatus: async (): Promise<boolean> => {
    return invoke('get_engine_status');
  },
};
