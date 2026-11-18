//! # Temporal Core
//! Pure math primitives, phase-space projection, and Riemann-zero resonance models
//! operating entirely in safe, zero-FFI Rust.

/// Fundamental resonance baseline (e.g. 587 kHz temporal locking scale)
pub const BASELINE_RESONANCE_HZ: f64 = 587_032.0;

/// A phase-space coordinate representing temporal viscosity and frequency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseCoordinate {
    pub real: f64,
    pub imag: f64,
    pub frequency: f64,
}

impl PhaseCoordinate {
    /// Create a new phase-space coordinate.
    pub fn new(real: f64, imag: f64, frequency: f64) -> Self {
        Self { real, imag, frequency }
    }

    /// Calculate magnitude squared (energy density proxy).
    pub fn energy_density(&self) -> f64 {
        self.real * self.real + self.imag * self.imag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_density() {
        let coord = PhaseCoordinate::new(3.0, 4.0, BASELINE_RESONANCE_HZ);
        assert_eq!(coord.energy_density(), 25.0);
    }
}
