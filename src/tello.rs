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

    /// Use this if you are already in the appropriate WiFi network. 
    pub async fn assume_wifi(&self) -> Result<Tello<Disconnected>>  {
        println!("[Tello] assuming WiFi has already been joined");
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
        let b = drone.battery().await?;
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
    /// Disconnect from the drone.
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
    pub async fn send(&self, command: &str) -> Result<String> {
        println!("[Tello] SEND {command}");

        let s = &self.state.sock;
        s.send(command.as_bytes()).await?;

        let response = self.recv().await?;

        // the drone sends "forced stop" after "stop" after a delay which may
        // arrive after more commands have been sent
        if response == "forced stop" {
            self.on_forced_stop();

            // try again
            self.recv().await
        }
        else {
            Ok(response)
        }          
    }

    async fn recv(&self) -> Result<String> {
        let s = &self.state.sock;
        let mut buf = vec![0; 256];        
        let n = s.recv(&mut buf).await?;

        buf.truncate(n);
        let r = String::from_utf8(buf)?;
        let response = r.trim().to_string();

        println!("[Tello] RECEIVED {response}");

        Ok(response)
    }

    fn on_forced_stop(&self) {
        println!("[Tello] FORCED STOP");
    }

    /// Sends a command, resolving to an error if the response is not "ok"
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect_ok(&self, command: &str) -> Result<()> {
        match self.send(command).await {
            Ok(response) => {
                if response == "ok" {
                    Ok(())
                }
                else {
                    Err(TelloError::from_not_ok_response(response))
                }
            }
            Err(err) => Err(err)
        }
    }

    /// Sends a command with a single value, resolving to an error if the 
    /// response is not "ok"
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// - `value` the value to append to the command
    /// 
    pub async fn send_value_expect_ok<T: std::fmt::Display>(&self, command: &str, value: T) -> Result<()> {
        match self.send(&format!("{command} {value}")).await {
            Ok(response) => {
                if response == "ok" {
                    Ok(())
                }
                else {
                    Err(TelloError::from_not_ok_response(response))
                }
            }
            Err(err) => Err(err)
        }
    }

    /// Sends a command, expecting no response at all from the drone.
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect_nothing(&self, command: &str) -> Result<()> {
        println!("[Tello] SEND {command}");

        let s = &self.state.sock;
        s.send(command.as_bytes()).await?;

        Ok(())
    }

    /// Sends a command, expecting a response that can be parsed as type `T` from the drone.
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect<T: std::str::FromStr>(&self, command: &str) -> Result<T> {
        let r = self.send(command).await?;
        let v = r.parse::<T>().map_err(|_| TelloError::ParseResponseError { msg: format!("unexpected response: \"{r}\"")})?;
        Ok(v)
    }

    /// The unique drone serial number.
    pub async fn serial_number(&self) -> Result<String> {
        self.send("sn?").await
    }

    /// The Tello SDK version.
    pub async fn sdk_version(&self) -> Result<String> {
        self.send("sdk?").await
    }

    /// The drone battery level as a percentage.
    pub async fn battery(&self) -> Result<u8> {
        self.send_expect::<u8>("battery?").await
    }

    /// The WiFi signal to noise ratio as a percentage.
    pub async fn wifi_signal_to_noise_ratio(&self) -> Result<u8> {
        self.send_expect::<u8>("wifi?").await
    }

    /// The flight time in seconds, requested directly from the drone.
    pub async fn flight_time(&self) -> Result<u16> {
        self.send_expect::<u16>("time?").await
    }

    /// Immediately stop all motors.
    ///
    /// warning! this will make the drone drop like a brick!
    ///
    pub async fn emergency_stop(&self) -> Result<()> {
        self.send_expect_nothing("emergency").await
    }

    /// Take off and hover.
    pub async fn take_off(&self) -> Result<()> {
        self.send_expect_ok("takeoff").await
    }

    /// Land and stop motors.
    pub async fn land(&self) -> Result<()> {
        self.send_expect_ok("land").await
    }

    /// The drone speed in cm/s, requested directly from the drone.
    pub async fn speed(&self) -> Result<f32> {
        self.send_expect::<f32>("speed?").await
    }

    /// Set the forward speed.
    /// 
    /// - `speed` Desired speed, 10-100 cm/s
    ///
    pub async fn set_speed(&self, speed: u8) -> Result<()> {
        self.send_value_expect_ok("speed", speed).await
    }

    /// Wait for the given length of time.
    ///
    /// - `duration` The time to wait
    ///
    pub async fn wait(&self, duration:Duration) -> Result<()> {
        println!("[Tello] waiting for {duration:#?}");
        sleep(duration).await;
        Ok(())
    }    

    /// Stop and hover in place.
    pub async fn stop(&self) -> Result<()> {
        // will also trigger a "forced stop" response
        self.send_expect_ok("stop").await
    }

    /// Turn clockwise.
    ///
    /// - `degrees` Angle in degrees 1-360°
    ///
    pub async fn turn_clockwise(&self, degrees: u16) -> Result<()> {
        self.send_value_expect_ok("cw", degrees).await   
    }

    /// Turn counter-clockwise.
    ///
    /// - `degrees` Angle in degrees 1-360°
    pub async fn turn_counterclockwise(&self, degrees: u16) -> Result<()> {
        self.send_value_expect_ok("ccw", degrees).await   
    }

    /// Move straight up.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_up(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("up", distance).await
    }

    /// Move straight down.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_down(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("down", distance).await
    }
    
    /// Move straight left.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_left(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("left", distance).await
    }
    
    /// Move straight right.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_right(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("right", distance).await
    }
    
    /// Move straight forwards.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_forward(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("forward", distance).await
    }
    
    /// Move straight backwards.
    ///
    /// - `distance` Distance to travel, 20-500 cm
    ///
    pub async fn move_back(&self, distance: u16) -> Result<()> {
        self.send_value_expect_ok("back", distance).await
    }

    /// Flip left.
    /// *nb* fails if battery is low 
    pub async fn flip_left(&self) -> Result<()> {
        self.send_expect_ok("flip l").await
    }        

    /// Flip right.
    /// *nb* fails if battery is low 
    pub async fn flip_right(&self) -> Result<()> {
        self.send_expect_ok("flip r").await
    }        

    /// Flip forward.
    /// *nb* fails if battery is low 
    pub async fn flip_forward(&self) -> Result<()> {
        self.send_expect_ok("flip f").await
    }        

    /// Flip back.
    /// *nb* fails if battery is low 
    pub async fn flip_back(&self) -> Result<()> {
        self.send_expect_ok("flip b").await
    }        

}