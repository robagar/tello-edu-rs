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

    drone.flip_left().await?;
    drone.flip_right().await?;

    drone.flip_forward().await?;
    drone.flip_back().await?;
    
    drone.land().await?;

    Ok(())
}