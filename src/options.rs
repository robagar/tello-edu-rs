use crate::state::*;

/// Tello drone connection and other usage options.
#[derive(Default)]
pub struct TelloOptions {
    pub state_sender: Option<TelloStateSender>
}

impl TelloOptions {
    /// Request state udpates from the drone.
    ///
    /// *nb* As messages are sent to the UDP broadcast address 0.0.0.0 this 
    /// only works in AP mode, ie using the drone's own WiFi network
    ///
    /// Returns the receiver end of the channel used to pass on updates
    ///  
    pub fn with_state(&mut self) -> TelloStateReceiver  {
        let (tx, rx) = make_tello_state_channel();
        self.state_sender = Some(tx);
        rx
    }
}