use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelloError {
    #[error("{msg}")]
	Generic { msg: String },

    #[error("WiFi not connected")]
	WiFiNotConnected
}