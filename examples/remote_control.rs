

fn main() {

    run_control().expect("failed to run control");
}

fn run_control() -> Result<(), String> {
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
        // use sdl2::controller::Axis;
        use sdl2::event::Event;

        match event {
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

const DEAD_ZONE:i16 = 1000;
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

