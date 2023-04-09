

use crate::errors::Error; 



pub async fn connect() -> Result<(), Error> {
    Err(Error::WiFiNotConnected) 
}

