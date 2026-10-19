use circuitforge_lib::model::*;
use circuitforge_lib::netlist::NetlistGenerator;
use std::collections::HashMap;

#[test]
fn test_benchmark_1_resistor_dc_source() {
    let mut project = Project::default();
    project.name = "Benchmark 1: Ohm's Law DC".to_string();
    project.simulation.analysis_type = AnalysisType::OperatingPoint;

    let v1 = Component {
        id: "v1".to_string(),
        designator: "V1".to_string(),
        component_type: ComponentType::Vdc,
        value: "10V".to_string(),
        x: 100.0,
        y: 200.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 200.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let gnd = Component {
        id: "gnd".to_string(),
        designator: "GND".to_string(),
        component_type: ComponentType::Ground,
        value: "0".to_string(),
        x: 100.0,
        y: 260.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let w1 = Wire {
        id: "w1".to_string(),
        points: vec![Point { x: 100.0, y: 160.0 }, Point { x: 160.0, y: 160.0 }],
    };

    let w2 = Wire {
        id: "w2".to_string(),
        points: vec![
            Point { x: 240.0, y: 160.0 },
            Point { x: 240.0, y: 240.0 },
            Point { x: 100.0, y: 240.0 },
        ],
    };

    project.circuit.components = vec![v1, r1, gnd];
    project.circuit.wires = vec![w1, w2];

    let netlist = NetlistGenerator::build_netlist(&project).expect("Failed to build netlist");
    let sim_result = circuitforge_lib::engine::SimulationEngine::run_simulation(&project, &netlist);
    assert!(
        sim_result.success,
        "Simulation should succeed: {:?}",
        sim_result.error_message
    );

    let current_v1 = sim_result
        .op_table
        .get("i(v_v1)")
        .or_else(|| sim_result.op_table.get("v_v1#branch"));
    if let Some(&curr) = current_v1 {
        assert!(
            (curr.abs() - 0.01).abs() < 1e-6,
            "Ohm's law current error: {}",
            curr
        );
    }
}

#[test]
fn test_benchmark_2_voltage_divider() {
    let mut project = Project::default();
    project.name = "Benchmark 2: Voltage Divider".to_string();
    project.simulation.analysis_type = AnalysisType::OperatingPoint;

    // Vin = 12V
    let v1 = Component {
        id: "v1".to_string(),
        designator: "V1".to_string(),
        component_type: ComponentType::Vdc,
        value: "12V".to_string(),
        x: 100.0,
        y: 200.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // R1 = 3k (top)
    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "3k".to_string(),
        x: 200.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // R2 = 1k (bottom, vertical: rotation 90)
    // pins with rotation 90: p1 at (240, 200 - 40) = (240, 160), p2 at (240, 200 + 40) = (240, 240)
    let r2 = Component {
        id: "r2".to_string(),
        designator: "R2".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 240.0,
        y: 200.0,
        rotation: 90,
        mirror: false,
        properties: HashMap::new(),
    };

    let gnd = Component {
        id: "gnd".to_string(),
        designator: "GND".to_string(),
        component_type: ComponentType::Ground,
        value: "0".to_string(),
        x: 100.0,
        y: 260.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let lbl_out = NetLabel {
        name: "VOUT".to_string(),
        x: 240.0,
        y: 160.0,
    };

    // Wire from V1 pos (100, 160) to R1 p1 (160, 160)
    let w1 = Wire {
        id: "w1".to_string(),
        points: vec![Point { x: 100.0, y: 160.0 }, Point { x: 160.0, y: 160.0 }],
    };

    // Wire from R2 p2 (240, 240) to V1 neg & GND (100, 240)
    let w2 = Wire {
        id: "w2".to_string(),
        points: vec![Point { x: 240.0, y: 240.0 }, Point { x: 100.0, y: 240.0 }],
    };

    project.circuit.components = vec![v1, r1, r2, gnd];
    project.circuit.wires = vec![w1, w2];
    project.circuit.net_labels = vec![lbl_out];

    let netlist = NetlistGenerator::build_netlist(&project).expect("Failed to build netlist");
    let sim_result = circuitforge_lib::engine::SimulationEngine::run_simulation(&project, &netlist);
    assert!(
        sim_result.success,
        "Simulation should succeed: {:?}",
        sim_result.error_message
    );

    // Vout = Vin * R2 / (R1 + R2) = 12 * 1k / 4k = 3.0V
    let vout = sim_result.op_table.get("v(vout)").copied().unwrap_or(0.0);
    assert!(
        (vout - 3.0).abs() < 1e-5,
        "Voltage divider formula mismatch: got {}",
        vout
    );
}

#[test]
fn test_benchmark_3_rc_step_transient() {
    let mut project = Project::default();
    project.name = "Benchmark 3: RC Step Response".to_string();
    project.simulation.analysis_type = AnalysisType::Transient;
    project.simulation.transient = Some(TransientAnalysis {
        step: "1u".to_string(),
        stop: "5m".to_string(),
        start: Some("0".to_string()),
        max_step: Some("1u".to_string()),
    });

    // Step pulse source from 0 to 10V
    let mut vpulse_props = HashMap::new();
    vpulse_props.insert("v1".to_string(), "0".to_string());
    vpulse_props.insert("v2".to_string(), "10".to_string());
    vpulse_props.insert("td".to_string(), "0".to_string());
    vpulse_props.insert("tr".to_string(), "1n".to_string());
    vpulse_props.insert("tf".to_string(), "1n".to_string());
    vpulse_props.insert("pw".to_string(), "10m".to_string());
    vpulse_props.insert("per".to_string(), "20m".to_string());

    let v1 = Component {
        id: "v1".to_string(),
        designator: "V1".to_string(),
        component_type: ComponentType::Vpulse,
        value: "10V".to_string(),
        x: 100.0,
        y: 200.0,
        rotation: 0,
        mirror: false,
        properties: vpulse_props,
    };

    // R = 1k
    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 200.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // C = 1uF (tau = R * C = 1ms)
    // Vertical rotation 90: p1 at (240, 160), p2 at (240, 240)
    let c1 = Component {
        id: "c1".to_string(),
        designator: "C1".to_string(),
        component_type: ComponentType::Capacitor,
        value: "1u".to_string(),
        x: 240.0,
        y: 200.0,
        rotation: 90,
        mirror: false,
        properties: HashMap::new(),
    };

    let gnd = Component {
        id: "gnd".to_string(),
        designator: "GND".to_string(),
        component_type: ComponentType::Ground,
        value: "0".to_string(),
        x: 100.0,
        y: 260.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let lbl_out = NetLabel {
        name: "VOUT".to_string(),
        x: 240.0,
        y: 160.0,
    };

    let w1 = Wire {
        id: "w1".to_string(),
        points: vec![Point { x: 100.0, y: 160.0 }, Point { x: 160.0, y: 160.0 }],
    };

    let w2 = Wire {
        id: "w2".to_string(),
        points: vec![Point { x: 240.0, y: 240.0 }, Point { x: 100.0, y: 240.0 }],
    };

    project.circuit.components = vec![v1, r1, c1, gnd];
    project.circuit.wires = vec![w1, w2];
    project.circuit.net_labels = vec![lbl_out];

    let netlist = NetlistGenerator::build_netlist(&project).expect("Failed to build netlist");
    let sim_result = circuitforge_lib::engine::SimulationEngine::run_simulation(&project, &netlist);
    assert!(
        sim_result.success,
        "Simulation should succeed: {:?}",
        sim_result.error_message
    );

    let tran_plot = sim_result
        .plots
        .iter()
        .find(|p| p.plot_name.to_lowercase().contains("transient"))
        .expect("Transient plot missing");
    let time_vec = tran_plot.real_data.get("time").expect("time data missing");
    let vout_vec = tran_plot
        .real_data
        .get("v(vout)")
        .expect("v(vout) data missing");

    // Check V(t) at t = 1ms (1 tau): theoretical value is 10 * (1 - e^-1) = 6.3212V
    let mut val_at_tau = None;
    for (idx, &t) in time_vec.iter().enumerate() {
        if (t - 0.001).abs() < 1e-5 {
            val_at_tau = Some(vout_vec[idx]);
            break;
        }
    }

    assert!(val_at_tau.is_some(), "Point at tau = 1ms not found");
    let v_tau = val_at_tau.unwrap();
    let expected = 10.0 * (1.0 - (-1.0_f64).exp());
    assert!(
        (v_tau - expected).abs() < 0.05,
        "RC step at 1 tau: expected {:.4}, got {:.4}",
        expected,
        v_tau
    );
}

#[test]
fn test_circuit_validation_rules() {
    let mut circuit = Circuit {
        components: Vec::new(),
        wires: Vec::new(),
        junctions: Vec::new(),
        net_labels: Vec::new(),
    };

    let errors = circuit.validate();
    assert!(errors.iter().any(|e| e.code == "NO_GROUND"));

    let r1 = Component {
        id: "1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 0.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };
    let r2 = Component {
        id: "2".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "2k".to_string(),
        x: 50.0,
        y: 0.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };
    circuit.components.push(r1);
    circuit.components.push(r2);

    let errors = circuit.validate();
    assert!(errors.iter().any(|e| e.code == "DUPLICATE_DESIGNATOR"));
}

#[test]
fn test_benchmark_4_rc_ac_analysis() {
    let mut project = Project::default();
    project.name = "Benchmark 4: RC Low-Pass AC".to_string();
    project.simulation.analysis_type = AnalysisType::Ac;
    project.simulation.ac = Some(AcAnalysis {
        sweep_type: AcSweepType::Decade,
        points: 50,
        start_freq: "10".to_string(),
        stop_freq: "1Meg".to_string(),
    });

    let mut vac_props = HashMap::new();
    vac_props.insert("mag".to_string(), "1.0".to_string());
    vac_props.insert("phase".to_string(), "0.0".to_string());

    let v1 = Component {
        id: "v1".to_string(),
        designator: "V1".to_string(),
        component_type: ComponentType::Vac,
        value: "AC 1.0 0.0".to_string(),
        x: 100.0,
        y: 200.0,
        rotation: 0,
        mirror: false,
        properties: vac_props,
    };

    // R = 1k
    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 200.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // C = 159.155nF -> Cutoff frequency f_c = 1 / (2 * pi * R * C) = 1 / (2 * 3.14159 * 1000 * 159.155e-9) ≈ 1000 Hz
    let c1 = Component {
        id: "c1".to_string(),
        designator: "C1".to_string(),
        component_type: ComponentType::Capacitor,
        value: "159.155n".to_string(),
        x: 240.0,
        y: 200.0,
        rotation: 90,
        mirror: false,
        properties: HashMap::new(),
    };

    let gnd = Component {
        id: "gnd".to_string(),
        designator: "GND".to_string(),
        component_type: ComponentType::Ground,
        value: "0".to_string(),
        x: 100.0,
        y: 260.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let lbl_out = NetLabel {
        name: "VOUT".to_string(),
        x: 240.0,
        y: 160.0,
    };

    let w1 = Wire {
        id: "w1".to_string(),
        points: vec![Point { x: 100.0, y: 160.0 }, Point { x: 160.0, y: 160.0 }],
    };

    let w2 = Wire {
        id: "w2".to_string(),
        points: vec![Point { x: 240.0, y: 240.0 }, Point { x: 100.0, y: 240.0 }],
    };

    project.circuit.components = vec![v1, r1, c1, gnd];
    project.circuit.wires = vec![w1, w2];
    project.circuit.net_labels = vec![lbl_out];

    let netlist = NetlistGenerator::build_netlist(&project).expect("Failed to build netlist");
    let sim_result = circuitforge_lib::engine::SimulationEngine::run_simulation(&project, &netlist);
    assert!(
        sim_result.success,
        "Simulation should succeed: {:?}",
        sim_result.error_message
    );

    let ac_plot = sim_result
        .plots
        .iter()
        .find(|p| p.plot_name.to_lowercase().contains("ac"))
        .expect("AC plot missing");

    // Check frequency and complex/real data
    let freq_vec = ac_plot
        .real_data
        .get("frequency")
        .expect("frequency missing");
    let vout_complex = ac_plot
        .complex_data
        .get("v(vout)")
        .expect("v(vout) complex data missing");

    // At cutoff frequency ~1000 Hz, magnitude |Vout| should be ~ 1/sqrt(2) = 0.707 (-3dB)
    let mut mag_at_fc = None;
    for (idx, &f) in freq_vec.iter().enumerate() {
        if (f - 1000.0).abs() < 50.0 {
            let c = &vout_complex[idx];
            let mag = (c.real * c.real + c.imag * c.imag).sqrt();
            mag_at_fc = Some(mag);
            break;
        }
    }

    assert!(
        mag_at_fc.is_some(),
        "Point near cutoff frequency 1000 Hz not found"
    );
    let mag = mag_at_fc.unwrap();
    assert!(
        (mag - 0.707).abs() < 0.05,
        "RC low-pass AC cutoff magnitude: expected ~0.707, got {:.4}",
        mag
    );
}

#[test]
fn test_circuit_missing_ground_rejection() {
    let mut project = Project::default();
    project.name = "Floating Circuit".to_string();
    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 100.0,
        y: 100.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };
    project.circuit.components = vec![r1];

    let errs = project.circuit.validate();
    assert!(
        !errs.is_empty(),
        "Should reject circuit with no ground node"
    );
    assert_eq!(errs[0].code, "NO_GROUND");
}

#[test]
fn test_differential_voltage_probes() {
    let mut project = Project::default();
    project.name = "Differential Voltage Probe Test".to_string();
    project.simulation.analysis_type = AnalysisType::Transient;
    project.simulation.transient = Some(TransientAnalysis {
        step: "10u".to_string(),
        stop: "1m".to_string(),
        start: Some("0".to_string()),
        max_step: Some("10u".to_string()),
    });

    // 12V DC source: node 1 (100, 160) to GND (100, 240)
    let v1 = Component {
        id: "v1".to_string(),
        designator: "V1".to_string(),
        component_type: ComponentType::Vdc,
        value: "12V".to_string(),
        x: 100.0,
        y: 200.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // R1 = 3k between (160, 160) and (240, 160)
    let r1 = Component {
        id: "r1".to_string(),
        designator: "R1".to_string(),
        component_type: ComponentType::Resistor,
        value: "3k".to_string(),
        x: 200.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // R2 = 1k between (240, 160) and (240, 240)
    let r2 = Component {
        id: "r2".to_string(),
        designator: "R2".to_string(),
        component_type: ComponentType::Resistor,
        value: "1k".to_string(),
        x: 240.0,
        y: 200.0,
        rotation: 90,
        mirror: false,
        properties: HashMap::new(),
    };

    let gnd = Component {
        id: "gnd".to_string(),
        designator: "GND".to_string(),
        component_type: ComponentType::Ground,
        value: "0".to_string(),
        x: 100.0,
        y: 260.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // Positive Probe placed across R1 at top node (160, 160) [12V]
    let vp_pos = Component {
        id: "vp_pos".to_string(),
        designator: "VP1+".to_string(),
        component_type: ComponentType::ProbeVoltagePos,
        value: "V+".to_string(),
        x: 160.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    // Negative Probe placed at divider node (240, 160) [3V]
    let vp_neg = Component {
        id: "vp_neg".to_string(),
        designator: "VP1-".to_string(),
        component_type: ComponentType::ProbeVoltageNeg,
        value: "V-".to_string(),
        x: 240.0,
        y: 160.0,
        rotation: 0,
        mirror: false,
        properties: HashMap::new(),
    };

    let w1 = Wire {
        id: "w1".to_string(),
        points: vec![Point { x: 100.0, y: 160.0 }, Point { x: 160.0, y: 160.0 }],
    };
    let w2 = Wire {
        id: "w2".to_string(),
        points: vec![Point { x: 240.0, y: 240.0 }, Point { x: 100.0, y: 240.0 }],
    };

    project.circuit.components = vec![v1, r1, r2, gnd, vp_pos, vp_neg];
    project.circuit.wires = vec![w1, w2];

    let netlist = NetlistGenerator::build_netlist(&project).expect("Netlist generation failed");
    let result = circuitforge_lib::engine::SimulationEngine::run_simulation(&project, &netlist);

    assert!(result.success, "Simulation failed: {:?}", result.error_message);
    let tran_plot = &result.plots[0];

    // Check that differential vector V(VP1+ - VP1-) was computed
    let diff_vec = tran_plot.real_data.get("V(VP1+ - VP1-)").expect("Differential vector missing");
    let first_val = diff_vec[0];
    
    // VR1 = 12V - 3V = 9V
    assert!(
        (first_val - 9.0).abs() < 1e-4,
        "Differential voltage measurement should be 9.0V, got {}",
        first_val
    );
}
