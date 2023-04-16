use thiserror::Error;

pub type Result<T> = std::result::Result<T, TelloError>;

#[derive(Error, Debug)]
pub enum TelloError {
    #[error("{msg}")]
	Generic { msg: String },

    #[error("WiFi not connected")]
	WiFiNotConnected,

    #[error("IO error: {msg}")]
	IOError { msg: String },

	#[error("Failed to decode data from the drone: {msg} ")]
	DecodeError { msg: String },

	#[error("Failed to parse data from the drone: {msg} ")]
	ParseError { msg: String },

	#[error("Expected response \"ok\", but received \"{response}\"")]
	NotOkResponse { response: String },

	#[error("Value out of range")]
	OutOfRange,

	#[error("Non-specific error response")]
	NonSpecificError
}

impl From<std::io::Error> for TelloError {
	fn from(err: std::io::Error) -> TelloError {
		TelloError::IOError { msg: err.to_string() }
	}
}

impl From<std::string::FromUtf8Error> for TelloError {
	fn from(err: std::string::FromUtf8Error) -> TelloError {
		TelloError::DecodeError { msg: err.to_string() }
	}
}

impl TelloError {
	pub fn from_not_ok_response(response: String) -> TelloError {
		match response.as_str() {
			"error" => TelloError::NonSpecificError,
			"out of range" => TelloError::OutOfRange,
			_ => TelloError::NotOkResponse { response }
		}
	}
}