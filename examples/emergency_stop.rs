extern crate tello_edu;

use tello_edu::{Tello, Result};

#[tokio::main]
async fn main() {
    fly().await.unwrap();
}

async fn fly() -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect().await?;

    drone.take_off().await?;
    drone.emergency_stop().await?; // warning! this will make the drone drop like a brick

    Ok(())
}