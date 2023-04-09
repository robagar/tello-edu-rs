extern crate tello_tokio;

use tello_tokio::wifi;

#[tokio::main]
async fn main() {
	println!("Connecting to WiFi...");
    wifi::connect().await.unwrap();
}