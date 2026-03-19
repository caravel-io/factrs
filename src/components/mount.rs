use crate::filesystem::slurp;

use crate::Collector;
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::to_value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize, Debug, Clone)]
pub struct MountPoint {
    pub location: String,
    pub device: String,
    pub filesystem: String,
    pub options: Vec<String>,
}

pub struct MountComponent;

impl MountComponent {
    pub fn new() -> Self {
        Self
    }
}
impl Collector for MountComponent {
    fn name(&self) -> &'static str {
        "mounts"
    }

    fn collect(&self) -> Result<serde_json::Value> {
        let contents = slurp(Path::new("/proc/mounts")).context("failed to read file")?;
        let mounts = parse_mounts(&contents);
        let facts = build_mount_facts(mounts);
        let j = to_value(facts).context("serializing to json value")?;
        Ok(j)
    }
}

fn parse_mounts(contents: &str) -> Vec<MountPoint> {
    contents
        .lines()
        .filter_map(|line| {
            let parts = line.split_whitespace().collect::<Vec<&str>>();
            if parts.len() < 4 {
                return None;
            }
            Some(MountPoint {
                location: parts[1].to_string(),
                device: parts[0].to_string(),
                filesystem: parts[2].to_string(),
                options: parts[3].split(",").map(str::to_string).collect(),
            })
        })
        .collect()
}

/// This function is used to build the mount facts.
/// Currently, it just exports the mounts from parsing, but this is where
/// we could add additional information into the mount points like data used, etc.
fn build_mount_facts(mounts: Vec<MountPoint>) -> HashMap<String, MountPoint> {
    let mut mount_results: HashMap<String, MountPoint> = HashMap::new();

    for mount in mounts.iter() {
        mount_results.insert(mount.location.clone(), mount.clone());
    }

    mount_results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mounts() {
        let contents = "/dev/sda1 / xfs rw,relatime 0 0\ntmpfs /tmp tmpfs rw,nosuid,nodev 0 0\n";
        let mounts = parse_mounts(contents);
        assert_eq!(mounts.len(), 2);

        let root = mounts.iter().find(|m| m.location == "/").unwrap();
        assert_eq!(root.device, "/dev/sda1");
        assert_eq!(root.filesystem, "xfs");
        assert_eq!(root.options, vec!["rw", "relatime"]);

        let tmp = mounts.iter().find(|m| m.location == "/tmp").unwrap();
        assert_eq!(tmp.device, "tmpfs");
        assert_eq!(tmp.filesystem, "tmpfs");
        assert_eq!(tmp.options, vec!["rw", "nosuid", "nodev"]);
    }

    #[test]
    fn test_parse_mounts_skips_short_lines() {
        let contents = "/dev/sda1 /\nvalid /mnt xfs rw 0 0\n";
        let mounts = parse_mounts(contents);
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].location, "/mnt");
    }

    #[test]
    fn test_build_mount_facts() {
        let mounts = vec![
            MountPoint {
                location: "/".to_string(),
                device: "/dev/sda1".to_string(),
                filesystem: "xfs".to_string(),
                options: vec!["rw".to_string()],
            },
            MountPoint {
                location: "/tmp".to_string(),
                device: "tmpfs".to_string(),
                filesystem: "tmpfs".to_string(),
                options: vec!["rw".to_string()],
            },
        ];
        let facts = build_mount_facts(mounts);
        assert_eq!(facts.len(), 2);
        assert_eq!(facts["/"].device, "/dev/sda1");
        assert_eq!(facts["/tmp"].filesystem, "tmpfs");
    }
}
