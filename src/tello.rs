use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

use crate::errors::TelloError;
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

    pub async fn wait_for_wifi(&self) -> Result<Tello<Disconnected>, TelloError>  {
        println!("[Tello] waiting for WiFi...");
        wait_for_wifi("TELLO").await?;
        Ok(Tello { state: Disconnected })
    }
}

impl Tello<Disconnected> {
    pub async fn connect(&self) -> Result<Tello<Connected>, TelloError> {
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
                    break;
                }
                Err(err) => {
                    println!("[Tello] connection attempt #{i} failed ({err}), retrying...");
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }

        let drone = Tello { state: Connected { sock } };

        println!("[Tello] putting drone in command mode...");
        drone.send("command").await?;

        println!("[Tello] CONNECTED");

        Ok(drone)
    } 
}

impl Tello<Connected> {
    pub async fn send(&self, msg:&str) -> Result<String, TelloError> {
        println!("[Tello] SEND {msg}");
        let s = &self.state.sock;
        s.send(msg.as_bytes()).await?;

        let mut buf = vec![0; 256];        
        s.recv(&mut buf).await?;
        let response = String::from_utf8(buf)?;

        println!("[Tello] RECEIVED {response}");

        Ok(response)
    }
}