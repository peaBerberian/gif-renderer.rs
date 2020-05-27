use std::str;
use glutin::{ PossiblyCurrent, WindowedContext };
use crate::event_loop::EventLoop;

const WINDOW_TITLE : &str = "GIF Displayer (Esc key to exit)";

pub struct Window {
    pub windowed_context: WindowedContext<PossiblyCurrent>,
    pub base_width : u16,
    pub base_height : u16,
}

impl Window {
    /// Create new empty window with the dimensions given.
    /// This window won't display anything by itself, you will need to use a
    /// separate rendering logic to do that.
    pub fn new(event_loop : &EventLoop, width : u16, height : u16) -> Window {
        let window = create_window(width as f32, height as f32, &event_loop);
        Window { windowed_context : window, base_width : width, base_height : height }
    }

    // /// Change the dimensions of the window
    // pub fn update_window_size(&self, width : u32, height : u32) {
    //     let window = self.windowed_context.window();
    //     let current_size = window.inner_size();
    //     if current_size.width == width && current_size.height == height {
    //         return;
    //     }
    //     let size = glutin::dpi::LogicalSize::new(width as f32, height as f32);
    //     window.set_inner_size(size);
    // }

    // pub fn get_inner_size(&self) -> glutin::dpi::PhysicalSize<u32> {
    //     self.windowed_context.window().inner_size()
    // }

    /// Refresh the window's content
    pub fn refresh(&self) {
        self.windowed_context.swap_buffers().unwrap_or_else(| e | {
            eprintln!("Could not refresh the window due to an error: {}", e);
            std::process::exit(1);
        });
    }
}

fn create_window(
    width : f32,
    height : f32,
    event_loop : &EventLoop
) -> WindowedContext<glutin::PossiblyCurrent> {
    let wb = glutin::window::WindowBuilder::new()
        .with_title(WINDOW_TITLE)
        .with_resizable(false)
        .with_inner_size(glutin::dpi::LogicalSize::new(width, height));

    let windowed_context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(wb, event_loop.glutin_event_loop())
        .expect("Could not build the window.");

    unsafe {
        windowed_context
            .make_current()
            .expect("Failed to make current context.")
    }
}
