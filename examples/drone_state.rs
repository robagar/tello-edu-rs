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

    // we want state updates...
    let mut state_receiver = options.with_state();

    // ...so spawn task to receive them
    tokio::spawn(async move {
        loop {
            let state = state_receiver.recv().await.unwrap();
            println!("STATE {state:#?}");
        }
    });

    // connect using these options
    let drone = drone.connect_with(options).await?;

    // go!
    drone.take_off().await?;
    drone.turn_clockwise(360).await?;
    drone.land().await?;

    Ok(())
}