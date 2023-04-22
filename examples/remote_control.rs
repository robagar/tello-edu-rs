//////////////////////////////////////////////////////////////////////////////
//
// Remote control usiing a game controller
//
// Developed & tested with a bluetooth XBox One controller (highly recommended)
//
//  Controls: 
//  - Start (the one above the right stick): take off
//  - Left Stick: go forwards/backwards and turn left/right
//  - Right Stick: up/down and strafe left/right
//  - D-Pad: flip
//  - Left Shoulder + Right Shoulder: emergency stop
//
// Note that the Tello drone will automatically land if forced down to ~20cm 
// above a surface
// 
//////////////////////////////////////////////////////////////////////////////

extern crate tello_edu;

use sdl2::controller::Axis;
use sdl2::controller::Button;
use sdl2::event::Event;


use tello_edu::{TelloOptions, Tello, Result, TelloCommandSender, TelloCommand};


fn main() {
    let mut options = TelloOptions::default();

    // we want to send commands...
    let command_sender = options.with_command();

    // run async Tokio runtime in a thread...
    std::thread::spawn(move || {
        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        tokio_runtime.block_on(async {
            fly(options).await.unwrap();
        });
    });

    run_control(command_sender).expect("failed to run control");
}

fn run_control(command_sender: TelloCommandSender) -> anyhow::Result<(),  String> {
    // This is required for certain controllers to work on Windows without the
    // video subsystem enabled:
    sdl2::hint::set("SDL_JOYSTICK_THREAD", "1");

    let sdl_context = sdl2::init()?;
    let game_controller_subsystem = sdl_context.game_controller()?;

    let num_controllers = game_controller_subsystem
        .num_joysticks()
        .map_err(|e| format!("can't enumerate controllers: {}", e))?;


    if num_controllers == 0 {
        return Err("no game controllers found".to_string());
    }

    // use the first one available
    let controller = (0..num_controllers)
        .find_map(|id| {
            if !game_controller_subsystem.is_game_controller(id) {
                println!("{} is not a game controller", id);
                return None;
            }

            match game_controller_subsystem.open(id) {
                Ok(c) => {
                    println!("using controller \"{}\"", c.name());
                    Some(c)
                }
                Err(e) => {
                    println!("failed to open controller {id}: {e:?}");
                    None
                }
            }
        })
        .expect("failed to use any controller");

    // remote control state
    let mut left_right:i8 = 0;
    let mut forwards_backwards:i8 = 0;
    let mut up_down:i8 = 0;
    let mut yaw:i8 = 0;

    for event in sdl_context.event_pump()?.wait_iter() {

        match event {
            // both shoulder buttons together to immediately stop motors (and drop like a brick!)
            Event::ControllerButtonDown { button: Button::LeftShoulder, .. } 
            | Event::ControllerButtonDown { button: Button::RightShoulder, .. } => {
                if controller.button(Button::LeftShoulder) && controller.button(Button::RightShoulder) {
                    command_sender.send(TelloCommand::EmergencyStop)
                }
                else {
                    Ok(())
                }
            }

            // start button to take off
            Event::ControllerButtonUp { button: Button::Start, .. } => {
                command_sender.send(TelloCommand::TakeOff)
            }

            // X to land
            Event::ControllerButtonDown { button: Button::X, .. } => {
                left_right = 0;
                forwards_backwards = 0;
                up_down = 0;
                yaw = 0;
                command_sender.send(TelloCommand::Land)
            }

            // B to stop
            Event::ControllerButtonDown { button: Button::B, .. } => {
                left_right = 0;
                forwards_backwards = 0;
                up_down = 0;
                yaw = 0;
                command_sender.send(TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw} )
                // command_sender.send(TelloCommand::StopAndHover)
            }

            // left stick Y to go forwards
            Event::ControllerAxisMotion { axis: Axis::LeftY, value, .. } => {
                forwards_backwards = -remote_control_value(value);
                command_sender.send(TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw} )
            }            

            // left stick X to turn
            Event::ControllerAxisMotion { axis: Axis::LeftX, value, .. } => {
                yaw = remote_control_value(value);
                command_sender.send(TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw} )
            }            

            // right stick Y to move vertically
            Event::ControllerAxisMotion { axis: Axis::RightY, value, .. } => {
                up_down = -remote_control_value(value);
                command_sender.send(TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw} )
            }            

            // right stick X to strafe
            Event::ControllerAxisMotion { axis: Axis::RightX, value, .. } => {
                left_right = remote_control_value(value);
                command_sender.send(TelloCommand::RemoteControl { left_right, forwards_backwards, up_down, yaw} )
            }

            // D-pad to flip
            Event::ControllerButtonDown { button: Button::DPadLeft, .. } => {
                command_sender.send(TelloCommand::FlipLeft)
            }            
            Event::ControllerButtonDown { button: Button::DPadRight, .. } => {
                command_sender.send(TelloCommand::FlipRight)
            }            
            Event::ControllerButtonDown { button: Button::DPadUp, .. } => {
                command_sender.send(TelloCommand::FlipForward)
            }            
            Event::ControllerButtonDown { button: Button::DPadDown, .. } => {
                command_sender.send(TelloCommand::FlipBack)
            }            

            Event::Quit { .. } => break,
            _ => Ok(()),
        }.map_err(|err| format!("error sending command: {err}"))?;

    }

    Ok(())
}

//////////////////////////////////////////////////////////////////////////////

const DEAD_ZONE:i16 = 10;
const AXIS_MAX:f32 = 32767.0;

/// Axis value to [-1.0, 1.0]
fn normalize_axis_value(value:i16) -> Option<f32> {
    if value > DEAD_ZONE || value < -DEAD_ZONE {
        // outside dead zone
        let normalized_value = value as f32 / AXIS_MAX;
        if normalized_value > 1.0 {
            Some(1.0)
        }
        else if normalized_value < -1.0 {
            Some(-1.0)
        }
        else {
            Some(normalized_value)
        }
    }
    else {
        // in dead zone
        None
    }
}

/// Axis value to [-100,100]
fn remote_control_value(value:i16) -> i8 {
    match normalize_axis_value(value) {
        Some(v) => (v * 100.0) as i8,
        None => 0
    }
}

//////////////////////////////////////////////////////////////////////////////

async fn fly(options:TelloOptions) -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect_with(options).await?;

    drone.handle_commands().await?;

    Ok(())
}
