use std::io::Cursor;

use sprite_render::{ default_render, Camera, SpriteRender };
use audio_engine::{ AudioEngine, OggDecoder };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, ElementState, MouseButton  },
    dpi::{ LogicalSize, PhysicalSize, PhysicalPosition }
};

use rand::Rng;

mod time;
use time::Instant;

mod game;
use game::Game;

fn audio_engine() -> &'static AudioEngine {
    use std::sync::Once;
    static mut AUDIO_ENGINE: Option<AudioEngine> = None;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        AUDIO_ENGINE = Some(AudioEngine::new().unwrap())
    });
    unsafe {
        AUDIO_ENGINE.as_ref().unwrap()
    }
}

fn resize<R: SpriteRender + ?Sized, T: Rng>(size: PhysicalSize<u32>,render: &mut R, camera: &mut Camera, game: &mut Game<T>) {
    render.resize(size.width, size.height);
    camera.resize(size);
    let prop = size.width as f32 / size.height as f32;
    if prop > 1.0 { // landscape
        if prop  > 1280.0/720.0 {
            camera.set_position(0.0, 0.0);
        } else if prop > 1280.0/720.0/2.0 + 0.5 {
            camera.set_position(- camera.width() as f32 / 2.0 + 1.1*1280.0/720.0, 0.0);
        } else {
            camera.set_position(camera.width() as f32 / 2.0 - 1.1, 0.0);
        }
    } else { // portrait
        if prop < 720.0/1280.0 {
            camera.set_position(0.0, 0.0);
        } else if prop < 1.0/(1280.0/720.0/2.0 + 0.5) {
            camera.set_position(0.0, camera.height() as f32 / 2.0 - 1.1*1280.0/720.0);
        } else {
            camera.set_position(0.0, - camera.height() as f32 / 2.0 + 1.1);
        }
    }
    game.resize(camera.width(), camera.height());
    game.update(0.0, &game::Input::default());
}


fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(768.0, 553.0));
    
    // create the SpriteRender
    let (window, mut render) = default_render(wb, &events_loop, true);
    let pipe_texture = {
        let image = image::load_from_memory(
            include_bytes!(concat!(env!("OUT_DIR"), "/atlas.png"))
        ).unwrap().to_rgba();

        render.load_texture(image.width(), image.height(), image.into_raw().as_slice(), true)
    };

    let mut music = audio_engine().new_sound(
            OggDecoder::new(Cursor::new(&include_bytes!("../res/sound/pipe.ogg")[..]))
        ).unwrap();
    music.set_loop(true);
    music.play();

    use rand::SeedableRng;
    let mut game = Game::new(
        pipe_texture,
        rand::rngs::SmallRng::seed_from_u64(
            time::SystemTime::now().duration_since(time::UNIX_EPOCH).unwrap().as_millis() as u64
        )
    );
    let mut camera = Camera::new(window.inner_size(), 2.2);
    camera.set_position(0.0, 0.0);

    let mut clock = Instant::now();
    let mut frame_count = 0;

    game.resize(1280.0, 720.0);

    let mut cursor = PhysicalPosition::new(0.0, 0.0);

    let mut input = game::Input::default();
    
    resize(window.inner_size(), render.as_mut(), &mut camera, &mut game);
    events_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                    WindowEvent::MouseInput { button, state, ..} => {
                        if button == MouseButton::Left {
                            input.mouse_left_state = match state {
                                ElementState::Pressed => 1,
                                ElementState::Released => 3,
                            };
                        } else if button == MouseButton::Right {
                            input.mouse_rigth_state = match state {
                                ElementState::Pressed => 1,
                                ElementState::Released => 3,
                            };
                        }
                    },
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor = position;
                        let (x,y) = camera.position_to_word_space(cursor.x as f32, cursor.y as f32);
                        input.mouse_x = x;
                        input.mouse_y = y;
                    }
                    WindowEvent::Resized(size) => {
                        resize(size, render.as_mut(), &mut camera, &mut game);
                    }
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                game.update(1.0/60.0, &input);
                #[cfg(target_arch = "wasm32")] {
                    unsafe { // Wasm is single-thread, so this never will be a poblem (hopefully)
                        &mut *(audio_engine() as *const _ as *mut AudioEngine)
                    }.update();
                }
                input.update();
                window.request_redraw();
            }
            
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // draw
                frame_count +=1;
                if frame_count % 60 == 0 {
                    let elapsed = clock.elapsed().as_secs_f32();
                    clock = Instant::now();
                    window.set_title(&format!("PipeMania | {:9.2} FPS",
                        60.0/elapsed)
                    );
                }
                render.render()
                    .clear_screen(&[0.0f32, 0.3, 0.0, 1.0])
                    .draw_sprites(&mut camera, &game.get_sprites())
                    .finish();
            }
            _ => ()
        }
    });
}
