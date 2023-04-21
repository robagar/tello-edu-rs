extern crate tello_edu;

use tello_edu::{TelloOptions, Tello, Result};


#[tokio::main]
async fn main() {
    let mut options = TelloOptions::default();

    // we want video...
    let mut video_receiver = options.with_video();

    tokio::spawn(async move {
        loop {
            let frame = video_receiver.recv().await;
            println!("video frame: {frame:?}");
        }
    });

    fly(options).await.unwrap();

}


async fn fly(options:TelloOptions) -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect_with(&options).await?;

    drone.start_video().await?;

    drone.take_off().await?;
    drone.land().await?;

    Ok(())
}