extern crate tello_tokio;

use tello_tokio::Tello;

#[tokio::main]
async fn main() {
    let drone = Tello::new();

    let drone = drone.wait_for_wifi().await.unwrap();

    let drone = drone.connect().await.unwrap();

    drone.take_off().await.unwrap();
    drone.land().await.unwrap();

    // drone.disconnect().await.unwrap();
}