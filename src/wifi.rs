use std::process::Command;
use tokio::time::{sleep, Duration};

use crate::{TelloError, Result}; 

//////////////////////////////////////////////////////////////////////////////
// macOS

#[cfg(target_os = "macos")]
fn list_wifi_devices() -> Result<Vec<String>> {
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

#[cfg(target_os = "macos")]
pub async fn wait_for_wifi(ssid_prefix: &str) -> Result<()> {
    let devices = list_wifi_devices()?;

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

//////////////////////////////////////////////////////////////////////////////
// linux

#[cfg(target_os = "linux")]
pub async fn wait_for_wifi(ssid_prefix: &str) -> Result<()> {
    loop {
        let s = run_command("iwgetid", &["-r"])?;
        if s.starts_with(ssid_prefix) {
            return Ok(())
        }
        sleep(Duration::from_millis(100)).await;
    }
}

//////////////////////////////////////////////////////////////////////////////
// anything else

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub async fn wait_for_wifi(ssid_prefix: &str) -> Result<()> {
    println!("[WiFi] warning - wait_for_wifi has not been implemented for this OS, assuming joined already and continuing");
    Ok(())
}

//////////////////////////////////////////////////////////////////////////////

fn run_command(cmd:&str, args: &[&str]) -> Result<String> {
    let raw_output = Command::new(cmd)
        .args(args)
        .output()
        .expect("failed to run {cmd}");

    String::from_utf8(raw_output.stdout).map_err(
        |e|  TelloError::Generic { msg: format!("failed to decode {cmd} output - {e:?}") }
    )
}
