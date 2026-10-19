use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ComponentType {
    Resistor,
    Capacitor,
    Inductor,
    Diode,
    BjtNpn,
    BjtPnp,
    Nmos,
    Pmos,
    OpAmp,
    Potentiometer,
    ZenerDiode,
    Led,
    ProbeVoltage,
    ProbeVoltagePos,
    ProbeVoltageNeg,
    ProbeCurrent,
    Vdc,
    Vac,
    Vsin,
    Vpulse,
    Idc,
    Iac,
    Switch,
    Transformer,
    Vreg,
    Ground,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentPin {
    pub id: String,
    pub name: String,
    pub offset_x: f64,
    pub offset_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Component {
    pub id: String,
    pub designator: String,
    pub component_type: ComponentType,
    pub value: String,
    pub x: f64,
    pub y: f64,
    pub rotation: i32, // 0, 90, 180, 270
    pub mirror: bool,
    pub properties: HashMap<String, String>,
}

impl Component {
    pub fn get_pins(&self) -> Vec<ComponentPin> {
        let (_rad, cos_r, sin_r) = {
            let r = (self.rotation as f64).to_radians();
            (r, r.cos().round(), r.sin().round())
        };

        let raw_pins = match self.component_type {
            ComponentType::Resistor | ComponentType::Capacitor | ComponentType::Inductor => {
                vec![("p1", "1", -40.0, 0.0), ("p2", "2", 40.0, 0.0)]
            }
            ComponentType::Diode | ComponentType::ZenerDiode | ComponentType::Led => vec![
                ("anode", "+", -40.0, 0.0),
                ("cathode", "-", 40.0, 0.0),
            ],
            ComponentType::Potentiometer => vec![
                ("term1", "1", -40.0, 0.0),
                ("wiper", "W", 0.0, -30.0),
                ("term2", "2", 40.0, 0.0),
            ],
            ComponentType::ProbeVoltage
            | ComponentType::ProbeVoltagePos
            | ComponentType::ProbeVoltageNeg => vec![
                ("probe", "V", 0.0, 0.0),
            ],
            ComponentType::ProbeCurrent => vec![
                ("in", "I+", -20.0, 0.0),
                ("out", "I-", 20.0, 0.0),
            ],
            ComponentType::Switch => vec![
                ("p1", "1", -40.0, 0.0),
                ("p2", "2", 40.0, 0.0),
            ],
            ComponentType::Transformer => vec![
                ("p1", "P+", -30.0, -30.0),
                ("p2", "P-", -30.0, 30.0),
                ("s1", "S+", 30.0, -30.0),
                ("s2", "S-", 30.0, 30.0),
            ],
            ComponentType::Vreg => vec![
                ("in", "IN", -30.0, 0.0),
                ("gnd", "GND", 0.0, 30.0),
                ("out", "OUT", 30.0, 0.0),
            ],
            ComponentType::Vdc
            | ComponentType::Vac
            | ComponentType::Vsin
            | ComponentType::Vpulse => vec![("pos", "+", 0.0, -40.0), ("neg", "-", 0.0, 40.0)],
            ComponentType::Idc | ComponentType::Iac => vec![("pos", "+", 0.0, -40.0), ("neg", "-", 0.0, 40.0)],
            ComponentType::Ground => vec![("gnd", "0", 0.0, -20.0)],
            ComponentType::BjtNpn | ComponentType::BjtPnp => vec![
                ("base", "B", -30.0, 0.0),
                ("collector", "C", 20.0, -30.0),
                ("emitter", "E", 20.0, 30.0),
            ],
            ComponentType::Nmos | ComponentType::Pmos => vec![
                ("gate", "G", -30.0, 0.0),
                ("drain", "D", 20.0, -30.0),
                ("source", "S", 20.0, 30.0),
            ],
            ComponentType::OpAmp => vec![
                ("in_inv", "-", -40.0, -20.0),
                ("in_noninv", "+", -40.0, 20.0),
                ("out", "OUT", 40.0, 0.0),
            ],
        };

        raw_pins
            .into_iter()
            .map(|(id, name, ox, oy)| {
                let mx = if self.mirror { -ox } else { ox };
                let rx = mx * cos_r - oy * sin_r;
                let ry = mx * sin_r + oy * cos_r;
                ComponentPin {
                    id: id.to_string(),
                    name: name.to_string(),
                    offset_x: rx,
                    offset_y: ry,
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Wire {
    pub id: String,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetLabel {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Circuit {
    pub components: Vec<Component>,
    pub wires: Vec<Wire>,
    pub junctions: Vec<Point>,
    pub net_labels: Vec<NetLabel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AnalysisType {
    OperatingPoint,
    Transient,
    Ac,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransientAnalysis {
    pub step: String,
    pub stop: String,
    pub start: Option<String>,
    pub max_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum AcSweepType {
    Decade,
    Octave,
    Linear,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AcAnalysis {
    pub sweep_type: AcSweepType,
    pub points: usize,
    pub start_freq: String,
    pub stop_freq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SimulationConfig {
    pub analysis_type: AnalysisType,
    pub transient: Option<TransientAnalysis>,
    pub ac: Option<AcAnalysis>,
    pub dc_op: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: String,
    pub name: String,
    pub description: Option<String>,
    pub created: String,
    pub modified: String,
    pub simulation: SimulationConfig,
    pub circuit: Circuit,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            schema_version: "1.0.0".to_string(),
            name: "Untitled Circuit".to_string(),
            description: None,
            created: chrono::Utc::now().to_rfc3339(),
            modified: chrono::Utc::now().to_rfc3339(),
            simulation: SimulationConfig {
                analysis_type: AnalysisType::Transient,
                transient: Some(TransientAnalysis {
                    step: "10u".to_string(),
                    stop: "5m".to_string(),
                    start: Some("0".to_string()),
                    max_step: Some("10u".to_string()),
                }),
                ac: Some(AcAnalysis {
                    sweep_type: AcSweepType::Decade,
                    points: 50,
                    start_freq: "10".to_string(),
                    stop_freq: "1Meg".to_string(),
                }),
                dc_op: true,
            },
            circuit: Circuit {
                components: Vec::new(),
                wires: Vec::new(),
                junctions: Vec::new(),
                net_labels: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub component_id: Option<String>,
    pub is_fatal: bool,
}

impl Circuit {
    pub fn validate(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // 1. Check for Ground node
        let has_ground = self
            .components
            .iter()
            .any(|c| c.component_type == ComponentType::Ground);
        if !has_ground {
            errors.push(ValidationError {
                code: "NO_GROUND".to_string(),
                message: "No Ground (0V reference) node found. Every SPICE circuit requires a ground reference."
                    .to_string(),
                component_id: None,
                is_fatal: true,
            });
        }

        // 2. Check for duplicate reference designators
        let mut seen_designators: HashSet<String> = HashSet::new();
        for c in &self.components {
            if c.component_type != ComponentType::Ground {
                let d_upper = c.designator.to_uppercase();
                if seen_designators.contains(&d_upper) {
                    errors.push(ValidationError {
                        code: "DUPLICATE_DESIGNATOR".to_string(),
                        message: format!("Duplicate component designator: '{}'", c.designator),
                        component_id: Some(c.id.clone()),
                        is_fatal: true,
                    });
                } else {
                    seen_designators.insert(d_upper);
                }
            }
        }

        // 3. Check for empty or invalid component values (except Ground and Probes)
        for c in &self.components {
            if c.component_type != ComponentType::Ground
                && c.component_type != ComponentType::ProbeVoltage
                && c.component_type != ComponentType::ProbeVoltagePos
                && c.component_type != ComponentType::ProbeVoltageNeg
                && c.component_type != ComponentType::ProbeCurrent
                && c.value.trim().is_empty()
            {
                errors.push(ValidationError {
                    code: "EMPTY_VALUE".to_string(),
                    message: format!("Component '{}' has an empty value.", c.designator),
                    component_id: Some(c.id.clone()),
                    is_fatal: true,
                });
            }
        }

        errors
    }
}
