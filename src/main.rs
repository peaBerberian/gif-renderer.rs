use gif_renderer::{
    decode_gif_with_header, frames_store::FramesStore, parse_header, DecodedGif, GifHeader,
    GifParsingError, GifReader,
};
use pixels::{Error as PixelsError, Pixels, SurfaceTexture};
use std::sync::{
    mpsc::{channel, Receiver},
    Arc,
};
use std::time::Instant;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes, WindowId},
};

const WINDOW_TITLE: &str = "GIF Displayer";

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Error: Missing file path in argument.");
        std::process::exit(1);
    }

    let file = std::fs::File::open(&args[1]).unwrap_or_else(|err| {
        eprintln!("Error: Error while opening {}: {}", &args[1], err);
        std::process::exit(1);
    });
    let mut reader = GifReader::new(std::io::BufReader::new(file));
    let header = parse_header(&mut reader).unwrap_or_else(|err| {
        eprintln!("Error while parsing the GIF header: {}", err);
        std::process::exit(1);
    });

    let (tx, rx) = channel::<Result<DecodedGif, GifParsingError>>();
    let decode_header = header.clone();
    std::thread::spawn(move || {
        let result = decode_gif_with_header(&mut reader, decode_header);
        let _ = tx.send(result);
    });

    NativeGifApp::run(header, rx).unwrap_or_else(|err| {
        eprintln!("Error while running the native app: {}", err);
        std::process::exit(1);
    });
}

struct NativeGifApp {
    header: GifHeader,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    pixels: Option<Pixels<'static>>,
    surface_size: PhysicalSize<u32>,
    frames: FramesStore<Vec<u8>>,
    current_frame: Option<Vec<u8>>,
    decoder_receiver: Receiver<Result<DecodedGif, GifParsingError>>,
    decode_status: DecodeStatus,
    next_redraw_at: Option<Instant>,
}

enum DecodeStatus {
    Loading,
    Ready,
    Error(String),
}

impl NativeGifApp {
    fn run(
        header: GifHeader,
        decoder_receiver: Receiver<Result<DecodedGif, GifParsingError>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let event_loop = EventLoop::new()?;
        let initial_surface_size = PhysicalSize::new(header.width as u32, header.height as u32);
        let mut app = Self {
            header,
            window: None,
            window_id: None,
            pixels: None,
            surface_size: initial_surface_size,
            frames: FramesStore::new(),
            current_frame: None,
            decoder_receiver,
            decode_status: DecodeStatus::Loading,
            next_redraw_at: Some(Instant::now()),
        };
        event_loop.run_app(&mut app)?;
        Ok(())
    }

    fn window_attributes(&self) -> WindowAttributes {
        Window::default_attributes()
            .with_title(window_title(&self.decode_status, self.header.width, self.header.height))
            .with_inner_size(LogicalSize::new(
                self.header.width as f64,
                self.header.height as f64,
            ))
            .with_min_inner_size(LogicalSize::new(1.0, 1.0))
    }

    fn apply_decoded_gif(&mut self, decoded: DecodedGif) {
        self.frames.set_loop_iterations(decoded.loop_count);
        for frame in decoded.frames {
            self.frames.add_frame(frame.pixels_rgba, frame.delay_cs);
        }
        self.frames.end_of_frames();
        self.decode_status = DecodeStatus::Ready;
        self.update_window_title();
    }

    fn update_window_title(&self) {
        if let Some(window) = &self.window {
            window.set_title(&window_title(
                &self.decode_status,
                self.header.width,
                self.header.height,
            ));
        }
    }

    fn poll_decoder(&mut self) {
        if let Ok(result) = self.decoder_receiver.try_recv() {
            match result {
                Ok(decoded) => self.apply_decoded_gif(decoded),
                Err(err) => {
                    self.decode_status = DecodeStatus::Error(err.to_string());
                    self.update_window_title();
                }
            }
        }
    }

    fn tick(&mut self) -> Option<std::time::Duration> {
        if !matches!(
            self.decode_status,
            DecodeStatus::Loading | DecodeStatus::Ready
        ) {
            return None;
        }

        let frame_change = self.frames.check();
        let next_recheck = frame_change.delay_before_recheck();
        if let Some(new_frame) = frame_change.into_frame_data() {
            self.current_frame = Some(new_frame.clone());
        }
        next_recheck
    }

    fn redraw(&mut self) -> Result<(), PixelsError> {
        let Some(pixels) = self.pixels.as_mut() else {
            return Ok(());
        };

        let frame = pixels.frame_mut();
        frame.fill(0);
        if let Some(current_frame) = &self.current_frame {
            draw_fitted_frame(
                frame,
                self.surface_size.width as usize,
                self.surface_size.height as usize,
                current_frame,
                self.header.width as usize,
                self.header.height as usize,
            );
        }
        pixels.render()
    }

    fn resize_surface(&mut self, size: PhysicalSize<u32>) -> Result<(), PixelsError> {
        self.surface_size = size;
        if let Some(pixels) = self.pixels.as_mut() {
            pixels.resize_surface(size.width, size.height)?;
            if size.width > 0 && size.height > 0 {
                pixels.resize_buffer(size.width, size.height)?;
            }
        }
        Ok(())
    }
}

impl ApplicationHandler for NativeGifApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(self.window_attributes())
                .expect("failed to create window"),
        );
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, Arc::clone(&window));
        let pixels = Pixels::new(size.width, size.height, surface_texture)
            .expect("failed to create pixel surface");

        self.window_id = Some(window.id());
        self.pixels = Some(pixels);
        self.surface_size = size;
        self.window = Some(window);
        self.next_redraw_at = Some(Instant::now());
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Err(err) = self.resize_surface(size) {
                    eprintln!("Error while resizing pixel surface: {}", err);
                    event_loop.exit();
                    return;
                }
                self.next_redraw_at = Some(Instant::now());
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(size) = self.window.as_ref().map(|window| window.inner_size()) {
                    if let Err(err) = self.resize_surface(size) {
                        eprintln!("Error while resizing pixel surface after scale change: {}", err);
                        event_loop.exit();
                        return;
                    }
                }
                self.next_redraw_at = Some(Instant::now());
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.poll_decoder();
                let next_redraw = self.tick();
                if let Err(err) = self.redraw() {
                    eprintln!("Error while rendering frame: {}", err);
                    event_loop.exit();
                    return;
                }
                self.next_redraw_at = next_redraw.map(|delay| Instant::now() + delay);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(window) = &self.window else {
            return;
        };

        match self.next_redraw_at {
            None => event_loop.set_control_flow(ControlFlow::Wait),
            Some(when) => {
                let now = Instant::now();
                if now >= when {
                    window.request_redraw();
                    event_loop.set_control_flow(ControlFlow::Poll);
                } else {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(when));
                }
            }
        }
    }
}

fn window_title(status: &DecodeStatus, width: u16, height: u16) -> String {
    match status {
        DecodeStatus::Loading => format!("{WINDOW_TITLE} - Decoding... ({width}x{height})"),
        DecodeStatus::Ready => format!("{WINDOW_TITLE} ({width}x{height})"),
        DecodeStatus::Error(err) => format!("{WINDOW_TITLE} - Error: {err}"),
    }
}

fn draw_fitted_frame(
    target: &mut [u8],
    target_width: usize,
    target_height: usize,
    source: &[u8],
    source_width: usize,
    source_height: usize,
) {
    if target_width == 0 || target_height == 0 || source_width == 0 || source_height == 0 {
        return;
    }

    let scale = f32::min(
        target_width as f32 / source_width as f32,
        target_height as f32 / source_height as f32,
    );
    let draw_width = (source_width as f32 * scale).floor().max(1.0) as usize;
    let draw_height = (source_height as f32 * scale).floor().max(1.0) as usize;
    let offset_x = (target_width.saturating_sub(draw_width)) / 2;
    let offset_y = (target_height.saturating_sub(draw_height)) / 2;

    for y in 0..draw_height {
        let src_y = y * source_height / draw_height;
        for x in 0..draw_width {
            let src_x = x * source_width / draw_width;

            let src_index = (src_y * source_width + src_x) * 4;
            let dst_index = ((offset_y + y) * target_width + (offset_x + x)) * 4;
            target[dst_index..dst_index + 4].copy_from_slice(&source[src_index..src_index + 4]);
        }
    }
}
