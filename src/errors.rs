use thiserror::Error;

#[derive(Error, Debug)]
pub enum TelloError {
    #[error("{msg}")]
	Generic { msg: String },

    #[error("WiFi not connected")]
	WiFiNotConnected,

    #[error("IO error: {msg}")]
	IOError { msg: String },

	#[error("Failed to decode response from drone: {msg} ")]
	DecodeResponseError {msg: String}
}

impl From<std::io::Error> for TelloError {
	fn from(err: std::io::Error) -> TelloError {
		TelloError::IOError { msg: err.to_string() }
	}
}

impl From<std::string::FromUtf8Error> for TelloError {
	fn from(err: std::string::FromUtf8Error) -> TelloError {
		TelloError::DecodeResponseError { msg: err.to_string() }
	}
}