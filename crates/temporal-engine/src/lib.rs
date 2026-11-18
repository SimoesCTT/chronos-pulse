//! # Temporal Engine
//! Asynchronous telemetry orchestrator, rolling phase-space ring buffer,
//! and instantaneous harmonic anomaly detection.

use std::collections::VecDeque;
use std::io;
use std::time::{Duration, Instant};
use temporal_collector::SystemSnapshot;
use temporal_core::{PhaseCoordinate, BASELINE_RESONANCE_HZ};
use tokio::sync::watch;

/// Current telemetry state emitted by the engine on each sampling tick.
#[derive(Debug, Clone)]
pub struct TelemetryFrame {
    pub cpu_ratio: f64,
    pub mem_ratio: f64,
    pub core_frequencies: Vec<f64>,
    pub disk_read_bytes_sec: f64,
    pub disk_write_bytes_sec: f64,
    pub phase: PhaseCoordinate,
    pub jitter_ms: f64,
    pub anomaly_detected: bool,
    pub history: Vec<PhaseCoordinate>,
}

impl Default for TelemetryFrame {
    fn default() -> Self {
        Self {
            cpu_ratio: 0.0,
            mem_ratio: 0.0,
            core_frequencies: Vec::new(),
            disk_read_bytes_sec: 0.0,
            disk_write_bytes_sec: 0.0,
            phase: PhaseCoordinate::new(0.0, 0.0, BASELINE_RESONANCE_HZ),
            jitter_ms: 0.0,
            anomaly_detected: false,
            history: Vec::new(),
        }
    }
}

/// Asynchronous engine that continuously samples system metrics and projects them
/// into a bounded phase-space ring buffer.
pub struct TemporalEngine {
    interval: Duration,
    capacity: usize,
}

impl TemporalEngine {
    /// Create a new engine with a given sampling interval and ring buffer capacity.
    pub fn new(interval: Duration, capacity: usize) -> Self {
        Self { interval, capacity }
    }

    /// Spawn the background sampling loop and return a watch receiver for live frames.
    pub fn spawn(self) -> watch::Receiver<TelemetryFrame> {
        let (tx, rx) = watch::channel(TelemetryFrame::default());

        tokio::spawn(async move {
            let mut prev_snap = SystemSnapshot::capture().await.unwrap_or_default();
            let mut ring: VecDeque<PhaseCoordinate> = VecDeque::with_capacity(self.capacity);
            let mut ticker = tokio::time::interval(self.interval);
            let mut last_tick = Instant::now();
            let mut step: f64 = 0.0;

            loop {
                ticker.tick().await;

                let now = Instant::now();
                let elapsed_sec = now.duration_since(last_tick).as_secs_f64();
                let elapsed_ms = elapsed_sec * 1000.0;
                let target_ms = self.interval.as_secs_f64() * 1000.0;
                let jitter_ms = (elapsed_ms - target_ms).abs();
                last_tick = now;

                if let Ok(curr_snap) = SystemSnapshot::capture().await {
                    let cpu_ratio = curr_snap.cpu_utilization_since(&prev_snap);
                    let mem_ratio = curr_snap.memory_utilization();

                    // Calculate disk throughput (sectors = 512 bytes)
                    let sectors_read_delta = curr_snap.disk_sectors_read.saturating_sub(prev_snap.disk_sectors_read);
                    let sectors_write_delta = curr_snap.disk_sectors_written.saturating_sub(prev_snap.disk_sectors_written);
                    
                    let dt = if elapsed_sec > 0.0 { elapsed_sec } else { 0.05 };
                    let disk_read_bytes_sec = (sectors_read_delta * 512) as f64 / dt;
                    let disk_write_bytes_sec = (sectors_write_delta * 512) as f64 / dt;

                    let core_frequencies = curr_snap.core_frequencies_mhz.clone();
                    prev_snap = curr_snap;

                    // Project CPU & memory dynamics onto rotating phase-space coordinates
                    step += 0.15;
                    let radius = (cpu_ratio * 0.7 + mem_ratio * 0.3).clamp(0.0, 1.0);
                    let theta = step + (jitter_ms * 0.05);
                    let freq = BASELINE_RESONANCE_HZ * (1.0 + (cpu_ratio - 0.5) * 0.01);

                    let coord = PhaseCoordinate::new(
                        radius * theta.cos(),
                        radius * theta.sin(),
                        freq,
                    );

                    if ring.len() == self.capacity {
                        ring.pop_front();
                    }
                    ring.push_back(coord);

                    // Flag phase disruption if energy density spikes or scheduler jitter exceeds threshold
                    let anomaly_detected = coord.energy_density() > 0.72 || jitter_ms > (target_ms * 0.35);

                    let frame = TelemetryFrame {
                        cpu_ratio,
                        mem_ratio,
                        core_frequencies,
                        disk_read_bytes_sec,
                        disk_write_bytes_sec,
                        phase: coord,
                        jitter_ms,
                        anomaly_detected,
                        history: ring.iter().copied().collect(),
                    };

                    if tx.send(frame).is_err() {
                        break;
                    }
                }
            }
        });

        rx
    }

    /// Run a single synchronous-style sample cycle for testing or CLI inspection.
    pub async fn sample_once(&self) -> io::Result<SystemSnapshot> {
        SystemSnapshot::capture().await
    }
}
