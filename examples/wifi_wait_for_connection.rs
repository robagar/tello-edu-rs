extern crate tello_tokio;

use tello_tokio::Tello;

#[tokio::main]
async fn main() {
	let drone = Tello::new();
	println!("Created drone: {drone:?}");

    let drone = drone.wait_for_wifi().await.unwrap();
    println!("WiFi available, drone is now: {drone:?}");

    drone.connect().await.unwrap();
}