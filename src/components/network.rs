use crate::Collector;
use crate::filesystem::slurp;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::to_value;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

// These are the structs used to deserialize from JSON
#[derive(Debug, Deserialize)]
struct IPDevice {
    ifname: String,
    mtu: u32,
    operstate: String,
    link_type: String,
    address: String,
    addr_info: Vec<AddrInfo>,
}

#[derive(Debug, Deserialize)]
struct AddrInfo {
    family: String,
    local: String,
    prefixlen: u32,
    scope: String,
}
// end JSON fields

// These are the structs used to format it how I want them to serialize
#[derive(Debug, Serialize)]
pub struct NetworkFacts {
    pub hostname: String,
    pub domain: Option<String>,
    pub fqdn: Option<String>,
    pub primary: Option<String>,
    pub ip: Option<String>,
    pub ip6: Option<String>,
    pub mac: Option<String>,
    pub mtu: Option<u32>,
    pub interfaces: HashMap<String, Interface>,
}
#[derive(Serialize, Debug)]
pub struct Interface {
    pub name: String,
    pub ip: Option<String>,
    pub prefix: Option<u32>,
    pub ip6: Option<String>,
    pub prefix6: Option<u32>,
    pub mtu: Option<u32>,
    pub mac: Option<String>,
    pub operational_state: String,
    pub link_type: String,
}
// end JSON serialize

pub struct NetworkComponent;
impl NetworkComponent {
    pub fn new() -> Self {
        Self
    }
}

impl Collector for NetworkComponent {
    fn name(&self) -> &'static str {
        "network"
    }

    fn collect(&self) -> Result<serde_json::Value> {
        let hostname = get_hostname()?;
        let domain = get_domain()?;
        let fqdn = build_fqdn(&hostname, &domain);

        let ip_devices_output = get_all_ip_devices_output()?;
        let system_devices = parse_ip_devices_output(&ip_devices_output)?;

        // ip is ordered by ifindex, primary should be first, skipping loopback
        let mut interfaces: HashMap<String, Interface> = HashMap::new();

        // primary device, will be filled out later
        let mut primary_ifname = None;
        let mut primary_ip = None;
        let mut primary_ip6 = None;
        let mut primary_mac = None;
        let mut primary_mtu = None;

        let mut primary_done = false;
        for device in system_devices {
            // properties to be filled out by iterating through the device infos
            let mut ip = None;
            let mut prefix = None;
            let mut ip6 = None;
            let mut prefix6 = None;

            for addr_info in &device.addr_info {
                if addr_info.scope == "link" {
                    // dunno what this is :)
                    continue;
                }
                if addr_info.family == "inet" {
                    ip = Some(addr_info.local.clone());
                    prefix = Some(addr_info.prefixlen);
                }
                if addr_info.family == "inet6" {
                    ip6 = Some(addr_info.local.clone());
                    prefix6 = Some(addr_info.prefixlen);
                }
            }

            // find the first occurrence of the "ether" device type
            // that will be our primary
            if !primary_done && device.link_type == "ether" {
                primary_ifname = Some(device.ifname.clone());
                primary_ip = ip.clone();
                primary_ip6 = ip6.clone();
                primary_mac = Some(device.address.clone());
                primary_mtu = Some(device.mtu.clone());
                primary_done = true;
            }

            interfaces.insert(
                device.ifname.clone(),
                Interface {
                    name: device.ifname.clone(),
                    operational_state: device.operstate.clone(),
                    mtu: Some(device.mtu),
                    mac: Some(device.address.clone()),
                    link_type: device.link_type.clone(),
                    ip: ip,
                    prefix: prefix,
                    ip6: ip6,
                    prefix6: prefix6,
                },
            );
        }
        let facts = NetworkFacts {
            hostname: hostname,
            domain: domain,
            fqdn: fqdn,
            primary: primary_ifname,
            ip: primary_ip,
            ip6: primary_ip6,
            mac: primary_mac,
            mtu: primary_mtu,
            interfaces: interfaces,
        };
        let j = to_value(facts).context("serializing to json value")?;
        Ok(j)
    }
}

fn get_hostname() -> Result<String> {
    Ok(slurp(Path::new("/proc/sys/kernel/hostname")).context("failed to read hostname")?)
}

fn get_domain() -> Result<Option<String>> {
    let domain =
        slurp(Path::new("/proc/sys/kernel/domainname")).context("failed to read domainname")?;
    if domain.is_empty() || domain == "(none)" {
        return Ok(None);
    }
    Ok(Some(domain))
}

fn build_fqdn(hostname: &str, domain: &Option<String>) -> Option<String> {
    return match domain {
        None => None,
        Some(d) => Some(format!("{}.{}", hostname, d)),
    };
}

fn parse_ip_devices_output(output: &str) -> Result<Vec<IPDevice>> {
    let devices: Vec<IPDevice> = serde_json::from_str(&output)?;
    Ok(devices)
}

fn get_all_ip_devices_output() -> Result<String> {
    let output = Command::new("ip")
        .arg("-j")
        .arg("addr")
        .arg("show")
        .output()
        .with_context(|| format!("running ip -j addr show"))?
        .stdout;
    let output = String::from_utf8(output)?;
    Ok(output.trim_end().to_string())
}
