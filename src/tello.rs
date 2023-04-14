use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};

use crate::errors::{Result, TelloError};
use crate::wifi::wait_for_wifi;

const DEFAULT_DRONE_HOST:&str = "192.168.10.1";

const CONTROL_UDP_PORT:i32 = 8889;
// const STATE_UDP_PORT = 8890

/// Initial state - no WiFi network
#[derive(Debug)]
pub struct NoWifi;

/// The drone WiFi has been joined, but no UDP messages have been sent or received.
#[derive(Debug)]
pub struct Disconnected;

/// The connection exchange has been completed and the drone is ready to fly.
#[derive(Debug)]
pub struct Connected {
    sock: UdpSocket,
}

/// For interacting with the Tello EDU drone using the simple text-based UDP protocol.
///
/// The basic flow from the user's point of view is
///
///    SEND `command` → drone does something → RECEIVE `response` when it's finished
///
/// Messages are plain ASCII text, eg command `forward 10` → response `ok`
///
/// ```
/// use tello_edu::{Tello, Result};
/// 
/// #[tokio::main]
/// async fn main() {
///     fly().await.unwrap();
/// }
/// 
/// async fn fly() -> Result<()> {
///     // create a new drone in the `NoWifi` state     
///     let drone = Tello::new();
///
///     // wait until the host computer joins the drone's WiFi network
///     // (joining the network is not automatic - how it happens is up to you)
///     let drone = drone.wait_for_wifi().await?;
/// 
///     // establish connection and put the drone in "command" mode
///     let drone = drone.connect().await?;
/// 
///     // fly!
///     drone.take_off().await?;
///     drone.turn_clockwise(360).await?;
///     drone.land().await?;
/// 
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct Tello<S = NoWifi> {
    /// The connection state of the drone.
    state: S
}

impl Tello<NoWifi> {
    /// Create a new drone in a completely unconnected state.
    pub fn new() -> Self {
        Self { state: NoWifi }
    }

    /// Wait until the host joins the drone's WiFi network
    ///
    /// *nb* exactly how the the network is joined is up to you
    ///
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

    /// Sends a command to the drone using the simple Tello UDP protocol, returning the reponse.
    ///
    /// The basic flow from the user's point of view is
    ///
    ///    SEND command → drone does something → RECEIVE response when it's finished
    ///
    /// Messages are plain ASCII text, eg command `forward 10` → response `ok`
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send(&self, command:&str) -> Result<String> {
        println!("[Tello] SEND {command}");

        let s = &self.state.sock;
        s.send(command.as_bytes()).await?;

        let mut buf = vec![0; 256];        
        let n = s.recv(&mut buf).await?;

        buf.truncate(n);
        let r = String::from_utf8(buf)?;
        let response = r.trim().to_string();

        println!("[Tello] RECEIVED {response}");

        Ok(response)
    }

    /// Sends a command, resolving to an error if the response is not "ok"
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect_ok(&self, command:&str) -> Result<()> {
        match self.send(command).await {
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

    /// Sends a command, expecting no response at all from the drone.
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect_nothing(&self, command:&str) -> Result<()> {
        println!("[Tello] SEND {command}");

        let s = &self.state.sock;
        s.send(command.as_bytes()).await?;

        Ok(())
    }

    /// Queries the drone battery level as a percentage.
    pub async fn query_battery(&self) -> Result<u8> {
        let response = self.send("battery?").await?;
        let battery = response.parse::<u8>()?;
        Ok(battery)
    }

    /// Immediately stop all motors
    ///
    /// warning! this will make the drone drop like a brick
    pub async fn emergency_stop(&self) -> Result<()> {
        self.send_expect_nothing("emergency").await
    }

    /// Take off and hover.
    pub async fn take_off(&self) -> Result<()> {
        self.send_expect_ok("takeoff").await
    }

    /// Land and stop motors
    pub async fn land(&self) -> Result<()> {
        self.send_expect_ok("land").await
    }

    /// Turn clockwise.
    ///
    /// - `degrees` Angle in degrees 1-360°
    ///
    pub async fn turn_clockwise(&self, degrees: u16) -> Result<()> {
        self.send_expect_ok(&format!("cw {degrees}")).await   
    }

    /// Turn counter-clockwise.
    ///
    /// - `degrees` Angle in degrees 1-360°
    pub async fn turn_counterclockwise(&self, degrees: u16) -> Result<()> {
        self.send_expect_ok(&format!("ccw {degrees}")).await   
    }

    /// Move straight up.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_up(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("up {distance}")).await
    }

    /// Move straight down.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_down(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("down {distance}")).await
    }
    
    /// Move straight left.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_left(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("left {distance}")).await
    }
    
    /// Move straight right.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_right(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("right {distance}")).await
    }
    
    /// Move straight forwards.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_forward(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("forward {distance}")).await
    }
    
    /// Move straight backwards.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_back(&self, distance: i16) -> Result<()> {
        self.send_expect_ok(&format!("back {distance}")).await
    }

}