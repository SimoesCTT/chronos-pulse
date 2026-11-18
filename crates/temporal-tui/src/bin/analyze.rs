//! # Chronos Analyze
//! Pure-Rust telemetry log analyzer for chronos-pulse JSONL output.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = if args.len() > 1 {
        &args[1]
    } else {
        "telemetry.jsonl"
    };

    println!("[*] Analyzing telemetry log: {}", path);

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[-] Error reading file '{}': {}", path, e);
            std::process::exit(1);
        }
    };

    let mut total_frames = 0;
    let mut anomaly_count = 0;
    let mut max_cpu = 0.0;
    let mut max_jitter = 0.0;
    let mut max_energy = 0.0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total_frames += 1;

        // Simple zero-dependency JSON field extractor for our flat schema
        let cpu = extract_f64(line, "cpu_ratio");
        let jitter = extract_f64(line, "jitter_ms");
        let real = extract_f64(line, "real");
        let imag = extract_f64(line, "imag");
        let anomaly = line.contains("\"anomaly\":true");

        if anomaly {
            anomaly_count += 1;
        }
        if cpu > max_cpu {
            max_cpu = cpu;
        }
        if jitter > max_jitter {
            max_jitter = jitter;
        }
        let energy = real * real + imag * imag;
        if energy > max_energy {
            max_energy = energy;
        }
    }

    println!("\n=== CHRONOS PULSE TELEMETRY REPORT ===");
    println!("Total Frames Analyzed : {}", total_frames);
    println!("Phase Anomalies Flagged : {}", anomaly_count);
    println!("Peak CPU Ratio          : {:.2}%", max_cpu * 100.0);
    println!("Peak Scheduler Jitter   : {:.2} ms", max_jitter);
    println!("Peak Energy Density     : {:.5}", max_energy);
    println!("========================================");
}

fn extract_f64(json: &str, key: &str) -> f64 {
    let pattern = format!("\"{}\":", key);
    if let Some(pos) = json.find(&pattern) {
        let start = pos + pattern.len();
        let rest = &json[start..];
        let end = rest
            .find(|c: char| c == ',' || c == '}' || c == ' ')
            .unwrap_or(rest.len());
        rest[..end].parse().unwrap_or(0.0)
    } else {
        0.0
    }
}
