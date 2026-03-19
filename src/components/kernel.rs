use std::path::{Path, PathBuf};

use crate::filesystem::slurp;

use crate::Collector;
use anyhow::Result;
use serde::Serialize;
use serde_json::to_value;

#[derive(Serialize, Debug)]
pub struct KernelFacts {
    pub ostype: String,
    pub arch: String,
    pub release: String,
    pub version: String,
    pub majorversion: String,
}

pub struct KernelComponent {
    fsroot: PathBuf,
}

impl KernelComponent {
    pub fn new() -> Self {
        Self {
            fsroot: Path::new("/").to_path_buf(),
        }
    }

    #[cfg(test)]
    fn with_root(fsroot: PathBuf) -> Self {
        Self { fsroot: fsroot }
    }
}

impl Collector for KernelComponent {
    fn name(&self) -> &'static str {
        return "kernel";
    }

    fn collect(&self) -> Result<serde_json::Value> {
        let fsroot = &self.fsroot;
        let ostype = slurp(fsroot.join("proc/sys/kernel/ostype"))?;
        let arch = slurp(fsroot.join("proc/sys/kernel/arch"))?;
        let release = slurp(fsroot.join("proc/sys/kernel/osrelease"))?;

        let version = parse_version(&release);
        let majorversion = parse_majorversion(&release);

        let kf = KernelFacts {
            ostype,
            arch,
            release,
            majorversion,
            version,
        };
        Ok(to_value(kf)?)
    }
}

fn parse_version(release: &str) -> String {
    release
        .split_once("-")
        .map(|(v, _)| v)
        .unwrap_or(&release)
        .to_string()
}

fn parse_majorversion(release: &str) -> String {
    release.split(".").take(2).collect::<Vec<_>>().join(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("5.15.0-48-generic"), "5.15.0");
        assert_eq!(parse_version("5.15.0-48"), "5.15.0");
        assert_eq!(parse_version("5.15.0"), "5.15.0");
        assert_eq!(parse_version("5.15"), "5.15");
        assert_eq!(parse_version("5"), "5");
    }

    #[test]
    fn test_parse_majorversion() {
        assert_eq!(parse_majorversion("5.15.0-48-generic"), "5.15");
        assert_eq!(parse_majorversion("5.15.0-48"), "5.15");
        assert_eq!(parse_majorversion("5.15.0"), "5.15");
        assert_eq!(parse_majorversion("5.15"), "5.15");
        assert_eq!(parse_majorversion("5"), "5");
    }

    #[test]
    fn test_kernel_component_common() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("proc/sys/kernel")).unwrap();
        fs::write(root.join("proc/sys/kernel/ostype"), "Linux").unwrap();
        fs::write(root.join("proc/sys/kernel/arch"), "x86_64").unwrap();
        fs::write(root.join("proc/sys/kernel/osrelease"), "5.15.0-48-generic").unwrap();

        let kernel_component = KernelComponent::with_root(root.to_path_buf());
        let facts = kernel_component.collect().unwrap();

        assert_eq!(facts["ostype"], "Linux");
        assert_eq!(facts["arch"], "x86_64");
        assert_eq!(facts["release"], "5.15.0-48-generic");
        assert_eq!(facts["version"], "5.15.0");
        assert_eq!(facts["majorversion"], "5.15");
    }

    #[test]
    fn test_kernel_component_weird() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("proc/sys/kernel")).unwrap();
        fs::write(root.join("proc/sys/kernel/ostype"), "Linux").unwrap();
        fs::write(root.join("proc/sys/kernel/arch"), "x86_64").unwrap();
        fs::write(root.join("proc/sys/kernel/osrelease"), "5.15.0-48").unwrap();

        let kernel_component = KernelComponent::with_root(root.to_path_buf());
        let facts = kernel_component.collect().unwrap();

        assert_eq!(facts["ostype"], "Linux");
        assert_eq!(facts["arch"], "x86_64");
        assert_eq!(facts["release"], "5.15.0-48");
        assert_eq!(facts["version"], "5.15.0");
        assert_eq!(facts["majorversion"], "5.15");
    }
}
