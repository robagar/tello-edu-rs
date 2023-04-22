extern crate tello_edu;

// use sdl2::controller::Axis;
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

fn run_control(command_sender: TelloCommandSender) -> std::result::Result<(), String> {
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
    let _controller = (0..num_controllers)
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

    for event in sdl_context.event_pump()?.wait_iter() {

        match event {
            Event::ControllerButtonUp { button: Button::Start, .. } => {
                command_sender.send(TelloCommand::TakeOff).expect("failed to send take off command");
            }


            Event::ControllerAxisMotion {
                axis, value, ..
            } => {
                if let Some(normalized_value) = normalize_axis_value(value) {
                    println!("Axis {axis:?}: {value} -> {normalized_value}");
                }
            }
            Event::ControllerButtonDown { button, .. } => println!("Button {:?} down", button),
            Event::ControllerButtonUp { button, .. } => println!("Button {:?} up", button),


            Event::Quit { .. } => break,
            _ => (),
        }
    }

    Ok(())
}

const DEAD_ZONE:i16 = 10;
const AXIS_MAX:f32 = 32767.0;

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

async fn fly(options:TelloOptions) -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect_with(options).await?;

    drone.handle_commands().await?;

    Ok(())
}
