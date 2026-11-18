# Chronos Pulse

**Chronos Pulse** is a 100% pure-Rust, zero-FFI, phase-space system telemetry daemon and real-time terminal diagnostics suite. It breaks away from traditional linear time-series monitoring (`htop`, Prometheus, heavy metrics exporters) by mapping live hardware resource dynamics directly onto continuous temporal phase fields and harmonic resonance models.

---

## What is Chronos Pulse and Why Does It Exist?

Traditional system monitors treat performance as flat, isolated Cartesian coordinates (e.g., CPU at 45%, Memory at 60%). They rely on heavy C libraries (`libproc`), bloated web stacks, or heavy time-series databases that obscure micro-stutters and scheduling anomalies behind noisy log files.

**Chronos Pulse** approaches system telemetry through a deterministic lens:
1. **Zero C/C++ Dependencies (No FFI):** Written entirely in safe, zero-FFI Rust. It bypasses wrapper libraries and interfaces directly with Linux `procfs` and `sysfs` via raw, highly optimized asynchronous I/O.
2. **Phase-Domain Projection:** Instead of logging raw numerical dumps, it projects aggregate CPU utilization, memory pressure, per-core frequencies, and scheduler jitter into rotating phase-space vectors and energy-density trajectories.
3. **Instantaneous Anomaly Snapping:** Harmonic phase disruptions, thermal throttling spikes, or CPU scheduler lockups manifest immediately as phase angle deviations or energy-density threshold breaches rather than hiding in log files.
4. **Terminal-First & 60 FPS Native:** Renders ultra-smooth Braille canvas trajectory plots and subsystem meters via `ratatui` with negligible CPU overhead (< 0.1%).

---

## Workspace Architecture

The project is structured as a modular Cargo workspace designed for extreme modularity and auditability:

```text
chronos-pulse/
├── Cargo.toml
├── LICENSE
├── README.md
└── crates/
    ├── temporal-core/       # Pure math primitives, phase-space coords, and resonance models
    ├── temporal-collector/  # Zero-FFI procfs, sysfs, and diskstats parsers
    ├── temporal-engine/     # Asynchronous event loop, rolling ring buffer, and anomaly detection
    └── temporal-tui/        # Ratatui 60 FPS dashboard, CLI argument parser, and JSONL logger / analyzer
temporal-core: Provides the fundamental mathematical structures (PhaseCoordinate, energy density calculations, baseline resonance scaling).

temporal-collector: Asynchronously scrapes /proc/stat, /proc/meminfo, /proc/diskstats, and per-core scaling frequencies from /sys/devices/system/cpu/.

temporal-engine: Orchestrates sampling intervals, computes sliding-window delta throughput, and flags harmonic phase anomalies.

temporal-tui: Manages the terminal render loop, interactive controls, optional --log JSONL serialization, and the offline chronos-analyze utility.

Building and Running
Prerequisites
Linux system with Rust (edition 2024 / stable toolchain).

Build Release Binaries

cargo build --release --workspace

Run the Live TUI Dashboard

cargo run --release --bin chronos-pulse

Run with JSONL Telemetry Logging
To record live telemetry frames to disk for post-run analysis:

cargo run --release --bin chronos-pulse -- --log telemetry.jsonl

Analyze Logged Telemetry
To run the built-in pure-Rust analyzer report against a recorded JSONL log:

cargo run --release --bin chronos-analyze -- telemetry.jsonl

License
Distributed under the MIT License. See LICENSE for details.
