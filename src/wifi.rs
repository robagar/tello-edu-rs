    use std::process::Command;

use crate::TelloError; 



pub async fn wait_for_wifi(ssid_prefix: &str) -> Result<(), TelloError> {
    macos::wait_for_wifi(ssid_prefix).await
}

mod macos {
    use tokio::time::{sleep, Duration};
    use crate::TelloError;
    use super::run_command; 

    fn list_devices() -> Result<Vec<String>, TelloError> {
        let output = run_command("networksetup", &["-listallhardwareports"])?;

        let mut found_wifi = false;
        let mut devices:Vec<String> = vec![];
         for l in output.lines() {
            if !found_wifi {
                // looking for something like "Hardware Port: Wi-Fi"...
                if l.contains("Wi-Fi") {
                    found_wifi = true;
                }
            }
            else {
                // ...then next like should be like "Device: en1"
                found_wifi = false;
                devices.push(l.trim_start_matches("Device: ").to_string());
            }
        }

        Ok(devices)
    }

    pub async fn wait_for_wifi(ssid_prefix: &str) -> Result<(), TelloError> {
        let devices = list_devices()?;


        // wait for any one of them to connect
        let waiting_for = format!("Current Wi-Fi Network: {ssid_prefix}");
        loop {
            for device in devices.iter() {
                let s = run_command("networksetup", &["-getairportnetwork", device])?;
                if s.starts_with(&waiting_for) {
                    return Ok(())
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}



fn run_command(cmd:&str, args: &[&str]) -> Result<String, TelloError> {
    let raw_output = Command::new(cmd)
        .args(args)
        .output()
        .expect("failed to run {cmd}");

    String::from_utf8(raw_output.stdout).map_err(
        |e|  TelloError::Generic { msg: format!("failed to decode {cmd} output - {e:?}") }
    )
}
