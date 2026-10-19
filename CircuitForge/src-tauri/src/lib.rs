pub mod commands {
    use crate::engine::SimulationEngine;
    use crate::model::{Circuit, Project, ValidationError};
    use crate::netlist::NetlistGenerator;
    use crate::rawfile::SimulationResult;
    use std::path::PathBuf;

    #[tauri::command]
    pub fn validate_circuit(circuit: Circuit) -> Vec<ValidationError> {
        circuit.validate()
    }

    #[tauri::command]
    pub fn generate_netlist(project: Project) -> Result<String, String> {
        NetlistGenerator::build_netlist(&project)
    }

    #[tauri::command]
    pub fn run_simulation(project: Project) -> Result<SimulationResult, String> {
        let errors = project.circuit.validate();
        let fatal_errors: Vec<_> = errors.into_iter().filter(|e| e.is_fatal).collect();
        if !fatal_errors.is_empty() {
            return Ok(SimulationResult {
                success: false,
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                plots: Vec::new(),
                op_table: std::collections::HashMap::new(),
                error_message: Some(fatal_errors[0].message.clone()),
            });
        }

        let netlist = NetlistGenerator::build_netlist(&project)?;
        let result = SimulationEngine::run_simulation(&project, &netlist);
        Ok(result)
    }

    #[tauri::command]
    pub fn save_project(path: String, project: Project) -> Result<bool, String> {
        let p = PathBuf::from(&path);
        let temp_path = p.with_extension(format!("tmp.{}", chrono::Utc::now().timestamp_millis()));

        let json_data = serde_json::to_string_pretty(&project)
            .map_err(|e| format!("Failed to serialize project: {}", e))?;

        std::fs::write(&temp_path, json_data)
            .map_err(|e| format!("Failed to write temporary project file: {}", e))?;

        std::fs::rename(&temp_path, &p)
            .map_err(|e| format!("Failed to save project file atomically: {}", e))?;

        Ok(true)
    }

    #[tauri::command]
    pub fn open_project(path: String) -> Result<Project, String> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read project file: {}", e))?;

        let project: Project =
            serde_json::from_str(&content).map_err(|e| format!("Malformed project file: {}", e))?;

        Ok(project)
    }

    #[tauri::command]
    pub fn get_engine_status() -> Result<bool, String> {
        Ok(SimulationEngine::locate_ngspice().is_some())
    }
}

pub mod engine;
pub mod model;
pub mod netlist;
pub mod rawfile;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::validate_circuit,
            commands::generate_netlist,
            commands::run_simulation,
            commands::save_project,
            commands::open_project,
            commands::get_engine_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running circuitforge application");
}
