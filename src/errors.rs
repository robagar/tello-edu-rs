use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelloError {
    #[error("WiFi not connected")]
	WiFiNotConnected
}