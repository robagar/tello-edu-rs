use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("WiFi not connected")]
	WiFiNotConnected
}