use std::io::Cursor;

use audio_engine::{AudioEngine, OggDecoder};
use sprite_render::Camera;

use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

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
        .with_inner_size(LogicalSize::new(768.0f32, 553.0))
        .with_visible(false);

    #[cfg(target_arch = "wasm32")]
    let wb = {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;

        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("main_canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .map_err(|_| ())
            .unwrap();

        wb.with_canvas(Some(canvas))
    };

    let window = wb.build(&event_loop).unwrap();

    // create the SpriteRender
    let render = {
        cfg_if::cfg_if! {
            if #[cfg(feature = "opengl")] {
                sprite_render::GLSpriteRender::new(&window, true).unwrap()
            } else if #[cfg(feature = "webgl")] {
                sprite_render::WebGLSpriteRender::new(&window)
            } else {
                ()
            }
        }
    };

    let music = OggDecoder::new(Cursor::new(&include_bytes!("../res/sound/pipe.ogg")[..])).unwrap();
    let music = audio_effect::SlowDown::new(music);
    let slow_down_ref = music.slow_down.clone();
    let music = audio_effect::WithIntro::new(
        OggDecoder::new(Cursor::new(
            &include_bytes!("../res/sound/pipe-intro.ogg")[..],
        ))
        .unwrap(),
        music,
    );

    let mut music = audio_engine().new_sound(music).unwrap();
    music.play();

    use rand::SeedableRng;
    let size = window.inner_size();
    let camera = Camera::new(size.width, size.height, 2.2);
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

    let mut input = game::Input::default();
    window.set_visible(true);
    game.resize(window.inner_size(), window.id());
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
                    input.mouse_x = position.x as f32;
                    input.mouse_y = position.y as f32;
                }
                WindowEvent::Resized(size) => {
                    game.resize(size, window_id);
                }
                _ => (),
            },

            Event::MainEventsCleared => {
                game.update(1.0 / 60.0, &input);
                #[cfg(target_arch = "wasm32")]
                {
                    audio_engine().resume();
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
                game.render(window_id);
            }
            _ => (),
        }
    });
}
