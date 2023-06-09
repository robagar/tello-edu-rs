extern crate tello_edu;

use tello_edu::{Tello, Result};

#[tokio::main]
async fn main() {
	wifi_wait_for_connection().await.unwrap();
}

async fn wifi_wait_for_connection() -> Result<()>{
    let drone = Tello::new();
    println!("Created drone: {drone:?}");

    let drone = drone.wait_for_wifi().await?;
    println!("WiFi available, drone is now: {drone:?}");

    let drone = drone.connect().await?;
    println!("connected, drone is now: {drone:?}");

    let drone = drone.disconnect().await?;
    println!("disconnected, drone is now: {drone:?}");

    Ok(())
}