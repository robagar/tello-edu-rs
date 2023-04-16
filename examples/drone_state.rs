extern crate tello_edu;

use tello_edu::{TelloOptions, Tello, Result};

#[tokio::main]
async fn main() {
    fly().await.unwrap();
}

async fn fly() -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let mut options = TelloOptions::default();
    let mut state_rx = options.with_state();

    tokio::spawn(async move {
        loop {
            let state = state_rx.recv().await.unwrap();
            println!("STATE {state:#?}");
        }
    });

    let drone = drone.connect_with(&options).await?;

    drone.take_off().await?;
    drone.turn_clockwise(360).await?;
    drone.land().await?;

    Ok(())
}