use tokio::{spawn, task};
use tokio::sync::mpsc;
use tokio::net::UdpSocket;

use crate::errors::{Result, TelloError};

const STATE_UDP_PORT:u32 = 8890;

pub type TelloStateSender = mpsc::UnboundedSender<TelloState>;
pub type TelloStateReceiver = mpsc::UnboundedReceiver<TelloState>;

pub fn make_tello_state_channel() -> (TelloStateSender, TelloStateReceiver) {
    mpsc::unbounded_channel()
}

/// The live state of the drone.
#[derive(Debug, Default)]
pub struct TelloState {
    pub roll: i16,
    pub pitch: i16,
    pub yaw: i16,
    pub height: i16,
    pub barometer: f32,
    pub battery: u8,
    pub time_of_flight: u16,
    pub motor_time: u16,
    pub temperature_low: i16,
    pub temperature_high: i16,
    pub velocity: Vector3<i16>,
    pub acceleration: Vector3<f32>
}

#[derive(Debug, Default)]
pub struct Vector3<T> {
    x: T,
    y: T,
    z: T
}

impl TelloState {
    /// Parses a state string received from the drone.
    ///
    /// Example message:
    /// "mid:-1;x:-100;y:-100;z:-100;mpry:-1,-1,-1;pitch:0;roll:0;yaw:-3;vgx:0;vgy:0;vgz:1;templ:58;temph:60;tof:71;h:50;bat:82;baro:-57.14;time:14;agx:17.00;agy:-4.00;agz:-956.00;"
    ///
    pub fn from_message(s: &str) -> Result<TelloState> {
        let mut state = TelloState::default();

        for f in s.split(";") {
            if f.is_empty() { continue; }

            let (k,v) = split_key_value(f)?;

            match k.as_str() {
                "roll" => state.roll = value_as(&v)?,
                "pitch" => state.pitch = value_as(&v)?,
                "yaw" => state.yaw = value_as(&v)?,
                "h" => state.height = value_as(&v)?,
                "baro" => state.barometer = value_as(&v)?,
                "bat" => state.battery = value_as(&v)?,
                "tof" => state.time_of_flight = value_as(&v)?,
                "time" => state.motor_time = value_as(&v)?,
                "templ" => state.temperature_low = value_as(&v)?,
                "temph" => state.temperature_high = value_as(&v)?,
                "vgx" => state.velocity.x = value_as(&v)?,
                "vgy" => state.velocity.y = value_as(&v)?,
                "vgz" => state.velocity.z = value_as(&v)?,
                "agx" => state.acceleration.x = value_as(&v)?,
                "agy" => state.acceleration.y = value_as(&v)?,
                "agz" => state.acceleration.z = value_as(&v)?,
                _ => {}
            }
        }

        Ok(state)
    }
}

fn split_key_value(kv: &str) -> Result<(String, String)> {
    let mut i = kv.split(":");
    let k = i.next().ok_or_else(|| TelloError::ParseError { msg: kv.to_string() })?;
    let v = i.next().ok_or_else(|| TelloError::ParseError { msg: kv.to_string() })?;
    Ok((k.to_string(),v.to_string()))
}

fn value_as<T: std::str::FromStr>(s: &str) -> Result<T> {
    s.parse::<T>().map_err(|_| TelloError::ParseError { msg: s.to_string() })
}

// fn value_as_some<T: std::str::FromStr>(s: &str) -> Result<Option<T>> {
//     let v = s.parse::<T>().map_err(|_| TelloError::ParseError { msg: s.to_string() })?;
//     Ok(Some(v))
// }

#[derive(Debug)]
pub(crate) struct StateListener {
    task: task::JoinHandle<()>
}   

impl StateListener {
    pub(crate) async fn start_listening(sender:TelloStateSender) -> Result<Self> { 
        let local_address = format!("0.0.0.0:{STATE_UDP_PORT}");
        println!("[State] START LISTENING at {local_address}");

        let sock = UdpSocket::bind(&local_address).await?;

        let task = spawn(async move {
            loop {
                let s = &sock;
                let mut buf = vec![0; 1024];        
                let n = s.recv(&mut buf).await.unwrap();

                buf.truncate(n);
                let r = String::from_utf8(buf).unwrap();
                let raw_state = r.trim().to_string();

                let state = TelloState::from_message(&raw_state).unwrap();
                sender.send(state).unwrap();
            }
        });

        Ok(Self { task })
    }

    pub(crate) async fn stop_listening(&self) -> Result<()> {
        println!("[State] STOP LISTENING");
        self.task.abort();
        // TODO?
        // let _err = self.task.await;
        Ok(())
    }
 }