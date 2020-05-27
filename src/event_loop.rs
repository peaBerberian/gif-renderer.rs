use std::time;
use glutin::{
    event::{ Event, VirtualKeyCode::Escape, WindowEvent },
    event_loop::{
        ControlFlow,
        EventLoop as GlutinEventLoop,
        EventLoopProxy as GlutinEventLoopProxy
    },
};
use crate::open_gl::GlRenderer;

#[derive(Debug)]
pub enum GifEvent {
    /// Information about the number of time the GIF image should loop is
    /// available.
    /// `None` for no looping, `Some(0)` for infinite looping.
    /// Any other `Some(n)` indicates that all GIF frames should be displayed
    /// `n` times.
    LoopingInfo(Option<u16>),

    /// Information about the next frame is available
    GifFrameData { data : Vec<u32>, delay_until_next : Option<u16> },

    /// All frames have been communicated
    GifFrameEnd,
}
pub type EventLoopProxy = GlutinEventLoopProxy<GifEvent>;

/// Abstraction over Glutin's EventLoop allowing to display decoded GIF frames
/// at the wanted interval while handling events from the outside world.
pub struct EventLoop {
    event_loop : GlutinEventLoop<GifEvent>,
}

impl EventLoop {
    /// Create a new event loop.
    /// This event loop won't run until you call the `run` function.
    pub fn new() -> EventLoop {
        let event_loop : GlutinEventLoop<GifEvent> = GlutinEventLoop::with_user_event();
        EventLoop { event_loop }
    }

    /// Create an `EventLoopProxy`, which will allow to send GIF frame data -
    /// even from another thread - through its `send_event` method.
    pub fn create_proxy(&self) -> EventLoopProxy {
        self.event_loop.create_proxy()
    }

    pub fn glutin_event_loop(&self) -> &GlutinEventLoop<GifEvent> {
        &self.event_loop
    }

    /// Run and consume the EventLoop so that it can display GIF frames at the
    /// right time while reacting to user keyboard events and window manager
    /// events.
    ///
    /// Please note that this method will run indefinitely until certain events
    /// are received. To be able to run logic concurrently while this method is
    /// running, you will need to spawn another thread.
    /// Even then, you can still interact with the event_loop by using the
    /// `EventLoopProxy` created by the `create_proxy` method.
    pub fn run(self, renderer : GlRenderer) {
        const WAIT_TIME : time::Duration = time::Duration::from_millis(10);

        let mut last_rendering_time : time::Instant = time::Instant::now();

        // Store every frames and the corresponding delays to the next frame, if one.
        // This will be needed if the GIF has to loop
        let mut frames : Vec<(Vec<u32>, Option<u16>)> = vec![];
        let mut current_delay : Option<u16> = Some(0);
        let mut curr_frame_idx = 0;
        let mut no_more_frame = false;
        let mut loop_left : Option<u16> = None;

        self.event_loop.run(move |ev, _, control_flow| {
            *control_flow = ControlFlow::WaitUntil(
                time::Instant::now() + WAIT_TIME);

            match ev {
                Event::LoopDestroyed => return,
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                        return;
                    },
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(Escape) = input.virtual_keycode {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    },
                    WindowEvent::Resized(size) => {
                        unsafe {
                            renderer.resize(size.width, size.height);
                        }
                    },
                    _ => return,
                },
                Event::RedrawRequested {..} => {
                    if curr_frame_idx < frames.len() {
                        unsafe { renderer.redraw(); }
                    }
                }
                Event::UserEvent(ev) => {
                    match ev {
                        GifEvent::GifFrameData { data, delay_until_next } => {
                            frames.push((data, delay_until_next));
                        },
                        GifEvent::LoopingInfo(looping_info) => {
                            loop_left = looping_info;
                        }
                        GifEvent::GifFrameEnd => no_more_frame = true,
                    }
                }
                _ => (),
            }

            let now = time::Instant::now();
            match current_delay {
                None => {},
                Some(delay) => {
                    if frames.is_empty() {
                        return;
                    }
                    let delay_dur = time::Duration::from_millis(10 * delay as u64);
                    if now - last_rendering_time >= delay_dur {
                        if curr_frame_idx < frames.len() {
                            unsafe { renderer.draw(&frames[curr_frame_idx].0); }
                            current_delay = frames[curr_frame_idx].1;
                            curr_frame_idx += 1;
                            last_rendering_time = now;
                        } else if no_more_frame {
                            match loop_left {
                                None => {
                                    // *control_flow = ControlFlow::Exit;
                                    return;
                                },
                                Some(x) => {
                                    match x {
                                        0 => { /* Infinite looping, do nothing. */ },
                                        1 => { loop_left = None; }
                                        x => { loop_left = Some(x - 1); }
                                    };
                                    unsafe { renderer.draw(&frames[0].0); }
                                    current_delay = frames[0].1;
                                    curr_frame_idx = 1;
                                    last_rendering_time = now;
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
