use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplexNum {
    pub real: f64,
    pub imag: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawVariable {
    pub name: String,
    pub var_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationPlot {
    pub title: String,
    pub plot_name: String,
    pub variables: Vec<RawVariable>,
    pub real_data: HashMap<String, Vec<f64>>,
    pub complex_data: HashMap<String, Vec<ComplexNum>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub plots: Vec<SimulationPlot>,
    pub op_table: HashMap<String, f64>,
    pub error_message: Option<String>,
}

pub struct RawfileParser;

impl RawfileParser {
    pub fn parse_ascii_rawfile(content: &str) -> Result<Vec<SimulationPlot>, String> {
        let mut plots = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            if line.starts_with("Title:") {
                let title = line.trim_start_matches("Title:").trim().to_string();
                let mut plot_name = String::new();
                let mut num_vars = 0;
                let mut num_pts = 0;
                let mut is_complex = false;
                let mut variables = Vec::new();

                i += 1;
                while i < lines.len() {
                    let h_line = lines[i].trim();
                    if h_line.starts_with("Plotname:") {
                        plot_name = h_line.trim_start_matches("Plotname:").trim().to_string();
                    } else if h_line.starts_with("Flags:") {
                        let flags = h_line.trim_start_matches("Flags:").trim().to_lowercase();
                        if flags.contains("complex") {
                            is_complex = true;
                        }
                    } else if h_line.starts_with("No. Variables:") {
                        num_vars = h_line
                            .trim_start_matches("No. Variables:")
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(0);
                    } else if h_line.starts_with("No. Points:") {
                        num_pts = h_line
                            .trim_start_matches("No. Points:")
                            .trim()
                            .parse::<usize>()
                            .unwrap_or(0);
                    } else if h_line.starts_with("Variables:") {
                        i += 1;
                        for _ in 0..num_vars {
                            if i < lines.len() {
                                let parts: Vec<&str> = lines[i].split_whitespace().collect();
                                if parts.len() >= 3 {
                                    variables.push(RawVariable {
                                        name: parts[1].to_string(),
                                        var_type: parts[2].to_string(),
                                    });
                                }
                                i += 1;
                            }
                        }
                        continue;
                    } else if h_line.starts_with("Values:") {
                        i += 1;
                        break;
                    }
                    i += 1;
                }

                let mut real_data: HashMap<String, Vec<f64>> = HashMap::new();
                let mut complex_data: HashMap<String, Vec<ComplexNum>> = HashMap::new();

                for v in &variables {
                    if is_complex {
                        complex_data.insert(v.name.clone(), Vec::with_capacity(num_pts));
                    }
                    real_data.insert(v.name.clone(), Vec::with_capacity(num_pts));
                }

                // Parse Values
                for _ in 0..num_pts {
                    if i >= lines.len() {
                        break;
                    }
                    // First line of point: index, first variable value
                    let first_line = lines[i].trim();
                    let parts: Vec<&str> = first_line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let var_name = &variables[0].name;
                        if is_complex {
                            let (r, im) = Self::parse_complex(parts[1], parts.get(2).copied());
                            if let Some(v) = complex_data.get_mut(var_name) {
                                v.push(ComplexNum { real: r, imag: im })
                            }
                            if let Some(v) = real_data.get_mut(var_name) {
                                v.push(r)
                            }
                        } else {
                            let val = parts[1].parse::<f64>().unwrap_or(0.0);
                            if let Some(v) = real_data.get_mut(var_name) {
                                v.push(val)
                            }
                        }
                    }
                    i += 1;

                    // Remaining variables for this point
                    for var in variables.iter().take(num_vars).skip(1) {
                        if i >= lines.len() {
                            break;
                        }
                        let sub_parts: Vec<&str> = lines[i].split_whitespace().collect();
                        if !sub_parts.is_empty() {
                            let var_name = &var.name;
                            if is_complex {
                                let (r, im) =
                                    Self::parse_complex(sub_parts[0], sub_parts.get(1).copied());
                                if let Some(v) = complex_data.get_mut(var_name) {
                                    v.push(ComplexNum { real: r, imag: im })
                                }
                                let mag = (r * r + im * im).sqrt();
                                if let Some(v) = real_data.get_mut(var_name) {
                                    v.push(mag)
                                }
                            } else {
                                let val = sub_parts[0].parse::<f64>().unwrap_or(0.0);
                                if let Some(v) = real_data.get_mut(var_name) {
                                    v.push(val)
                                }
                            }
                        }
                        i += 1;
                    }
                }

                plots.push(SimulationPlot {
                    title,
                    plot_name,
                    variables,
                    real_data,
                    complex_data,
                });
            } else {
                i += 1;
            }
        }

        Ok(plots)
    }

    fn parse_complex(p1: &str, p2: Option<&str>) -> (f64, f64) {
        if p1.contains(',') {
            let split: Vec<&str> = p1.split(',').collect();
            let r = split
                .first()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            let im = split
                .get(1)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            (r, im)
        } else if let Some(sec) = p2 {
            let r = p1.parse::<f64>().unwrap_or(0.0);
            let im = sec.parse::<f64>().unwrap_or(0.0);
            (r, im)
        } else {
            let r = p1.parse::<f64>().unwrap_or(0.0);
            (r, 0.0)
        }
    }
}
