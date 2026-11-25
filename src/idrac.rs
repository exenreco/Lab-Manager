//! Bounded, read-only iDRAC RACADM collection over the existing SSH transport.
use crate::{model::Server, monitoring::Config};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Health {
    Ok,
    Warning,
    Critical,
    #[default]
    Unknown,
}
impl Health {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warning => "Warning",
            Self::Critical => "Critical",
            Self::Unknown => "Unknown",
        }
    }
    fn parse(value: &str) -> Self {
        match value
            .trim()
            .to_ascii_lowercase()
            .replace([' ', '-'], "")
            .as_str()
        {
            "ok" | "healthy" | "normal" | "online" | "ready" | "optimal" => Self::Ok,
            "warning" | "noncritical" | "degraded" | "rebuilding" => Self::Warning,
            "critical" | "failed" | "error" | "nonrecoverable" | "offline" => Self::Critical,
            _ => Self::Unknown,
        }
    }
    fn merge(self, other: Self) -> Self {
        if self == Self::Critical || other == Self::Critical {
            Self::Critical
        } else if self == Self::Warning || other == Self::Warning {
            Self::Warning
        } else if self == Self::Ok || other == Self::Ok {
            Self::Ok
        } else {
            Self::Unknown
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sensor {
    pub name: String,
    pub category: String,
    pub health: Health,
    pub reading: String,
    pub value: Option<f64>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Storage {
    pub name: String,
    pub kind: String,
    pub health: Health,
    pub state: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Hardware {
    pub health: Health,
    pub model: String,
    pub power_state: String,
    pub peak_temperature: Option<f64>,
    pub power_watts: Option<f64>,
    pub minimum_fan_rpm: Option<f64>,
    pub sensors: Vec<Sensor>,
    pub storage: Vec<Storage>,
    pub issues: Vec<String>,
    pub notes: Vec<String>,
}
impl Hardware {
    pub fn compact(&mut self) {
        self.model.clear();
        self.sensors.clear();
        self.storage.clear();
        self.notes.clear();
    }
    fn summarize(&mut self) {
        self.health = Health::Unknown;
        let system_power = self.sensors.iter().any(|sensor| {
            sensor.name.to_ascii_lowercase().contains("pwr consumption") && sensor.value.is_some()
        });
        for sensor in &self.sensors {
            self.health = self.health.merge(sensor.health);
            if matches!(sensor.health, Health::Warning | Health::Critical) {
                self.issues
                    .push(format!("{}: {}", sensor.name, sensor.health.label()));
            }
            if let Some(value) = sensor.value {
                if sensor.category == "TEMPERATURE" {
                    self.peak_temperature = Some(self.peak_temperature.unwrap_or(value).max(value));
                }
                if (sensor.reading.to_ascii_lowercase().contains("watt")
                    || sensor.reading.to_ascii_lowercase().ends_with('w'))
                    && (!system_power
                        || sensor.name.to_ascii_lowercase().contains("pwr consumption"))
                {
                    self.power_watts = Some(self.power_watts.unwrap_or(value).max(value));
                }
                if sensor.category == "FAN" && sensor.reading.to_ascii_lowercase().contains("rpm") {
                    self.minimum_fan_rpm = Some(self.minimum_fan_rpm.unwrap_or(value).min(value));
                }
            }
        }
        for disk in &self.storage {
            self.health = self.health.merge(disk.health);
            if matches!(disk.health, Health::Warning | Health::Critical) {
                self.issues
                    .push(format!("{}: {}", disk.name, disk.health.label()));
            }
        }
        self.issues.truncate(32);
    }
}

fn numeric(value: &str) -> Option<f64> {
    let text: String = value
        .chars()
        .skip_while(|c| !c.is_ascii_digit() && *c != '-')
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    text.parse::<f64>().ok().filter(|v| v.is_finite())
}
pub fn parse_sensors(text: &str) -> Vec<Sensor> {
    let mut category = String::new();
    let mut sensors = Vec::new();
    for line in text.lines() {
        if let Some((_, value)) = line
            .split_once(':')
            .filter(|(key, _)| key.trim().eq_ignore_ascii_case("Sensor Type"))
        {
            category = value.trim().to_ascii_uppercase();
            continue;
        }
        if category.is_empty() || sensors.len() >= 128 {
            continue;
        }
        let words: Vec<_> = line.split_whitespace().collect();
        let Some(index) = words.iter().position(|word| {
            Health::parse(word) != Health::Unknown
                || matches!(
                    word.to_ascii_lowercase().as_str(),
                    "unknown" | "n/a" | "present" | "absent" | "other" | "not" | "closed" | "open"
                )
        }) else {
            continue;
        };
        if index == 0 {
            continue;
        }
        let not_ok = words[index].eq_ignore_ascii_case("not")
            && words
                .get(index + 1)
                .is_some_and(|word| word.eq_ignore_ascii_case("ok"));
        let status_end = index + usize::from(not_ok);
        let name = words[..index].join(" ");
        let reading = if category == "POWER" {
            words[index..].join(" ")
        } else if let Some(first) = words.get(status_end + 1) {
            if let Some(unit) = words.get(index + 2).filter(|unit| {
                matches!(
                    unit.to_ascii_lowercase().as_str(),
                    "c" | "rpm" | "watts" | "w" | "amps" | "v"
                )
            }) {
                format!("{first} {unit}")
            } else {
                (*first).to_owned()
            }
        } else {
            String::new()
        };
        sensors.push(Sensor {
            name,
            category: category.clone(),
            health: if not_ok {
                Health::Critical
            } else {
                Health::parse(words[index])
            },
            value: numeric(&reading),
            reading,
        });
    }
    sensors
}
pub fn parse_storage(text: &str, kind: &str) -> Vec<Storage> {
    let mut records = Vec::new();
    let mut current = Storage {
        kind: kind.into(),
        ..Default::default()
    };
    for line in text.lines() {
        let line = line.trim();
        if (line.starts_with('[') && line.ends_with(']'))
            || (!line.contains(char::is_whitespace) && line.contains('.') && !line.contains('='))
        {
            if !current.name.is_empty() {
                records.push(current);
            }
            if records.len() >= 64 {
                return records;
            }
            current = Storage {
                name: line.trim_matches(['[', ']']).into(),
                kind: kind.into(),
                ..Default::default()
            };
        } else if let Some((field, value)) = line.split_once('=') {
            let field = field.trim().to_ascii_lowercase();
            let value = value.trim();
            match field.as_str() {
                "fqdd" => current.name = value.into(),
                "name" => current.name = value.into(),
                "status" | "rollupstatus" => {
                    current.health = current.health.merge(Health::parse(value))
                }
                "state" => {
                    current.state = value.into();
                    current.health = current.health.merge(Health::parse(value));
                }
                "failurepredicted"
                    if value.eq_ignore_ascii_case("yes") || value.eq_ignore_ascii_case("true") =>
                {
                    current.health = current.health.merge(Health::Warning)
                }
                _ => {}
            }
        }
    }
    if !current.name.is_empty() && records.len() < 64 {
        records.push(current);
    }
    records
}
fn valid_output(text: String) -> Result<String, String> {
    if text.lines().any(|line| {
        line.trim_start().starts_with("ERROR:")
            || line.contains("COMMAND NOT RECOGNIZED")
            || line.contains("Invalid subcommand")
    }) {
        Err(text
            .lines()
            .find(|line| {
                line.contains("ERROR")
                    || line.contains("RECOGNIZED")
                    || line.contains("Invalid subcommand")
            })
            .unwrap_or("RACADM query unavailable")
            .chars()
            .take(200)
            .collect())
    } else {
        Ok(text)
    }
}
pub fn collect(
    server: &Server,
    config: &Config,
    cancel: &AtomicBool,
    root: Option<&Path>,
) -> Result<Hardware, String> {
    let query = |command: &str| {
        if cancel.load(Ordering::Relaxed) {
            return Err("iDRAC collection cancelled".into());
        }
        valid_output(crate::monitoring_probe::ssh_output(
            server, config, cancel, root, command,
        )?)
    };
    let sensors = query("racadm getsensorinfo")?;
    let mut hardware = Hardware {
        sensors: parse_sensors(&sensors),
        ..Default::default()
    };
    if hardware.sensors.is_empty() {
        return Err("iDRAC returned no readable sensors. Check the RACADM login privileges and firmware support.".into());
    }
    match query("racadm getsysinfo") {
        Ok(text) => {
            for line in text.lines() {
                if let Some((field, value)) = line.split_once('=') {
                    match field.trim().to_ascii_lowercase().as_str() {
                        "system model" => hardware.model = value.trim().into(),
                        "power status" => hardware.power_state = value.trim().into(),
                        _ => {}
                    }
                }
            }
        }
        Err(error) => hardware.notes.push(format!("System details: {error}")),
    }
    for (command, kind) in [
        (
            "racadm storage get controllers -o -p Name,Status,RollupStatus",
            "Controller",
        ),
        (
            "racadm storage get pdisks -o -p Name,Status,State,FailurePredicted",
            "Physical disk",
        ),
        (
            "racadm storage get vdisks -o -p Name,Status,State",
            "Virtual disk",
        ),
    ] {
        match query(command) {
            Ok(text) => hardware.storage.extend(parse_storage(&text, kind)),
            Err(error) => hardware.notes.push(format!("{kind}: {error}")),
        }
    }
    hardware.summarize();
    Ok(hardware)
}

#[cfg(test)]
mod tests {
    #[test]
    fn firmware_power_supply_presence_unknown_sensors_and_unbracketed_storage_are_retained() {
        let sensors = super::parse_sensors(
            "Sensor Type : POWER\nPS1 Status Present AC 6.750Watts\nPS2 Status Present AC 8.250Watts\nSensor Type : TEMPERATURE\nCPU1 Temp Unknown NA NA NA\nSensor Type : CURRENT\nSystem Board Pwr Consumption Ok 0Watts NA NA\nSensor Type : FAN\nFan1 Not Ok 0 RPM\n",
        );
        assert_eq!(sensors.len(), 5);
        assert_eq!(sensors[0].health, super::Health::Unknown);
        let mut hardware = super::Hardware {
            sensors,
            ..Default::default()
        };
        hardware.summarize();
        assert_eq!(hardware.power_watts, Some(0.0));
        assert_eq!(hardware.health, super::Health::Critical);
        let storage = super::parse_storage(
            "AHCI.Slot.3-1\nStatus = Unknown\nName = BOSS-S1\nRAID.Integrated.1-1\nStatus = Ok\nName = PERC H740P\n",
            "Controller",
        );
        assert_eq!(storage.len(), 2);
        assert_eq!(storage[0].name, "BOSS-S1");
        assert_eq!(storage[1].health, super::Health::Ok);
    }
    #[test]
    fn sensors_preserve_names_units_and_hardware_failures() {
        let sensors = super::parse_sensors(
            "Sensor Type : TEMPERATURE\nSystem Board Inlet Temp Ok 23 C 0 45\nCPU1 Temp Warning 88C 0 90\nSensor Type : FAN\nFan1A Ok 5160 RPM\nFan2 Failed 0 RPM\nSensor Type : POWER\nSystem Board Pwr Consumption Ok 207 Watts\n",
        );
        assert_eq!(sensors.len(), 5);
        let mut hardware = super::Hardware {
            sensors,
            ..Default::default()
        };
        hardware.summarize();
        assert_eq!(hardware.health, super::Health::Critical);
        assert_eq!(hardware.peak_temperature, Some(88.0));
        assert_eq!(hardware.power_watts, Some(207.0));
        assert_eq!(hardware.minimum_fan_rpm, Some(0.0));
        hardware.compact();
        assert!(hardware.sensors.is_empty());
        assert_eq!(hardware.power_watts, Some(207.0));
    }
    #[test]
    fn storage_status_and_partial_readings_are_not_reported_as_healthy() {
        let records = super::parse_storage(
            "[InstanceID: Disk.Bay.0]\nName = SSD\nStatus = Ok\nState = Online\n[InstanceID: Disk.Bay.1]\nStatus = Critical\nState = Failed\n",
            "Physical disk",
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[1].health, super::Health::Critical);
        assert_eq!(
            super::Health::parse("Not Available"),
            super::Health::Unknown
        );
        assert!(super::valid_output("ERROR: insufficient privileges".into()).is_err());
    }
}
