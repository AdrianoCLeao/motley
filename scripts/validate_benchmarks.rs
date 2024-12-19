use std::fs;

fn main() {
    let path = "target/criterion";
    if !std::path::Path::new(path).exists() {
        eprintln!("No benchmarks were executed!");
        std::process::exit(1);
    }

    let summary_path = format!("{}/benchmark_summary.json", path);
    if !std::path::Path::new(&summary_path).exists() {
        eprintln!("Benchmark summary not found!");
        std::process::exit(1);
    }

    let summary = fs::read_to_string(summary_path).expect("Failed to read summary file");
    println!("Benchmark Summary: {}", summary);
}
