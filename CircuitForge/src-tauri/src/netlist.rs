use crate::model::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GridPoint(i64, i64);

impl GridPoint {
    fn from_f64(x: f64, y: f64) -> Self {
        Self(x.round() as i64, y.round() as i64)
    }
}

pub struct DisjointSet {
    parent: HashMap<GridPoint, GridPoint>,
}

impl DisjointSet {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    fn find(&mut self, p: &GridPoint) -> GridPoint {
        if !self.parent.contains_key(p) {
            self.parent.insert(p.clone(), p.clone());
            return p.clone();
        }
        let mut curr = p.clone();
        let mut path = Vec::new();
        while let Some(parent) = self.parent.get(&curr).cloned() {
            if parent == curr {
                break;
            }
            path.push(curr);
            curr = parent;
        }
        for node in path {
            self.parent.insert(node, curr.clone());
        }
        curr
    }

    fn union(&mut self, p1: &GridPoint, p2: &GridPoint) {
        let root1 = self.find(p1);
        let root2 = self.find(p2);
        if root1 != root2 {
            self.parent.insert(root1, root2);
        }
    }
}

pub struct NetlistGenerator;

impl NetlistGenerator {
    pub fn build_netlist(project: &Project) -> Result<String, String> {
        let circuit = &project.circuit;
        let mut dsu = DisjointSet::new();

        // 1. Connect all points in each wire
        for wire in &circuit.wires {
            for window in wire.points.windows(2) {
                let p1 = GridPoint::from_f64(window[0].x, window[0].y);
                let p2 = GridPoint::from_f64(window[1].x, window[1].y);
                dsu.union(&p1, &p2);
            }
        }

        // 2. Identify ground net root
        let mut ground_roots = HashSet::new();
        for c in &circuit.components {
            if c.component_type == ComponentType::Ground {
                for pin in c.get_pins() {
                    let pin_pt = GridPoint::from_f64(c.x + pin.offset_x, c.y + pin.offset_y);
                    ground_roots.insert(dsu.find(&pin_pt));
                }
            }
        }

        // 3. Map all named net labels
        let mut label_roots = HashMap::new();
        for lbl in &circuit.net_labels {
            let pt = GridPoint::from_f64(lbl.x, lbl.y);
            let root = dsu.find(&pt);
            label_roots.insert(root, lbl.name.clone());
        }

        // 4. Assign node IDs (Node 0 for ground, numbers for others)
        let mut node_mapping: HashMap<GridPoint, String> = HashMap::new();
        let mut next_node_id = 1;

        // Ground roots are always "0"
        for gr in &ground_roots {
            node_mapping.insert(gr.clone(), "0".to_string());
        }

        let mut get_node = |pt: &GridPoint, dsu: &mut DisjointSet| -> String {
            let root = dsu.find(pt);
            if ground_roots.contains(&root) {
                return "0".to_string();
            }
            if let Some(name) = label_roots.get(&root) {
                return name.clone();
            }
            if let Some(existing) = node_mapping.get(&root) {
                return existing.clone();
            }
            let name = format!("N{:03}", next_node_id);
            next_node_id += 1;
            node_mapping.insert(root, name.clone());
            name
        };

        // 5. Generate SPICE Netlist lines
        let mut lines = Vec::new();
        lines.push(format!(
            "* CircuitForge Generated SPICE Netlist: {}",
            project.name
        ));
        lines.push("* Clean-room Berkeley SPICE 3f5 / ngspice compatible".to_string());
        lines.push("".to_string());

        let mut models = HashSet::new();
        let mut subckts = HashSet::new();

        for c in &circuit.components {
            if c.component_type == ComponentType::Ground {
                continue;
            }

            let pins = c.get_pins();
            let pin_nodes: Vec<String> = pins
                .iter()
                .map(|p| {
                    let pt = GridPoint::from_f64(c.x + p.offset_x, c.y + p.offset_y);
                    get_node(&pt, &mut dsu)
                })
                .collect();

            match c.component_type {
                ComponentType::Resistor => {
                    let n1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let n2 = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    lines.push(format!("R_{} {} {} {}", c.designator, n1, n2, val));
                }
                ComponentType::Capacitor => {
                    let n1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let n2 = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    lines.push(format!("C_{} {} {} {}", c.designator, n1, n2, val));
                }
                ComponentType::Inductor => {
                    let n1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let n2 = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    lines.push(format!("L_{} {} {} {}", c.designator, n1, n2, val));
                }
                ComponentType::Diode => {
                    let na = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nc = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("DMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} D(IS=1e-14 RS=0.1 BV=100 IBV=1e-4)",
                        model_name
                    ));
                    lines.push(format!("D_{} {} {} {}", c.designator, na, nc, model_name));
                }
                ComponentType::ZenerDiode => {
                    let na = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nc = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let bv_str = Self::sanitize_spice_value(&c.value);
                    let bv = if bv_str.is_empty() { "5.1" } else { &bv_str };
                    let model_name = format!("ZENER_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} D(IS=1e-14 RS=0.5 BV={} IBV=5m)",
                        model_name, bv
                    ));
                    lines.push(format!("D_{} {} {} {}", c.designator, na, nc, model_name));
                }
                ComponentType::Led => {
                    let na = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nc = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("LEDMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} D(IS=1e-22 N=1.8 RS=2.0 BV=5 IBV=10u)",
                        model_name
                    ));
                    lines.push(format!("D_{} {} {} {}", c.designator, na, nc, model_name));
                }
                ComponentType::Potentiometer => {
                    let t1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let w = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let t2 = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let total_val = Self::sanitize_spice_value(&c.value);
                    let pos_ratio = c.properties.get("wiperPosition")
                        .and_then(|s| s.parse::<f64>().ok())
                        .unwrap_or(0.5)
                        .clamp(0.01, 0.99);
                    
                    // Synthesize as two series resistors splitting the total resistance
                    lines.push(format!("* Potentiometer {} wiper at {:.1}%", c.designator, pos_ratio * 100.0));
                    lines.push(format!("R_{}_TOP {} {} ({} * {:.4})", c.designator, t1, w, total_val, 1.0 - pos_ratio));
                    lines.push(format!("R_{}_BOT {} {} ({} * {:.4})", c.designator, w, t2, total_val, pos_ratio));
                }
                ComponentType::ProbeVoltage
                | ComponentType::ProbeVoltagePos
                | ComponentType::ProbeVoltageNeg
                | ComponentType::ProbeCurrent => {
                    // Voltage probes are non-loading measurement flags; current probes act as zero-volt voltage sources
                    if c.component_type == ComponentType::ProbeCurrent {
                        let n_in = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                        let n_out = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                        lines.push(format!("V_AMPERE_{} {} {} 0V", c.designator, n_in, n_out));
                    }
                }
                ComponentType::Switch => {
                    let p1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let p2 = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let state_prop = c.properties.get("state").map(|s| s.to_lowercase()).unwrap_or_else(|| "closed".to_string());
                    let is_closed = state_prop == "closed" || state_prop == "on" || state_prop == "1";
                    let r_val = if is_closed { "0.001" } else { "100Meg" };
                    lines.push(format!("* Switch {} ({})", c.designator, if is_closed { "CLOSED" } else { "OPEN" }));
                    lines.push(format!("R_SW_{} {} {} {}", c.designator, p1, p2, r_val));
                }
                ComponentType::Transformer => {
                    let p1 = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let p2 = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let s1 = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let s2 = pin_nodes.get(3).unwrap_or(&"0".to_string()).clone();
                    let l_pri = c.properties.get("lPri").cloned().unwrap_or_else(|| "10m".to_string());
                    let l_sec = c.properties.get("lSec").cloned().unwrap_or_else(|| "10m".to_string());
                    let k = c.properties.get("k").cloned().unwrap_or_else(|| "0.999".to_string());
                    lines.push(format!("L_{}_PRI {} {} {}", c.designator, p1, p2, l_pri));
                    lines.push(format!("L_{}_SEC {} {} {}", c.designator, s1, s2, l_sec));
                    lines.push(format!("K_{} L_{}_PRI L_{}_SEC {}", c.designator, c.designator, c.designator, k));
                }
                ComponentType::Vreg => {
                    let n_in = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let n_gnd = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let n_out = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    subckts.insert(
                        ".SUBCKT VREG_5V 1 2 3\n* 1: IN, 2: GND, 3: OUT\nRIN 1 2 10Meg\nE1 4 2 1 2 0.0\nVREF 4 5 DC 5.0\nROUT 5 3 0.05\n.ENDS VREG_5V".to_string()
                    );
                    lines.push(format!("X_{} {} {} {} VREG_5V", c.designator, n_in, n_gnd, n_out));
                }
                ComponentType::BjtNpn => {
                    let nb = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nc = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let ne = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("QMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} NPN(IS=1e-14 BF=100 VAF=100)",
                        model_name
                    ));
                    lines.push(format!(
                        "Q_{} {} {} {} {}",
                        c.designator, nc, nb, ne, model_name
                    ));
                }
                ComponentType::BjtPnp => {
                    let nb = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nc = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let ne = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("QMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} PNP(IS=1e-14 BF=100 VAF=100)",
                        model_name
                    ));
                    lines.push(format!(
                        "Q_{} {} {} {} {}",
                        c.designator, nc, nb, ne, model_name
                    ));
                }
                ComponentType::Nmos => {
                    let ng = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nd = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let ns = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("MMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} NMOS(LEVEL=1 VTO=1.0 KP=2e-4)",
                        model_name
                    ));
                    lines.push(format!(
                        "M_{} {} {} {} {} {}",
                        c.designator, nd, ng, ns, ns, model_name
                    ));
                }
                ComponentType::Pmos => {
                    let ng = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nd = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let ns = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    let model_name = format!("MMOD_{}", c.designator);
                    models.insert(format!(
                        ".MODEL {} PMOS(LEVEL=1 VTO=-1.0 KP=1e-4)",
                        model_name
                    ));
                    lines.push(format!(
                        "M_{} {} {} {} {} {}",
                        c.designator, nd, ng, ns, ns, model_name
                    ));
                }
                ComponentType::OpAmp => {
                    let n_inv = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let n_noninv = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let n_out = pin_nodes.get(2).unwrap_or(&"0".to_string()).clone();
                    subckts.insert(
                        ".SUBCKT OPAMP_IDEAL 1 2 3\n* 1: In-, 2: In+, 3: Out\nRIN 2 1 10Meg\nE1 3 0 2 1 100000\nROUT 3 0 10\n.ENDS OPAMP_IDEAL".to_string()
                    );
                    lines.push(format!(
                        "X_{} {} {} {} OPAMP_IDEAL",
                        c.designator, n_inv, n_noninv, n_out
                    ));
                }
                ComponentType::Vdc => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    let ac_mag = c
                        .properties
                        .get("acMag")
                        .unwrap_or(&"1".to_string())
                        .clone();
                    lines.push(format!(
                        "V_{} {} {} DC {} AC {}",
                        c.designator, np, nn, val, ac_mag
                    ));
                }
                ComponentType::Vac => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = c.value.trim();
                    let (mag, phase) = if val.to_uppercase().starts_with("AC") {
                        let parts: Vec<&str> = val.split_whitespace().collect();
                        let m = parts.get(1).copied().unwrap_or("1.0");
                        let p = parts.get(2).copied().unwrap_or("0");
                        (m.to_string(), p.to_string())
                    } else {
                        let m = Self::sanitize_spice_value(val);
                        let p = c
                            .properties
                            .get("phase")
                            .cloned()
                            .unwrap_or_else(|| "0".to_string());
                        (if m.is_empty() { "1.0".to_string() } else { m }, p)
                    };
                    lines.push(format!(
                        "V_{} {} {} DC 0 AC {} {}",
                        c.designator, np, nn, mag, phase
                    ));
                }
                ComponentType::Vsin => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let v_str = c.value.trim();
                    let (voff, vampl, freq) = if v_str.to_uppercase().starts_with("SIN(") {
                        let inner = v_str
                            .trim_start_matches(|ch| ch == 's' || ch == 'S' || ch == 'i' || ch == 'I' || ch == 'n' || ch == 'N' || ch == '(')
                            .trim_end_matches(')')
                            .trim();
                        let parts: Vec<&str> = inner.split_whitespace().collect();
                        let vo = parts.get(0).copied().unwrap_or("0");
                        let va = parts.get(1).copied().unwrap_or("1");
                        let fr = parts.get(2).copied().unwrap_or("1k");
                        (vo.to_string(), va.to_string(), fr.to_string())
                    } else {
                        let vo = c.properties.get("voff").cloned().unwrap_or_else(|| "0".to_string());
                        let va = if v_str.is_empty() {
                            c.properties.get("vampl").cloned().unwrap_or_else(|| "1".to_string())
                        } else {
                            Self::sanitize_spice_value(v_str)
                        };
                        let fr = c.properties.get("freq").cloned().unwrap_or_else(|| "1k".to_string());
                        (vo, va, fr)
                    };
                    lines.push(format!(
                        "V_{} {} {} SIN({} {} {}) AC {}",
                        c.designator, np, nn, voff, vampl, freq, vampl
                    ));
                }
                ComponentType::Vpulse => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let v_str = c.value.trim();
                    let (v1, v2, td, tr, tf, pw, per) = if v_str.to_uppercase().starts_with("PULSE(") {
                        let inner = v_str
                            .trim_start_matches(|ch| ch == 'p' || ch == 'P' || ch == 'u' || ch == 'U' || ch == 'l' || ch == 'L' || ch == 's' || ch == 'S' || ch == 'e' || ch == 'E' || ch == '(')
                            .trim_end_matches(')')
                            .trim();
                        let parts: Vec<&str> = inner.split_whitespace().collect();
                        (
                            parts.get(0).copied().unwrap_or("0").to_string(),
                            parts.get(1).copied().unwrap_or("5").to_string(),
                            parts.get(2).copied().unwrap_or("0").to_string(),
                            parts.get(3).copied().unwrap_or("1n").to_string(),
                            parts.get(4).copied().unwrap_or("1n").to_string(),
                            parts.get(5).copied().unwrap_or("1m").to_string(),
                            parts.get(6).copied().unwrap_or("2m").to_string(),
                        )
                    } else {
                        let v1 = c.properties.get("v1").cloned().unwrap_or_else(|| "0".to_string());
                        let v2 = if v_str.is_empty() {
                            c.properties.get("v2").cloned().unwrap_or_else(|| "5".to_string())
                        } else {
                            Self::sanitize_spice_value(v_str)
                        };
                        let td = c.properties.get("td").cloned().unwrap_or_else(|| "0".to_string());
                        let tr = c.properties.get("tr").cloned().unwrap_or_else(|| "1n".to_string());
                        let tf = c.properties.get("tf").cloned().unwrap_or_else(|| "1n".to_string());
                        let pw = c.properties.get("pw").cloned().unwrap_or_else(|| "1m".to_string());
                        let per = c.properties.get("per").cloned().unwrap_or_else(|| "2m".to_string());
                        (v1, v2, td, tr, tf, pw, per)
                    };
                    lines.push(format!(
                        "V_{} {} {} PULSE({} {} {} {} {} {} {}) AC 1",
                        c.designator, np, nn, v1, v2, td, tr, tf, pw, per
                    ));
                }
                ComponentType::Idc => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    lines.push(format!("I_{} {} {} DC {} AC 1", c.designator, np, nn, val));
                }
                ComponentType::Iac => {
                    let np = pin_nodes.first().unwrap_or(&"0".to_string()).clone();
                    let nn = pin_nodes.get(1).unwrap_or(&"0".to_string()).clone();
                    let val = Self::sanitize_spice_value(&c.value);
                    let phase = c.properties.get("phase").cloned().unwrap_or_else(|| "0".to_string());
                    lines.push(format!("I_{} {} {} DC 0 AC {} {}", c.designator, np, nn, val, phase));
                }
                ComponentType::Ground => {}
            }
        }

        // Subcircuits
        for sc in subckts {
            lines.push("".to_string());
            lines.push(sc);
        }

        // Models
        for m in models {
            lines.push("".to_string());
            lines.push(m);
        }

        // Analysis directives
        lines.push("".to_string());
        lines.push("* Simulation Analysis Directives".to_string());

        let sim = &project.simulation;
        match sim.analysis_type {
            AnalysisType::OperatingPoint => {
                lines.push(".OP".to_string());
            }
            AnalysisType::Transient => {
                if let Some(tr) = &sim.transient {
                    let max_step_str = tr.max_step.as_deref().unwrap_or("");
                    let start_str = tr.start.as_deref().unwrap_or("0");
                    lines.push(format!(
                        ".TRAN {} {} {} {}",
                        tr.step, tr.stop, start_str, max_step_str
                    ));
                } else {
                    lines.push(".TRAN 10u 5m".to_string());
                }
            }
            AnalysisType::Ac => {
                if let Some(ac) = &sim.ac {
                    let sw_type = match ac.sweep_type {
                        AcSweepType::Decade => "DEC",
                        AcSweepType::Octave => "OCT",
                        AcSweepType::Linear => "LIN",
                    };
                    lines.push(format!(
                        ".AC {} {} {} {}",
                        sw_type, ac.points, ac.start_freq, ac.stop_freq
                    ));
                } else {
                    lines.push(".AC DEC 50 10 1Meg".to_string());
                }
            }
        }

        lines.push(".END".to_string());

        Ok(lines.join("\n"))
    }

    pub fn sanitize_spice_value(val: &str) -> String {
        let trimmed = val.trim();
        // Handle "Meg" vs "m" (Mega vs Milli)
        // If user writes "1M" or "1m", SPICE interprets as 1 milli.
        // If user wrote "1Meg" or "1MEG", keep it.
        // Strip trailing unit letters like 'V', 'A', 'F', 'H', 'Ohm', 'Ω'
        let s = trimmed
            .trim_end_matches('V')
            .trim_end_matches('v')
            .trim_end_matches('A')
            .trim_end_matches('a')
            .trim_end_matches('F')
            .trim_end_matches('f')
            .trim_end_matches('H')
            .trim_end_matches('h')
            .trim_end_matches("Ohm")
            .trim_end_matches("ohm")
            .trim_end_matches('Ω')
            .trim();

        if s.is_empty() {
            "0".to_string()
        } else {
            s.to_string()
        }
    }

    pub fn resolve_probes(circuit: &Circuit) -> Vec<(String, String, String)> {
        let mut dsu = DisjointSet::new();
        for wire in &circuit.wires {
            for window in wire.points.windows(2) {
                let p1 = GridPoint::from_f64(window[0].x, window[0].y);
                let p2 = GridPoint::from_f64(window[1].x, window[1].y);
                dsu.union(&p1, &p2);
            }
        }
        let mut ground_roots = HashSet::new();
        for c in &circuit.components {
            if c.component_type == ComponentType::Ground {
                for pin in c.get_pins() {
                    let pin_pt = GridPoint::from_f64(c.x + pin.offset_x, c.y + pin.offset_y);
                    ground_roots.insert(dsu.find(&pin_pt));
                }
            }
        }
        let mut label_roots = HashMap::new();
        for lbl in &circuit.net_labels {
            let pt = GridPoint::from_f64(lbl.x, lbl.y);
            let root = dsu.find(&pt);
            label_roots.insert(root, lbl.name.clone());
        }
        let mut node_mapping: HashMap<GridPoint, String> = HashMap::new();
        let mut next_node_id = 1;
        for gr in &ground_roots {
            node_mapping.insert(gr.clone(), "0".to_string());
        }

        let mut get_node = |pt: &GridPoint, dsu: &mut DisjointSet| -> String {
            let root = dsu.find(pt);
            if ground_roots.contains(&root) {
                return "0".to_string();
            }
            if let Some(name) = label_roots.get(&root) {
                return name.clone();
            }
            if let Some(existing) = node_mapping.get(&root) {
                return existing.clone();
            }
            let name = format!("N{:03}", next_node_id);
            next_node_id += 1;
            node_mapping.insert(root, name.clone());
            name
        };

        // Pre-assign nodes to components in order
        for c in &circuit.components {
            if c.component_type == ComponentType::Ground {
                continue;
            }
            for p in c.get_pins() {
                let pt = GridPoint::from_f64(c.x + p.offset_x, c.y + p.offset_y);
                let _ = get_node(&pt, &mut dsu);
            }
        }

        // Find all positive probes and negative probes
        let mut pos_probes = Vec::new();
        let mut neg_probes = Vec::new();

        for c in &circuit.components {
            match c.component_type {
                ComponentType::ProbeVoltagePos | ComponentType::ProbeVoltage => {
                    let pt = GridPoint::from_f64(c.x, c.y);
                    let node = get_node(&pt, &mut dsu);
                    pos_probes.push((c.designator.clone(), node));
                }
                ComponentType::ProbeVoltageNeg => {
                    let pt = GridPoint::from_f64(c.x, c.y);
                    let node = get_node(&pt, &mut dsu);
                    neg_probes.push((c.designator.clone(), node));
                }
                _ => {}
            }
        }

        let mut results = Vec::new();

        // Pair positive and negative probes
        for (i, (pos_des, pos_node)) in pos_probes.into_iter().enumerate() {
            if let Some((neg_des, neg_node)) = neg_probes.get(i) {
                let label = format!("V({} - {})", pos_des, neg_des);
                results.push((label, pos_node, neg_node.clone()));
            } else if let Some((neg_des, neg_node)) = neg_probes.first() {
                let label = format!("V({} - {})", pos_des, neg_des);
                results.push((label, pos_node, neg_node.clone()));
            } else {
                let label = format!("V({})", pos_des);
                results.push((label, pos_node, "0".to_string()));
            }
        }

        results
    }
}
