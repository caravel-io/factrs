// This file was unfortunately mostly written by claude,
// though reviewed by me every step of the way.

use std::collections::HashSet;
use std::path::Path;

use crate::filesystem::slurp;

use crate::Collector;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::to_value;

#[derive(Serialize, Debug)]
pub struct CPUFacts {
    pub count: u32,
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub model: Vec<String>,
    pub architecture: String,
}

pub struct CPUComponent;

impl CPUComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Collector for CPUComponent {
    fn name(&self) -> &'static str {
        return "cpu";
    }

    fn collect(&self) -> Result<serde_json::Value> {
        let cf = get_cpu_info()?;
        Ok(to_value(cf)?)
    }
}

fn get_cpuinfo_contents() -> Result<String> {
    let content = slurp(Path::new("/proc/cpuinfo")).context("failed to read cpuinfo")?;
    Ok(content)
}

fn get_cpu_info() -> Result<CPUFacts> {
    let cpuinfo_contents = get_cpuinfo_contents()?;

    let cpu_count = get_cpu_count(&cpuinfo_contents);
    let phys_core_count = get_physical_core_count(&cpuinfo_contents, cpu_count);
    let log_core_count = get_logical_core_count(&cpuinfo_contents);
    let arch = get_architecture(&cpuinfo_contents);

    let model = get_cpu_model(&cpuinfo_contents);

    let cf = CPUFacts {
        count: cpu_count,
        physical_cores: phys_core_count,
        logical_cores: log_core_count,
        architecture: arch,
        model: model,
    };
    Ok(cf)
}

fn get_cpu_count(contents: &str) -> u32 {
    let ids: HashSet<&str> = contents
        .lines()
        .filter_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == "physical id").then(|| v.trim())
        })
        .collect();

    if ids.is_empty() { 1 } else { ids.len() as u32 }
}

fn get_physical_core_count(contents: &str, cpu_count: u32) -> u32 {
    // "cpu cores" reports cores per socket on x86
    let cores_per_socket = contents.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == "cpu cores").then(|| v.trim().parse::<u32>().ok())?
    });

    match cores_per_socket {
        Some(cores) => cores * cpu_count,
        // ARM doesn't have "cpu cores" so logical == physical
        None => get_logical_core_count(contents),
    }
}

fn get_logical_core_count(contents: &str) -> u32 {
    contents
        .lines()
        .filter(|line| line.trim_start().starts_with("processor"))
        .filter(|line| line.split_once(':').is_some())
        .count() as u32
}

fn get_cpu_model(contents: &str) -> Vec<String> {
    // x86: deduplicated "model name" values (multi-socket systems may have different CPUs)
    let models: HashSet<String> = contents
        .lines()
        .filter_map(|line| {
            let (k, v) = line.split_once(':')?;
            (k.trim() == "model name").then(|| v.trim().to_string())
        })
        .collect();

    if !models.is_empty() {
        let mut v: Vec<String> = models.into_iter().collect();
        v.sort();
        return v;
    }

    // ARM fallback: /proc/device-tree/model (e.g. "Apple M1", "Raspberry Pi 4 Model B")
    // Device-tree strings are null-terminated, so trim \0
    if let Ok(model) = slurp(Path::new("/proc/device-tree/model")) {
        let model = model.trim_matches('\0').trim().to_string();
        if !model.is_empty() {
            return vec![model];
        }
    }

    // Last resort: construct a generic string from CPU implementer and part
    let implementer = contents.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == "CPU implementer").then(|| v.trim().to_string())
    });
    let part = contents.lines().find_map(|line| {
        let (k, v) = line.split_once(':')?;
        (k.trim() == "CPU part").then(|| v.trim().to_string())
    });

    match (implementer, part) {
        (Some(imp), Some(prt)) => vec![format!("ARM (implementer={}, part={})", imp, prt)],
        _ => vec![],
    }
}

fn get_architecture(contents: &str) -> String {
    // probably need to do better here...
    for line in contents.lines() {
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        match k.trim() {
            "flags" => {
                let flags: Vec<&str> = v.split_whitespace().collect();
                return if flags.contains(&"lm") {
                    "x86_64".to_string()
                } else {
                    "x86".to_string()
                };
            }
            "CPU architecture" => {
                return match v.trim() {
                    "8" => "aarch64".to_string(),
                    "7" => "armv7l".to_string(),
                    other => format!("arm_v{}", other),
                };
            }
            _ => continue,
        }
    }
    "unknown".to_string()
}
