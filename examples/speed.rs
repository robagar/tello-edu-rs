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

    drone.set_speed(25).await?;
    drone.move_forward(300).await?;

    drone.turn_clockwise(180).await?;

    drone.set_speed(100).await?;
    drone.move_forward(300).await?;

    drone.turn_clockwise(180).await?;

    drone.land().await?;

    Ok(())
}