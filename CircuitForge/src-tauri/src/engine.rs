use crate::model::*;
use crate::netlist::NetlistGenerator;
use crate::rawfile::{RawVariable, RawfileParser, SimulationResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub struct SimulationEngine;

impl SimulationEngine {
    pub fn locate_ngspice() -> Option<PathBuf> {
        // 1. Check bundled resource directory relative to current exe
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(parent) = exe_path.parent()
        {
            // Production bundle resource path
            let bundled_1 = parent
                .join("resources")
                .join("ngspice")
                .join("ngspice_con.exe");
            if bundled_1.exists() {
                return Some(bundled_1);
            }
            let bundled_2 = parent.join("ngspice_con.exe");
            if bundled_2.exists() {
                return Some(bundled_2);
            }
        }

        // 2. Check development resource location
        let dev_path = PathBuf::from("resources/ngspice/ngspice_con.exe");
        if dev_path.exists() {
            return Some(dev_path.canonicalize().unwrap_or(dev_path));
        }

        let dev_path2 = PathBuf::from("src-tauri/resources/ngspice/ngspice_con.exe");
        if dev_path2.exists() {
            return Some(dev_path2.canonicalize().unwrap_or(dev_path2));
        }

        // 3. Fallback to system temp extraction or known location
        let temp_path = PathBuf::from(
            r"C:\Users\Tonmoy\AppData\Local\Temp\ngspice_extracted\Spice64\bin\ngspice_con.exe",
        );
        if temp_path.exists() {
            return Some(temp_path);
        }

        None
    }

    pub fn run_simulation(_project: &Project, netlist_str: &str) -> SimulationResult {
        let engine_path = match Self::locate_ngspice() {
            Some(p) => p,
            None => {
                return SimulationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    plots: Vec::new(),
                    op_table: HashMap::new(),
                    error_message: Some(
                        "ngspice simulation engine was not found on system or bundled resources."
                            .to_string(),
                    ),
                };
            }
        };

        // Create isolated temp directory
        let temp_dir = match tempfile::Builder::new()
            .prefix("circuitforge_sim_")
            .tempdir()
        {
            Ok(d) => d,
            Err(e) => {
                return SimulationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    plots: Vec::new(),
                    op_table: HashMap::new(),
                    error_message: Some(format!(
                        "Failed to create temporary simulation directory: {}",
                        e
                    )),
                };
            }
        };

        let cir_file = temp_dir.path().join("circuit.cir");
        let raw_file = temp_dir.path().join("output.raw");
        let log_file = temp_dir.path().join("output.log");

        // Write netlist with control block to generate ascii rawfile
        let mut full_netlist = netlist_str.to_string();
        // Insert control block before .END
        let control_block = format!(
            "\n.control\nset filetype=ascii\nrun\nwrite \"{}\"\n.endc\n",
            raw_file.to_str().unwrap_or("output.raw").replace('\\', "/")
        );

        if let Some(pos) = full_netlist.to_uppercase().rfind(".END") {
            full_netlist.insert_str(pos, &control_block);
        } else {
            full_netlist.push_str(&control_block);
            full_netlist.push_str("\n.END\n");
        }

        if let Err(e) = std::fs::write(&cir_file, &full_netlist) {
            return SimulationResult {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                plots: Vec::new(),
                op_table: HashMap::new(),
                error_message: Some(format!("Failed to write netlist file: {}", e)),
            };
        }

        // Execute ngspice_con in batch mode without shell
        let mut cmd = Command::new(&engine_path);
        cmd.arg("-b")
            .arg("-r")
            .arg(&raw_file)
            .arg("-o")
            .arg(&log_file)
            .arg(&cir_file)
            .current_dir(temp_dir.path());

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => {
                return SimulationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    plots: Vec::new(),
                    op_table: HashMap::new(),
                    error_message: Some(format!(
                        "Failed to spawn simulation engine process: {}",
                        e
                    )),
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let log_content = std::fs::read_to_string(&log_file).unwrap_or_default();

        let combined_log = format!("{}\n{}", stdout, log_content);
        let log_lower = combined_log.to_lowercase();

        // Check for fatal errors in SPICE log
        let has_fatal = log_lower.contains("fatal error")
            || log_lower.contains("singular matrix")
            || log_lower.contains("iteration limit reached")
            || log_lower.contains("no such vector")
            || log_lower.contains("simulation interrupted due to error")
            || log_lower.contains("no such parameter on this device");

        let mut plots = if raw_file.exists() {
            let raw_str = std::fs::read_to_string(&raw_file).unwrap_or_default();
            RawfileParser::parse_ascii_rawfile(&raw_str).unwrap_or_default()
        } else {
            Vec::new()
        };

        // Compute differential probe vectors
        let probe_pairs = NetlistGenerator::resolve_probes(&_project.circuit);
        for plot in &mut plots {
            for (probe_label, pos_node, neg_node) in &probe_pairs {
                let pos_key = format!("v({})", pos_node.to_lowercase());
                let neg_key = format!("v({})", neg_node.to_lowercase());

                // Find matching real vectors
                let pos_real = plot
                    .real_data
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == pos_key)
                    .map(|(_, v)| v.clone());

                let neg_real = if neg_node == "0" {
                    pos_real.as_ref().map(|v| vec![0.0; v.len()])
                } else {
                    plot.real_data
                        .iter()
                        .find(|(k, _)| k.to_lowercase() == neg_key)
                        .map(|(_, v)| v.clone())
                };

                if let (Some(pv), Some(nv)) = (pos_real, neg_real) {
                    let len = pv.len().min(nv.len());
                    let mut diff_vals = Vec::with_capacity(len);
                    for i in 0..len {
                        diff_vals.push(pv[i] - nv[i]);
                    }
                    plot.real_data.insert(probe_label.clone(), diff_vals);
                    let insert_idx = 1.min(plot.variables.len());
                    plot.variables.insert(
                        insert_idx,
                        RawVariable {
                            name: probe_label.clone(),
                            var_type: "voltage".to_string(),
                        },
                    );
                }

                // Complex vectors (for AC analysis)
                let pos_cplx = plot
                    .complex_data
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == pos_key)
                    .map(|(_, v)| v.clone());

                let neg_cplx = if neg_node == "0" {
                    pos_cplx.as_ref().map(|v| {
                        vec![
                            crate::rawfile::ComplexNum {
                                real: 0.0,
                                imag: 0.0
                            };
                            v.len()
                        ]
                    })
                } else {
                    plot.complex_data
                        .iter()
                        .find(|(k, _)| k.to_lowercase() == neg_key)
                        .map(|(_, v)| v.clone())
                };

                if let (Some(pc), Some(nc)) = (pos_cplx, neg_cplx) {
                    let len = pc.len().min(nc.len());
                    let mut diff_cplx = Vec::with_capacity(len);
                    let mut diff_mag = Vec::with_capacity(len);
                    for i in 0..len {
                        let dr = pc[i].real - nc[i].real;
                        let di = pc[i].imag - nc[i].imag;
                        diff_cplx.push(crate::rawfile::ComplexNum { real: dr, imag: di });
                        diff_mag.push((dr * dr + di * di).sqrt());
                    }
                    plot.complex_data.insert(probe_label.clone(), diff_cplx);
                    plot.real_data.insert(probe_label.clone(), diff_mag);
                }
            }
        }

        // Extract Operating Point table if available
        let mut op_table = HashMap::new();
        if let Some(op_plot) = plots
            .iter()
            .find(|p| p.plot_name.to_lowercase().contains("op"))
        {
            for v in &op_plot.variables {
                if let Some(vals) = op_plot.real_data.get(&v.name)
                    && let Some(&first) = vals.first()
                {
                    op_table.insert(v.name.clone(), first);
                }
            }
        }

        let is_success =
            output.status.success() && !has_fatal && (!plots.is_empty() || !op_table.is_empty());
        let error_msg = if !is_success {
            if has_fatal {
                let err_line = combined_log
                    .lines()
                    .find(|l| {
                        let ll = l.to_lowercase();
                        ll.contains("error on line")
                            || ll.contains("fatal error")
                            || ll.contains("singular matrix")
                            || ll.contains("simulation interrupted")
                            || ll.contains("no such parameter")
                    })
                    .unwrap_or("Simulation failed: convergence or matrix singularity error detected.");
                Some(err_line.trim().to_string())
            } else if plots.is_empty() {
                Some("Simulation completed but produced no output vectors.".to_string())
            } else {
                Some("Simulation engine exited with error.".to_string())
            }
        } else {
            None
        };

        SimulationResult {
            success: is_success,
            stdout: combined_log,
            stderr,
            exit_code: output.status.code(),
            plots,
            op_table,
            error_message: error_msg,
        }
    }
}
