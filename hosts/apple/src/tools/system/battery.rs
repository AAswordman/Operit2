#[derive(Default)]
struct BatterySnapshot {
    level: Option<i32>,
    charging: Option<bool>,
}

impl BatterySnapshot {
    fn observe(&mut self, event: &serde_json::Value) {
        if event["domain"] != "host" || event["platform"] != "ios" {
            return;
        }
        let (level, charging) = match event["topic"].as_str() {
            Some("system.battery.low" | "system.battery.okay") => ("level", "charging"),
            Some("system.power.connected" | "system.power.disconnected") => {
                ("batteryLevel", "connected")
            }
            _ => return,
        };
        self.level = event["payload"][level]
            .as_f64()
            .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
            .map(|value| value.round() as i32);
        self.charging = event["payload"][charging].as_bool();
    }
    fn value(&self) -> (i32, bool) {
        match (self.level, self.charging) {
            (Some(level), Some(charging)) => (level, charging),
            _ => (-1, false),
        }
    }
}

#[cfg(target_os = "ios")]
static IOS_BATTERY: std::sync::Mutex<BatterySnapshot> = std::sync::Mutex::new(BatterySnapshot {
    level: None,
    charging: None,
});

#[cfg(target_os = "ios")]
pub(super) fn observe_ios_event(encoded: &str) {
    if let Ok(event) = serde_json::from_str(encoded) {
        if let Ok(mut snapshot) = IOS_BATTERY.lock() {
            snapshot.observe(&event);
        }
    }
}

pub(super) fn current_battery() -> (i32, bool) {
    #[cfg(target_os = "ios")]
    {
        IOS_BATTERY
            .lock()
            .map(|snapshot| snapshot.value())
            .unwrap_or((-1, false))
    }
    #[cfg(target_os = "macos")]
    {
        super::run_command_output("/usr/bin/pmset", &["-g", "batt"])
            .map(|output| parse_pmset(&output))
            .unwrap_or((-1, false))
    }
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    {
        (-1, false)
    }
}

#[cfg(any(target_os = "macos", test))]
fn parse_pmset(output: &str) -> (i32, bool) {
    for line in output.lines() {
        let Some((prefix, state)) = line.split_once('%') else {
            continue;
        };
        let Some(level) = prefix
            .split_whitespace()
            .last()
            .and_then(|text| text.parse::<i32>().ok())
            .filter(|value| (0..=100).contains(value))
        else {
            continue;
        };
        let status = state
            .split(';')
            .map(str::trim)
            .find(|value| !value.is_empty())
            .unwrap_or("");
        return (
            level,
            matches!(status, "charging" | "charged" | "finishing charge"),
        );
    }
    (-1, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ios_levels_update_from_owner_power_events_without_waiting_for_low_threshold() {
        let mut snapshot = BatterySnapshot::default();
        assert_eq!(snapshot.value(), (-1, false));
        for (level, charging) in [(83.0, true), (72.0, false), (0.0, false)] {
            snapshot.observe(&serde_json::json!({"domain":"host", "platform":"ios", "topic":"system.power.connected", "payload":{"batteryLevel":level,"connected":charging}}));
            assert_eq!(snapshot.value(), (level as i32, charging));
        }
        snapshot.observe(&serde_json::json!({"domain":"host", "platform":"ios", "topic":"system.battery.okay", "payload":{"level":null,"charging":null}}));
        assert_eq!(snapshot.value(), (-1, false));
    }
    #[test]
    fn macos_parses_charge_state_and_keeps_absent_batteries_unknown() {
        for (status, expected) in [
            ("charging", true),
            ("charged", true),
            ("discharging", false),
            ("not charging", false),
        ] {
            assert_eq!(parse_pmset(&format!("Now drawing from AC Power\n -InternalBattery-0 (id=1)\t81%; {status}; 1:00 remaining")), (81, expected));
        }
        assert_eq!(parse_pmset("Now drawing from AC Power"), (-1, false));
        assert_eq!(parse_pmset("Battery 101%; charging;"), (-1, false));
    }
}
