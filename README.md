# tello-edu

A library for controlling and interacting with the [Tello EDU](https://www.ryzerobotics.com/tello-edu) drone using [asynchronous Rust](https://rust-lang.github.io/async-book/) and [Tokio](https://tokio.rs).  All operations are implemented as awaitable futures, completed when the drone sends acknowledgment of the command message.

```Rust
use tello_edu::{Tello, Result};

#[tokio::main]
async fn main() {
    fly().await.unwrap();
}

async fn fly() -> Result<()> {
    // create a new drone in the `NoWifi` state 
    let drone = Tello::new();

    // wait until the host computer joins the drone's Wifi network
    // (joining the network is not automatic - how it happens is up to you)
    let drone = drone.wait_for_wifi().await?;

    // establish connection and put the drone in "command" mode
    let drone = drone.connect().await?;

    // fly!
    drone.take_off().await?;
    drone.turn_clockwise(360).await?;
    drone.land().await?;

    Ok(())
}
```

(If Python is more your thing, there is also an equivalent asynchronous Python package - [tello-asyncio](https://pypi.org/project/tello-asyncio/).) 

## Version History

**0.1.0**

Basic drone control
- UDP connection for sending commands and receiving responses (default AP mode only - you must join the drone's own WiFi network)
- take off and land
- rotate clockwise and counter-clockwise
- move up, down, left, right, forward and back
- emergency stop

**0.1.1**

- wait for WiFi on Linux as well as macOS, assume WiFi on anything else

**0.2.0**

More drone control
- set & query speed
- stop
- flip left, right, forward and back
 

