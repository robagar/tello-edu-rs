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
    drone.move_up(50).await?;
    drone.move_down(50).await?;
    drone.move_left(50).await?;
    drone.move_right(50).await?;
    drone.move_forward(50).await?;
    drone.move_back(50).await?;
    drone.land().await?;

    Ok(())
}