use sprite_render::{ default_render, Camera };

use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
    event::{ Event, WindowEvent, KeyboardInput, ElementState, MouseButton  },
    dpi::{ LogicalSize, PhysicalPosition }
};

mod time;
use time::Instant;

mod game;
use game::Game;

fn main() {
    let events_loop = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title("Hello world!")
        .with_inner_size(LogicalSize::new(1280.0*0.8, 720.0*0.8));
    
    // create the SpriteRender
    let (window, mut render) = default_render(wb, &events_loop, true);
    let pipe_texture = {
        let image = image::load_from_memory(
            include_bytes!(concat!(env!("OUT_DIR"), "/atlas.png"))
        ).expect("File not Found!").to_rgba();

        render.load_texture(image.width(), image.height(), image.into_raw().as_slice(), true)
    };
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
                        render.resize(size.width, size.height);
                        camera.resize(size);
                        game.resize(size.width as f32, size.height as f32);
                        // game.update(0.0, &game::Input::default());
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
                render.render()
                    .clear_screen(&[0.0f32, 0.3, 0.0, 1.0])
                    .draw_sprites(&mut camera, &game.get_sprites())
                    .finish();
            }
            _ => ()
        }
    });
}