use sprite_render::{ SpriteRender, Camera };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, KeyboardInput, ElementState, MouseButton  },
    dpi::{ LogicalSize, PhysicalPosition }
};

mod game;
use game::Game;

fn main() {

    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(1280.0*0.8, 720.0*0.8));
    
    // create the SpriteRender
    let (window, mut render) = SpriteRender::new(wb, &events_loop, true);
    let pipe_texture = {
        let image = image::open(concat!(env!("OUT_DIR"), "/atlas.png")).expect("File not Found!").to_rgba();
        render.load_texture(image.width(), image.height(), image.into_raw().as_slice(), true)
    };
    
    let mut game = Game::new(pipe_texture, rand::thread_rng());
    let mut camera = Camera::new(window.inner_size(), 2.2);
    camera.set_position(0.0, 0.0);

    use std::time::{ Instant };
    let mut clock = Instant::now();
    let mut frame_count = 0;

    game.resize(1280.0, 720.0);

    let mut cursor = PhysicalPosition::new(0.0, 0.0);

    let mut input = game::Input::default();
    
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
                    WindowEvent::KeyboardInput { input: KeyboardInput {
                        virtual_keycode: Some(key),
                        state: ElementState::Pressed,
                        ..
                    }, ..} => match key {
                        _ => ()
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor = position;
                        let (x,y) = camera.position_to_word_space(cursor.x as f32, cursor.y as f32);
                        input.mouse_x = x;
                        input.mouse_y = y;
                    }
                    WindowEvent::Resized(size) => {
                        render.resize(size);
                        camera.resize(size);
                        game.resize(size.width as f32, size.height as f32);
                        game.update(0.0, &game::Input::default());
                    }
                    _ => (),
                }
            },

            Event::MainEventsCleared => {
                game.update(1.0/60.0, &input);
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
                render.draw(&mut camera, &game.get_sprites());
            }
            _ => ()
        }
    });
}