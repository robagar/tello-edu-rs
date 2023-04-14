use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

use crate::errors::{Result, TelloError};
use crate::wifi::wait_for_wifi;

const DEFAULT_DRONE_HOST:&str = "192.168.10.1";

const CONTROL_UDP_PORT:i32 = 8889;
// const STATE_UDP_PORT = 8890

// states
#[derive(Debug)]
pub struct NoWifi;

#[derive(Debug)]
pub struct Disconnected;

#[derive(Debug)]
pub struct Connected {
    sock: UdpSocket,
}

// #[derive(Debug)]
// pub struct Flying;

#[derive(Debug)]
pub struct Tello<S = NoWifi> {

    state: S
}

impl Tello<NoWifi> {
    pub fn new() -> Self {
        Self { state: NoWifi }
    }

    pub async fn wait_for_wifi(&self) -> Result<Tello<Disconnected>>  {
        println!("[Tello] waiting for WiFi...");
        wait_for_wifi("TELLO").await?;
        Ok(Tello { state: Disconnected })
    }
}

impl Tello<Disconnected> {
    pub async fn connect(&self) -> Result<Tello<Connected>> {
        let local_address = format!("0.0.0.0:{CONTROL_UDP_PORT}");

        let drone_host = DEFAULT_DRONE_HOST;
        let drone_address = format!("{drone_host}:{CONTROL_UDP_PORT}");

        println!("[Tello] CONNECT {local_address} → {drone_address}");

        // bind local socket
        println!("[Tello] binding local {local_address}...");
        let sock = UdpSocket::bind(&local_address).await?;
        
        // connect to drone
        println!("[Tello] connecting to drone at {drone_address}...");
        let mut i = 0;
        loop {
            i = i + 1;
            match sock.connect(&drone_address).await {
                Ok(_) => {
                    println!("[Tello] CONNECTED");
                    break;
                }
                Err(err) => {
                    println!("[Tello] connection attempt #{i} failed ({err}), retrying...");
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }

        let drone = Tello { state: Connected { sock } };

        // tell drone to expect text SDK commands (not the private binary protocol)
        println!("[Tello] putting drone in command mode...");
        drone.send_expect_ok("command").await?;

        // check battery
        let b = drone.query_battery().await?;
        if b < 10 {
            println!("[Tello] WARNING low battery: {b}%");
        }
        else {
            println!("[Tello] battery: {b}%");  
        }

        Ok(drone)
    } 
}

impl Tello<Connected> {
    pub fn disconnect(&self) -> Tello<Disconnected> {
        println!("[Tello] DISCONNECT");
        Tello { state: Disconnected }
    }

    pub async fn send(&self, msg:&str) -> Result<String> {
        println!("[Tello] SEND {msg}");

        let s = &self.state.sock;
        s.send(msg.as_bytes()).await?;

        let mut buf = vec![0; 256];        

        let n = s.recv(&mut buf).await?;

        buf.truncate(n);

        let r = String::from_utf8(buf)?;

        let response = r.trim().to_string();

        println!("[Tello] RECEIVED {response}");

        Ok(response)
    }

    pub async fn send_expect_ok(&self, msg:&str) -> Result<()> {
        match self.send(msg).await {
            Ok(response) => {
                if response == "ok" {
                    Ok(())
                }
                else {
                    Err(TelloError::NotOkResponse { response })
                }
            }
            Err(err) => Err(err)
        }
    }

    pub async fn query_battery(&self) -> Result<u8> {
        let response = self.send("battery?").await?;
        let battery = response.parse::<u8>()?;
        Ok(battery)
    }

    pub async fn take_off(&self) -> Result<()> {
        self.send_expect_ok("takeoff").await
    }

    pub async fn land(&self) -> Result<()> {
        self.send_expect_ok("land").await
    }
}