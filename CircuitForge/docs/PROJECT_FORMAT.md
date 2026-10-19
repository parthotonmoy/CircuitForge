# CircuitForge Project Format Specification (`.forge` / `.json`)

**Schema Version:** 1.0.0

CircuitForge projects are stored as standard UTF-8 JSON files (`.forge` or `.json`). All coordinates are represented in orthogonal grid units (standard grid pitch: 20 pixels).

---

## JSON Schema Overview

```json
{
  "$schema": "https://circuitforge.dev/schemas/v1/project.json",
  "schemaVersion": "1.0.0",
  "name": "RC Low-Pass Filter",
  "description": "Benchmark 1st-order low-pass filter with 1kHz cutoff",
  "created": "2026-10-19T10:00:00Z",
  "modified": "2026-10-19T10:30:00Z",
  "simulation": {
    "analysisType": "Transient",
    "transient": {
      "step": "10u",
      "stop": "5m",
      "start": "0",
      "maxStep": "10u"
    },
    "ac": {
      "sweepType": "Decade",
      "points": 50,
      "startFreq": "10",
      "stopFreq": "1Meg"
    },
    "dcOp": true
  },
  "circuit": {
    "components": [
      {
        "id": "c_v1",
        "designator": "V1",
        "componentType": "VDC",
        "value": "5V",
        "x": 100,
        "y": 200,
        "rotation": 0,
        "mirror": false,
        "properties": {
          "dc": "5V",
          "acMag": "1V",
          "acPhase": "0"
        }
      },
      {
        "id": "c_r1",
        "designator": "R1",
        "componentType": "Resistor",
        "value": "1k",
        "x": 220,
        "y": 140,
        "rotation": 90,
        "mirror": false,
        "properties": {}
      },
      {
        "id": "c_c1",
        "designator": "C1",
        "componentType": "Capacitor",
        "value": "159n",
        "x": 340,
        "y": 200,
        "rotation": 0,
        "mirror": false,
        "properties": {}
      },
      {
        "id": "c_gnd1",
        "designator": "GND1",
        "componentType": "Ground",
        "value": "0",
        "x": 100,
        "y": 280,
        "rotation": 0,
        "mirror": false,
        "properties": {}
      }
    ],
    "wires": [
      {
        "id": "w_1",
        "points": [
          { "x": 100, "y": 140 },
          { "x": 180, "y": 140 }
        ]
      },
      {
        "id": "w_2",
        "points": [
          { "x": 260, "y": 140 },
          { "x": 340, "y": 140 }
        ]
      }
    ],
    "junctions": [
      { "x": 340, "y": 140 }
    ],
    "netLabels": [
      { "name": "VOUT", "x": 340, "y": 130 }
    ]
  }
}
```

---

## File Integrity & Atomic Save Protocol

To protect against unexpected system termination or write failures:
1. Projects are serialized to a temporary file in the target directory: `<target_file>.tmp.<timestamp>`.
2. The temporary file is flushed and sync'd to disk.
3. An atomic filesystem rename (`std::fs::rename`) replaces the target file.
4. If deserialization encounters an unknown newer schema version, an informative error is returned indicating the required application version.
