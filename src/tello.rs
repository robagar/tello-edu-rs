use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};
use tokio::sync::Mutex;

use crate::errors::{Result, TelloError};
use crate::wifi::wait_for_wifi;
use crate::state::*;
use crate::video::*;
use crate::command::*;
use crate::options::TelloOptions;

const DEFAULT_DRONE_HOST:&str = "192.168.10.1";

const CONTROL_UDP_PORT:i32 = 8889;

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
    state_listener: Option<StateListener>,
    video_listener: Option<VideoListener>,
    command_receiver: Option<Mutex<TelloCommandReceiver>>
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
    inner: S
}

impl Tello<NoWifi> {
    /// Create a new drone in a completely unconnected state.
    pub fn new() -> Self {
        Self { inner: NoWifi }
    }

    /// Wait until the host joins the drone's WiFi network
    ///
    /// *nb* exactly how the the network is joined is up to you
    ///
    pub async fn wait_for_wifi(&self) -> Result<Tello<Disconnected>>  {
        println!("[Tello] waiting for WiFi...");
        wait_for_wifi("TELLO").await?;
        Ok(Tello { inner: Disconnected })
    }

    /// Use this if you are already in the appropriate WiFi network. 
    pub async fn assume_wifi(&self) -> Result<Tello<Disconnected>>  {
        println!("[Tello] assuming WiFi has already been joined");
        Ok(Tello { inner: Disconnected })
    }    
}

impl Tello<Disconnected> {
    /// Connect to the drone using the default options, ie
    /// - using the drone's own WiFi
    /// - drone address 192.168.10.1
    /// - no state updates
    /// - no video
    pub async fn connect(&self) -> Result<Tello<Connected>> {
        self.connect_with(TelloOptions::default()).await
    }

    /// Connect to the drone using the given options
    ///
    /// - `options` Connection options
    ///
    pub async fn connect_with(&self, options:TelloOptions) -> Result<Tello<Connected>> {
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

        // connected drone, control only
        let mut drone = Tello { inner: Connected { sock, state_listener: None, video_listener: None, command_receiver: None } };

        // want drone state?
        if let Some(state_tx) = &options.state_sender {
            let state_listener = StateListener::start_listening(state_tx.clone()).await?;
            drone.inner.state_listener = Some(state_listener);
        }

        // want drone video?
        if let Some(video_tx) = &options.video_sender {
            let video_listener = VideoListener::start_listening(video_tx.clone()).await?;
            drone.inner.video_listener = Some(video_listener);
        }

        // expecting commands?
        if let Some(command_rx) = options.command_receiver {
            drone.inner.command_receiver = Some(Mutex::new(command_rx));
        }

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
    pub async fn disconnect(&self) -> Result<Tello<Disconnected>> {
        println!("[Tello] DISCONNECT");

        if let Some(state_listener) = &self.inner.state_listener {
            state_listener.stop_listening().await?;
        }

        if let Some(video_listener) = &self.inner.video_listener {
            video_listener.stop_listening().await?;
        }

        Ok(Tello { inner: Disconnected })
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

        let s = &self.inner.sock;
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
        let s = &self.inner.sock;
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

        let s = &self.inner.sock;
        s.send(command.as_bytes()).await?;

        Ok(())
    }

    /// Sends a command, expecting a response that can be parsed as type `T` from the drone.
    ///
    /// - `command` the command to send, must be a valid Tello SDK command string
    /// 
    pub async fn send_expect<T: std::str::FromStr>(&self, command: &str) -> Result<T> {
        let r = self.send(command).await?;
        let v = r.parse::<T>().map_err(|_| TelloError::ParseError { msg: format!("unexpected response: \"{r}\"")})?;
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
    ///
    /// *nb* fails if battery is low
    /// 
    pub async fn flip_left(&self) -> Result<()> {
        self.send_expect_ok("flip l").await
    }        

    /// Flip right.
    ///
    /// *nb* fails if battery is low 
    ///
    pub async fn flip_right(&self) -> Result<()> {
        self.send_expect_ok("flip r").await
    }        

    /// Flip forward.
    ///
    /// *nb* fails if battery is low 
    ///
    pub async fn flip_forward(&self) -> Result<()> {
        self.send_expect_ok("flip f").await
    }        

    /// Flip back.
    ///
    /// *nb* fails if battery is low 
    ///
    pub async fn flip_back(&self) -> Result<()> {
        self.send_expect_ok("flip b").await
    }        

    /// Start video as stream of h264-encoded frames.
    ///
    /// Use `TelloOption::with_video()` to set up a channel for receiving the
    /// video frames.
    ///
    /// *nb* You must consume the frame data! The channel is unlimited and 
    /// will eventually use up all available memory if you don't.
    ///
    pub async fn start_video(&self) -> Result<()> {
        self.send_expect_ok("streamon").await
    }        

    /// Stop video streaming.
    pub async fn stop_video(&self) -> Result<()> {
        self.send_expect_ok("streamoff").await
    }

    /// Remote control'
    ///
    /// All arguments are -100 to 100 (not sure what units)
    /// - `left_right` Movement sideways
    /// - `forwards_backwards` Forwards/backwards
    /// - `up_down` Vertical movement
    /// - `yaw` Turn left or right
    ///
    pub async fn remote_control(&self, left_right:i8, forwards_backwards:i8, up_down:i8, yaw:i8) -> Result<()> {
        self.send_expect_nothing(&format!("rc {left_right} {forwards_backwards} {up_down} {yaw}")).await
    }


    //////////////////////////////////////////////////////////////////////////

    pub async fn handle_commands(&self) -> Result<()> {
        if let Some(command_receiver) = &self.inner.command_receiver { 
            let mut command_rx = command_receiver.lock().await;
            while let Some(command) = command_rx.recv().await {
                match command {
                    TelloCommand::TakeOff => self.take_off().await?,
                    TelloCommand::Land => self.land().await?,
                    TelloCommand::StopAndHover => self.stop().await?,
                    TelloCommand::EmergencyStop => self.emergency_stop().await?,
                    TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw } => 
                        self.remote_control(left_right, forwards_backwards, up_down, yaw).await?,
                    TelloCommand::FlipLeft => self.flip_left().await?,
                    TelloCommand::FlipRight => self.flip_right().await?,
                    TelloCommand::FlipForward => self.flip_forward().await?,
                    TelloCommand::FlipBack => self.flip_back().await?
                 }
            }
        }
    
        Ok(())

    }

}