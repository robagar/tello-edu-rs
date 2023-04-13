extern crate tello_tokio;

use tello_tokio::{Tello, Result};

#[tokio::main]
async fn main() {
    take_off_and_land().await.unwrap();
}

async fn take_off_and_land() -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect().await?;

    drone.take_off().await?;
    drone.land().await?;

    Ok(())
}