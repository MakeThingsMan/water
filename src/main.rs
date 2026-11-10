use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{PhysicalKey};
use winit::window::{Window, WindowId};

use crate::state::State;

mod texture;
mod model;
mod resources;
mod state;
mod helpers;


// State can't exist before its window, and the window now has to be created from inside the
// running loop, so the handler holds an Option that gets filled in resumed.
#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Shader Playground")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 540.0));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        // ponytail: desktop only calls resumed once; Android can call it again, guard with self.state.is_some() if you ever target it
        self.state = Some(pollster::block_on(State::new(window)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if window_id == state.window.id() {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(phsyical_size) => state.resize(phsyical_size),
                WindowEvent::RedrawRequested => {
                    state.window().request_redraw();
                    match state.surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame) => state.render(frame),
                        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                            state.render(frame);
                            // I'm assuming right now that this is the thing that gets called whenever you resize a frame,
                            state.resize(state.size);
                        }
                        wgpu::CurrentSurfaceTexture::Outdated => state.resize(state.size),
                        wgpu::CurrentSurfaceTexture::Lost => event_loop.exit(),
                        other => eprintln!("{:?}", other),
                    }
                },
                WindowEvent::KeyboardInput {
                     event: KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state:key_state,
                        ..
                     }, 
                     ..} => state.handle_key(event_loop, code, key_state.is_pressed()),
                _ => {}
            }
        }
    }
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}