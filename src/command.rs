use tokio::sync::mpsc;

#[derive(Debug)]
pub enum TelloCommand {
    TakeOff,
    Land
}


pub type TelloCommandSender = mpsc::UnboundedSender<TelloCommand>;
pub type TelloCommandReceiver = mpsc::UnboundedReceiver<TelloCommand>;

pub fn make_tello_command_channel() -> (TelloCommandSender, TelloCommandReceiver) {
    mpsc::unbounded_channel()
}

