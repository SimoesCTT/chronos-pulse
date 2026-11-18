//! # Temporal Collector
//! Zero-FFI, pure-Rust system telemetry scraper reading directly from Linux procfs and sysfs.

use std::io;
use std::path::Path;
use tokio::fs;

/// Snapshot of raw hardware, per-core frequency, block I/O counters, and memory at a given instant.
#[derive(Debug, Clone, Default)]
pub struct SystemSnapshot {
    pub cpu_user: u64,
    pub cpu_system: u64,
    pub cpu_idle: u64,
    pub cpu_iowait: u64,
    pub core_frequencies_mhz: Vec<f64>,
    pub disk_sectors_read: u64,
    pub disk_sectors_written: u64,
    pub disk_io_time_ms: u64,
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
}

impl SystemSnapshot {
    /// Sample current CPU, per-core frequencies, block device I/O, and memory metrics directly from procfs and sysfs.
    pub async fn capture() -> io::Result<Self> {
        let mut snapshot = Self::default();

        // 1. Parse aggregate CPU line from /proc/stat
        let stat_content = fs::read_to_string("/proc/stat").await?;
        for line in stat_content.lines() {
            if let Some(rest) = line.strip_prefix("cpu ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 4 {
                    snapshot.cpu_user = parts[0].parse().unwrap_or(0);
                    snapshot.cpu_system = parts[2].parse().unwrap_or(0);
                    snapshot.cpu_idle = parts[3].parse().unwrap_or(0);
                    if parts.len() >= 5 {
                        snapshot.cpu_iowait = parts[4].parse().unwrap_or(0);
                    }
                }
                break;
            }
        }

        // 2. Parse per-core frequencies from sysfs (/sys/devices/system/cpu/cpu*/cpufreq/scaling_cur_freq)
        // Fall back to /proc/cpuinfo if sysfs cpufreq is unavailable.
        let mut frequencies = Vec::new();
        let mut core_id = 0;
        loop {
            let freq_path = format!(
                "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_cur_freq",
                core_id
            );
            if !Path::new(&freq_path).exists() {
                break;
            }
            if let Ok(content) = fs::read_to_string(&freq_path).await {
                if let Ok(khz) = content.trim().parse::<f64>() {
                    frequencies.push(khz / 1000.0); // Convert kHz to MHz
                }
            }
            core_id += 1;
            if core_id > 256 {
                break; // Safety guard for massive core counts
            }
        }

        if frequencies.is_empty() {
            // Fallback: parse "cpu MHz" from /proc/cpuinfo
            if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo").await {
                for line in cpuinfo.lines() {
                    if let Some(rest) = line.strip_prefix("cpu MHz") {
                        if let Some(val_str) = rest.split(':').nth(1) {
                            if let Ok(mhz) = val_str.trim().parse::<f64>() {
                                frequencies.push(mhz);
                            }
                        }
                    }
                }
            }
        }
        snapshot.core_frequencies_mhz = frequencies;

        // 3. Parse block device I/O stats from /proc/diskstats (aggregating major physical drives like sd* or nvme*n*)
        if let Ok(diskstats) = fs::read_to_string("/proc/diskstats").await {
            let mut total_read_sectors = 0u64;
            let mut total_written_sectors = 0u64;
            let mut total_io_ms = 0u64;

            for line in diskstats.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 14 {
                    let dev_name = parts[2];
                    // Filter for standard physical block devices (e.g. nvme0n1, sda, vda) while ignoring partitions (e.g. sda1)
                    if (dev_name.starts_with("sd") && dev_name.as_bytes().last().map_or(false, |b| b.is_ascii_lowercase()))
                        || (dev_name.starts_with("nvme") && dev_name.contains("n1"))
                        || (dev_name.starts_with("vd") && dev_name.as_bytes().last().map_or(false, |b| b.is_ascii_lowercase()))
                    {
                        total_read_sectors += parts[5].parse::<u64>().unwrap_or(0);
                        total_written_sectors += parts[9].parse::<u64>().unwrap_or(0);
                        total_io_ms += parts[12].parse::<u64>().unwrap_or(0);
                    }
                }
            }
            snapshot.disk_sectors_read = total_read_sectors;
            snapshot.disk_sectors_written = total_written_sectors;
            snapshot.disk_io_time_ms = total_io_ms;
        }

        // 4. Parse MemTotal and MemAvailable from /proc/meminfo
        let mem_content = fs::read_to_string("/proc/meminfo").await?;
        for line in mem_content.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                snapshot.mem_total_kb = parse_kb_value(rest);
            } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
                snapshot.mem_available_kb = parse_kb_value(rest);
            }
        }

        Ok(snapshot)
    }

    /// Compute active CPU ratio between two sequential snapshots (0.0 to 1.0).
    pub fn cpu_utilization_since(&self, prev: &Self) -> f64 {
        let prev_idle = prev.cpu_idle + prev.cpu_iowait;
        let curr_idle = self.cpu_idle + self.cpu_iowait;

        let prev_total = prev.cpu_user + prev.cpu_system + prev_idle;
        let curr_total = self.cpu_user + self.cpu_system + curr_idle;

        let total_delta = curr_total.saturating_sub(prev_total);
        let idle_delta = curr_idle.saturating_sub(prev_idle);

        if total_delta == 0 {
            0.0
        } else {
            (total_delta.saturating_sub(idle_delta)) as f64 / total_delta as f64
        }
    }

    /// Compute memory utilization ratio (0.0 to 1.0).
    pub fn memory_utilization(&self) -> f64 {
        if self.mem_total_kb == 0 {
            0.0
        } else {
            let used = self.mem_total_kb.saturating_sub(self.mem_available_kb);
            used as f64 / self.mem_total_kb as f64
        }
    }
}

fn parse_kb_value(s: &str) -> u64 {
    s.split_whitespace()
        .next()
        .and_then(|num| num.parse::<u64>().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capture_procfs() {
        let snap = SystemSnapshot::capture().await.expect("Failed to read procfs/sysfs");
        assert!(snap.mem_total_kb > 0);
        println!("Captured {} CPU cores and {} sectors read", snap.core_frequencies_mhz.len(), snap.disk_sectors_read);
    }
}
