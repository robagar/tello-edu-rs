
use crate::TelloError; 



pub async fn connect() -> Result<(), TelloError> {
    macos::connect().await
}

mod macos {
    use std::process::Command;
    use crate::TelloError; 

    fn list_devices() -> Vec<String> {
        let raw_output = Command::new("networksetup")
            .arg("-listallhardwareports")
            .output()
            .expect("failed to run \"networksetup -listallhardwareports\"");

        let output = String::from_utf8(raw_output.stdout).unwrap();

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

        devices
    }

    pub async fn connect() -> Result<(), TelloError> {
        let devices = list_devices();

        for d in devices.iter() {
            println!("{d}");
        }

        Err(TelloError::WiFiNotConnected) 

    }
}