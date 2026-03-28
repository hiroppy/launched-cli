use anyhow::Result;
use plist::Value;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

pub struct Service {
    pub label: String,
    pub plist_path: String,
    pub category: String,
    pub pid: Option<i32>,
    pub exit_code: Option<i32>,
    pub start_calendar_interval: Option<Vec<CalendarInterval>>,
    pub start_interval: Option<u64>,
    pub standard_error_path: Option<String>,
    pub source: ServiceSource,
}

#[derive(Clone, PartialEq)]
pub enum ServiceSource {
    UserAgent,
    SystemAgent,
    SystemDaemon,
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct CalendarInterval {
    pub weekday: Option<u8>,
    pub month: Option<u8>,
    pub day: Option<u8>,
    pub hour: Option<u8>,
    pub minute: Option<u8>,
}

impl Service {
    pub fn from_plist(path: &Path) -> Result<Self> {
        let val = Value::from_file(path)?;
        let dict = val.as_dictionary().ok_or_else(|| anyhow::anyhow!("not a dict"))?;

        let label_from_plist = dict
            .get("Label")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let label_from_filename = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let label = label_from_plist.unwrap_or(label_from_filename);

        if label.is_empty() {
            anyhow::bail!("no label found for {:?}", path);
        }

        let category = label
            .split('.')
            .nth(1)
            .unwrap_or("other")
            .to_string();

        let start_calendar_interval = parse_calendar_intervals(dict);
        let start_interval = dict
            .get("StartInterval")
            .and_then(|v| v.as_unsigned_integer());

        let standard_error_path = dict
            .get("StandardErrorPath")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let source = source_from_path(path);

        Ok(Service {
            label,
            plist_path: path.to_string_lossy().to_string(),
            category,
            pid: None,
            exit_code: None,
            start_calendar_interval,
            start_interval,
            standard_error_path,
            source,
        })
    }
}

fn source_from_path(path: &Path) -> ServiceSource {
    let path_str = path.to_string_lossy();
    if path_str.contains("/Library/LaunchDaemons/") {
        ServiceSource::SystemDaemon
    } else if path_str.starts_with("/Library/LaunchAgents/") {
        ServiceSource::SystemAgent
    } else {
        ServiceSource::UserAgent
    }
}

fn parse_calendar_intervals(dict: &plist::Dictionary) -> Option<Vec<CalendarInterval>> {
    let val = dict.get("StartCalendarInterval")?;
    let dicts = if let Some(arr) = val.as_array() {
        arr.iter()
            .filter_map(|v| v.as_dictionary())
            .collect::<Vec<_>>()
    } else if let Some(d) = val.as_dictionary() {
        vec![d]
    } else {
        return None;
    };

    let intervals: Vec<CalendarInterval> = dicts
        .into_iter()
        .map(|d| CalendarInterval {
            weekday: d.get("Weekday").and_then(|v| v.as_unsigned_integer()).map(|v| v as u8),
            month: d.get("Month").and_then(|v| v.as_unsigned_integer()).map(|v| v as u8),
            day: d.get("Day").and_then(|v| v.as_unsigned_integer()).map(|v| v as u8),
            hour: d.get("Hour").and_then(|v| v.as_unsigned_integer()).map(|v| v as u8),
            minute: d.get("Minute").and_then(|v| v.as_unsigned_integer()).map(|v| v as u8),
        })
        .collect();

    if intervals.is_empty() { None } else { Some(intervals) }
}

pub fn scan_plist_dirs() -> Vec<Service> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        format!("{}/Library/LaunchAgents", home),
        "/Library/LaunchAgents".to_string(),
        "/Library/LaunchDaemons".to_string(),
    ];

    let mut services = Vec::new();
    for dir in &dirs {
        let path = Path::new(dir);
        if !path.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().is_some_and(|e| e == "plist") {
                    if let Ok(svc) = Service::from_plist(&p) {
                        services.push(svc);
                    }
                }
            }
        }
    }
    services
}

pub struct LaunchctlStatus {
    pub pid: Option<i32>,
    pub exit_code: Option<i32>,
}

pub fn query_launchctl_list() -> HashMap<String, LaunchctlStatus> {
    let mut map = HashMap::new();
    let output = match Command::new("launchctl").arg("list").output() {
        Ok(o) => o,
        Err(_) => return map,
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let pid = parts[0].parse::<i32>().ok();
            let exit_code = parts[1].parse::<i32>().ok();
            let label = parts[2].to_string();
            map.insert(label, LaunchctlStatus { pid, exit_code });
        }
    }
    map
}

pub fn merge_status(services: &mut [Service], statuses: &HashMap<String, LaunchctlStatus>) {
    for svc in services.iter_mut() {
        if let Some(status) = statuses.get(&svc.label) {
            svc.pid = status.pid;
            svc.exit_code = status.exit_code;
        } else {
            svc.pid = None;
            svc.exit_code = None;
        }
    }
}

pub fn load_service(plist_path: &str) -> Result<()> {
    let output = Command::new("launchctl")
        .args(["load", plist_path])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("launchctl load failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

pub fn unload_service(plist_path: &str) -> Result<()> {
    let output = Command::new("launchctl")
        .args(["unload", plist_path])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("launchctl unload failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}
