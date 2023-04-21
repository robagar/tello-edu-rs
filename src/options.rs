use crate::state::*;
use crate::video::*;

/// Tello drone connection and other usage options.
#[derive(Default)]
pub struct TelloOptions {
    pub(crate) state_sender: Option<TelloStateSender>,
    pub(crate) video_sender: Option<TelloVideoSender>
}

impl TelloOptions {
    /// Request state updates from the drone.
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

    /// Request video from the drone as a stream of h264-encoded 720p YUV 
    /// frames.
    ///
    /// *nb* As messages are sent to the UDP broadcast address 0.0.0.0 this 
    /// only works in AP mode, ie using the drone's own WiFi network
    ///
    /// Returns the receiver end of the channel used to pass on frames
    ///  
    pub fn with_video(&mut self) -> TelloVideoReceiver  {
        let (tx, rx) = make_tello_video_channel();
        self.video_sender = Some(tx);
        rx
    }

}