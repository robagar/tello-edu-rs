use tokio::sync::mpsc;

#[derive(Debug)]
pub enum TelloCommand {
    TakeOff,
    Land,
    StopAndHover,
    EmergencyStop,
    RemoteControl { left_right: i8, forwards_backwards: i8, up_down: i8, yaw: i8 },
    FlipLeft,
    FlipRight,
    FlipForward,
    FlipBack
}


pub type TelloCommandSender = mpsc::UnboundedSender<TelloCommand>;
pub type TelloCommandReceiver = mpsc::UnboundedReceiver<TelloCommand>;

pub fn make_tello_command_channel() -> (TelloCommandSender, TelloCommandReceiver) {
    mpsc::unbounded_channel()
}

