extern crate tello_edu;

use tello_edu::{TelloOptions, Tello, Result, VIDEO_WIDTH, VIDEO_HEIGHT, TelloVideoReceiver};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;

use openh264::decoder::Decoder;
use openh264::formats::YUVSource;

fn main() {
    let mut options = TelloOptions::default();

    // we want video...
    let video_receiver = options.with_video();

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

    // ...because SDL must run on the main thread
    run_gui(video_receiver).unwrap();
}

fn run_gui(mut video_receiver:TelloVideoReceiver) -> std::result::Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("tello-edu", VIDEO_WIDTH, VIDEO_HEIGHT)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::IYUV, VIDEO_WIDTH, VIDEO_HEIGHT)
        .map_err(|e| e.to_string())?;

    canvas.clear();
    canvas.present();

    let mut event_pump = sdl_context.event_pump()?;

    let mut video_channel_open = true;
    let mut decoder = Decoder::new().unwrap();

    'running: loop {
        // SDL event loop
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        if video_channel_open {
            // wait for next encoded frame of video
            match video_receiver.blocking_recv() {
                Some(frame) => {
                    // decode h264 to YUV
                    match decoder.decode(&frame.data) {
                        Ok(Some(f)) =>  {
                            // draw frame
                            texture.update_yuv(
                                None,
                                f.y(), f.y_stride() as usize,
                                f.u(), f.u_stride() as usize,
                                f.v(), f.v_stride() as usize
                            ).expect("failed to update texture");

                            canvas.copy(&texture, None, Some(Rect::new(0, 0, VIDEO_WIDTH, VIDEO_HEIGHT)))?;
                            canvas.present();
                        }
                        Ok(None) => {
                            println!("incomplete frame, dropped");
                        }
                        Err(err) => {
                            println!("h264 decoder error: {err})");
                        }
                    }
                }
                None => {
                    println!("VIDEO END");
                    video_channel_open = false;
                }
            }
        }
    }

    Ok(())
}

async fn fly(options:TelloOptions) -> Result<()> {
    let drone = Tello::new()
        .wait_for_wifi().await?;

    let drone = drone.connect_with(options).await?;

    drone.start_video().await?;

    drone.take_off().await?;
    drone.turn_clockwise(360).await?;
    drone.land().await?;

    Ok(())
}