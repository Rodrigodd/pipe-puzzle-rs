use std::io::Cursor;

use audio_engine::{AudioEngine, OggDecoder, WavDecoder};
use sprite_render::{default_render, Camera, SpriteRender};

use winit::{
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

use rand::Rng;

mod time;
use time::Instant;

mod game;
use game::Game;

mod audio_effect;

fn audio_engine() -> &'static AudioEngine {
    use std::sync::Once;
    static mut AUDIO_ENGINE: Option<AudioEngine> = None;
    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe { AUDIO_ENGINE = Some(AudioEngine::new().unwrap()) });
    unsafe { AUDIO_ENGINE.as_ref().unwrap() }
}

fn main() {
    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(768.0, 553.0))
        .with_visible(false);
    
    #[cfg(target_arch = "wasm32")]
    let wb = {
        use winit::platform::web::WindowBuilderExtWebSys;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("main_canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();
        
        wb.with_canvas(Some(canvas))
    };

    // create the SpriteRender
    let (window, render) = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "opengl")] {
                sprite_render::GLSpriteRender::new(wb, &event_loop, true)
            } else if #[cfg(feature = "webgl")] {
                sprite_render::WebGLSpriteRender::new(wb, &event_loop)
            } else {
                (wb.build(&event_loop).unwrap(), sprite_render::EmptySpriteRender)
            }
        }
    };
    
    let music = OggDecoder::new(Cursor::new(&include_bytes!("../res/sound/pipe.ogg")[..]));
    let music = audio_effect::SlowDown::new(music);
    let slow_down_ref = music.slow_down.clone();
    let music = audio_effect::WithIntro::new(
        WavDecoder::new(Cursor::new(&include_bytes!("../res/sound/pipe-intro.wav")[..])),
        music,
    );

    let mut music = audio_engine().new_sound(music).unwrap();
    music.play();

    use rand::SeedableRng;
    let camera = Camera::new(window.inner_size(), 2.2);
    let mut game = Game::new(
        rand::rngs::SmallRng::seed_from_u64(
            time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        ),
        music,
        slow_down_ref,
        camera,
        render,
    );

    let mut clock = Instant::now();
    let mut frame_count = 0;

    let mut cursor = PhysicalPosition::new(0.0, 0.0);

    let mut input = game::Input::default();
    window.set_visible(true);
    game.resize(window.inner_size());
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, window_id } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => *control_flow = winit::event_loop::ControlFlow::Exit,
                WindowEvent::MouseInput { button, state, .. } => {
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
                }
                WindowEvent::CursorMoved { position, .. } => {
                    cursor = position;
                    input.mouse_x = cursor.x as f32;
                    input.mouse_y = cursor.y as f32;
                }
                WindowEvent::Resized(size) => {
                    game.resize(size);
                }
                _ => (),
            },

            Event::MainEventsCleared => {
                game.update(1.0 / 60.0, &input);
                #[cfg(target_arch = "wasm32")]
                {
                    unsafe {
                        // Wasm is single-thread, so this never will be a poblem (hopefully)
                        &mut *(audio_engine() as *const _ as *mut AudioEngine)
                    }
                    .update();
                }
                input.update();
                window.request_redraw();
            }

            Event::RedrawRequested(window_id) if window_id == window.id() => {
                // draw
                frame_count += 1;
                if frame_count % 60 == 0 {
                    let elapsed = clock.elapsed().as_secs_f32();
                    clock = Instant::now();
                    window.set_title(&format!("PipeMania | {:9.2} FPS", 60.0 / elapsed));
                }
                game.render();
            }
            _ => (),
        }
    });
}
