use std::str;
use glutin::{ PossiblyCurrent, WindowedContext };
use crate::event_loop::EventLoop;

const WINDOW_TITLE : &str = "GIF Displayer (Esc key to exit)";

pub struct Window {
    /// Context used by the rendering logic to bind to this window
    pub windowed_context: WindowedContext<PossiblyCurrent>,

    /// initial width of the image.
    /// The is the ideal size the window should have to display the image at its
    /// initial size.
    pub base_width : u16,

    /// initial height of the image.
    /// The is the ideal size the window should have to display the image at its
    /// initial size.
    pub base_height : u16,
}

impl Window {
    /// Create new empty window.
    /// The image_width and image_height given should be the height and width of
    /// the image. The window will be initially created with those dimensions.
    /// This window won't display anything by itself, you will need to use a
    /// separate rendering logic to do that.
    pub fn new(event_loop : &EventLoop, image_width : u16, image_height : u16) -> Window {
        let window = create_window(image_width as f32, image_height as f32, event_loop);
        Window {
            windowed_context : window,
            base_width : image_width,
            base_height : image_height,
        }
    }

    /// Visually refresh the window's content
    pub fn refresh(&self) {
        self.windowed_context.swap_buffers().unwrap_or_else(| e | {
            eprintln!("Could not refresh the window due to an error: {}", e);
            std::process::exit(1);
        });
    }
}

/// Actually create the Window's context thanks to the `glutin` crate with the
/// width and height given.
fn create_window(
    width : f32,
    height : f32,
    event_loop : &EventLoop
) -> WindowedContext<glutin::PossiblyCurrent> {
    let wb = glutin::window::WindowBuilder::new()
        .with_title(WINDOW_TITLE)
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
