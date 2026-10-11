use std::process::Command;

use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema, PartialEq, Eq)]
pub struct ServiceReport {
    /// False when systemctl cannot be run on this machine.
    pub available: bool,
    pub message: Option<String>,
    pub units: Vec<UnitStatus>,
}

#[derive(Debug, Serialize, ToSchema, PartialEq, Eq)]
pub struct UnitStatus {
    pub name: String,
    pub description: String,
    pub active: String,
    pub sub: String,
    pub load: String,
    pub file_state: String,
    pub since: String,
    pub pid: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/api/services",
    tag = "services",
    responses((status = 200, description = "Status of the systemd units named in DESK_UNITS", body = ServiceReport))
)]
pub async fn services() -> Json<ServiceReport> {
    let units = configured_units();
    let report = tokio::task::spawn_blocking(move || inspect(&units))
        .await
        .unwrap_or_else(|_| unavailable("The service check stopped."));
    Json(report)
}

fn configured_units() -> Vec<String> {
    std::env::var("DESK_UNITS")
        .unwrap_or_default()
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect()
}

fn inspect(units: &[String]) -> ServiceReport {
    if units.is_empty() {
        return ServiceReport {
            available: true,
            message: Some("Set DESK_UNITS in .env to the systemd units you want to watch.".into()),
            units: Vec::new(),
        };
    }
    if let Some(name) = units.iter().find(|name| validate_unit(name).is_err()) {
        return unavailable(format!("{name:?} is not a systemd unit name."));
    }

    let output = Command::new("systemctl")
        .args([
            "show",
            "--no-pager",
            "-p",
            "Id",
            "-p",
            "Description",
            "-p",
            "ActiveState",
            "-p",
            "SubState",
            "-p",
            "LoadState",
            "-p",
            "UnitFileState",
            "-p",
            "ActiveEnterTimestamp",
            "-p",
            "MainPID",
        ])
        .args(units)
        .output();

    let output = match output {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return unavailable("systemctl is not on this machine.");
        }
        Err(error) => return unavailable(error.to_string()),
    };

    let units = parse_show(&String::from_utf8_lossy(&output.stdout));
    if output.status.success() || !units.is_empty() {
        return ServiceReport {
            available: true,
            message: None,
            units,
        };
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    unavailable(if detail.is_empty() {
        "systemctl could not read the units.".into()
    } else {
        detail.to_string()
    })
}

fn unavailable(message: impl Into<String>) -> ServiceReport {
    ServiceReport {
        available: false,
        message: Some(message.into()),
        units: Vec::new(),
    }
}

fn validate_unit(name: &str) -> Result<(), ()> {
    let allowed = !name.is_empty()
        && name.len() <= 256
        && !name.starts_with('-')
        && name.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '.' | '@' | '_' | '-' | ':' | '\\')
        });
    if allowed {
        Ok(())
    } else {
        Err(())
    }
}

pub fn parse_show(text: &str) -> Vec<UnitStatus> {
    text.split("\n\n")
        .filter_map(|block| {
            let mut status = UnitStatus {
                name: String::new(),
                description: String::new(),
                active: String::new(),
                sub: String::new(),
                load: String::new(),
                file_state: String::new(),
                since: String::new(),
                pid: None,
            };
            for line in block.lines() {
                let Some((key, value)) = line.split_once('=') else {
                    continue;
                };
                match key {
                    "Id" => status.name = value.to_string(),
                    "Description" => status.description = value.to_string(),
                    "ActiveState" => status.active = value.to_string(),
                    "SubState" => status.sub = value.to_string(),
                    "LoadState" => status.load = value.to_string(),
                    "UnitFileState" => status.file_state = value.to_string(),
                    "ActiveEnterTimestamp" if value != "n/a" => status.since = value.to_string(),
                    "MainPID" => status.pid = value.parse().ok().filter(|pid| *pid > 0),
                    _ => {}
                }
            }
            if status.name.is_empty() {
                None
            } else {
                Some(status)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_show, validate_unit};

    #[test]
    fn show_output_becomes_one_row_per_unit() {
        let units = parse_show(
            "Id=nginx.service\n\
             Description=A high performance web server\n\
             ActiveState=active\n\
             SubState=running\n\
             LoadState=loaded\n\
             UnitFileState=enabled\n\
             ActiveEnterTimestamp=Mon 2026-10-12 01:00:00 IST\n\
             MainPID=42\n\
             \n\
             Id=desk.service\n\
             Description=Option desk\n\
             ActiveState=inactive\n\
             SubState=dead\n\
             LoadState=not-found\n\
             UnitFileState=\n\
             ActiveEnterTimestamp=n/a\n\
             MainPID=0\n",
        );
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "nginx.service");
        assert_eq!(units[0].active, "active");
        assert_eq!(units[0].pid, Some(42));
        assert_eq!(units[1].load, "not-found");
        assert_eq!(units[1].pid, None);
        assert!(units[1].since.is_empty());
    }

    #[test]
    fn a_unit_name_cannot_smuggle_a_shell_command() {
        assert!(validate_unit("nginx.service").is_ok());
        assert!(validate_unit("getty@tty1.service").is_ok());
        assert!(validate_unit("nginx.service;reboot").is_err());
        assert!(validate_unit("-nginx.service").is_err());
    }
}
