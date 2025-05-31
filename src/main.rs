#![feature(vec_into_raw_parts)]

mod color;
mod decoder;
mod error;
mod gif_reader;
mod parser;

use std::time;

use eframe::egui;
use egui::{ColorImage, TextureHandle, ViewportBuilder};
use gif_reader::{GifRead, GifReader};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Error: Missing file path in argument.");
        std::process::exit(1);
    }
    let f = std::fs::File::open(&args[1]).unwrap_or_else(|err| {
        eprintln!("Error: Error while opening {}: {}", &args[1], err);
        std::process::exit(1);
    });

    let rdr = GifReader::new(std::io::BufReader::new(f));
    GifRendererEframeApp::initialize(rdr).unwrap();
}

const WINDOW_TITLE: &str = "GIF Displayer (Esc key to exit)";

use parser::GifEvent;

pub(crate) struct GifRendererEframeApp {
    texture: Option<TextureHandle>,

    width: usize,
    height: usize,
    receiver: std::sync::mpsc::Receiver<GifEvent>,

    // Store every frames and the corresponding delays to the next frame, if one.
    // This will be needed if the GIF has to loop
    frames: Vec<(ColorImage, Option<u16>)>,
    last_rendering_time: std::time::Instant,
    current_delay: Option<u16>,
    curr_frame_idx: usize,
    no_more_frame: bool,
    loop_left: Option<u16>,
}

impl GifRendererEframeApp {
    pub(crate) fn initialize(mut rdr: impl GifRead + Send + 'static) -> Result<(), eframe::Error> {
        let header = parser::parse_header(&mut rdr).unwrap_or_else(|err| {
            eprintln!("Error while parsing the GIF header: {}", err);
            std::process::exit(1);
        });
        let viewport = ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            // TODO:
            .with_inner_size((header.width as f32, header.height as f32));

        let options = eframe::NativeOptions {
            viewport,
            run_and_return: false,
            vsync: false,
            ..Default::default()
        };

        let width = header.width as usize;
        let height = header.height as usize;
        let (tx, rx) = std::sync::mpsc::channel::<GifEvent>();
        let app = Self {
            texture: None,
            width,
            height,
            receiver: rx,

            frames: vec![],
            last_rendering_time: time::Instant::now(),
            current_delay: Some(0),
            curr_frame_idx: 0,
            no_more_frame: false,
            loop_left: None,
        };
        // 4 - decode GIF in another thread
        std::thread::spawn(move || {
            if let Err(x) = parser::decode(&mut rdr, &header, tx) {
                eprintln!("Error while decoding: {}", x);
                std::process::exit(1);
            }
        });
        eframe::run_native(WINDOW_TITLE, options, Box::new(|_cc| Ok(Box::new(app))))
    }

    // fn resize(&mut self, new_width: usize, new_height: usize) {
    //     if new_width != self.width || new_height != self.height {
    //         self.width = new_width;
    //         self.height = new_height;
    //         self.texture = None;
    //     }
    // }
}

impl eframe::App for GifRendererEframeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                });
            }
        });

        while let Ok(event) = self.receiver.try_recv() {
            match event {
                GifEvent::Frame { data, duration } => {
                    // We used [u32] initially, but egui wants [u8].
                    // We could be transmuting and stuff for max efficiency, but I'm in the middle
                    // of changing the gui so I'm focusing on other things here
                    let mut data_u8 = Vec::with_capacity(data.len() * std::mem::size_of::<u32>());
                    for num in data {
                        data_u8.extend_from_slice(&num.to_ne_bytes()); // Slice is fine here
                    }
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [self.width, self.height],
                        &data_u8,
                    );
                    self.frames.push((img, duration));
                }
                GifEvent::LoopingInfo(looping_info) => {
                    self.loop_left = looping_info;
                }
                GifEvent::FrameEnd => self.no_more_frame = true,
            }
        }

        let now = time::Instant::now();
        // let mut curr_frame = self.texture;
        match self.current_delay {
            None => {}
            Some(delay) => {
                if !self.frames.is_empty() {
                    let delay_dur = time::Duration::from_millis(10 * delay as u64);
                    if now - self.last_rendering_time >= delay_dur {
                        if self.curr_frame_idx < self.frames.len() {
                            self.texture = Some(ctx.load_texture(
                                "frame",
                                self.frames[self.curr_frame_idx].0.clone(),
                                Default::default(),
                            ));
                            self.current_delay = self.frames[self.curr_frame_idx].1;
                            self.curr_frame_idx += 1;
                            self.last_rendering_time = now;
                        } else if self.no_more_frame {
                            match self.loop_left {
                                None => {
                                    // *control_flow = ControlFlow::Exit;
                                    return;
                                }
                                Some(x) => {
                                    match x {
                                        0 => { /* Infinite looping, do nothing. */ }
                                        1 => {
                                            self.loop_left = None;
                                        }
                                        x => {
                                            self.loop_left = Some(x - 1);
                                        }
                                    };
                                    self.texture = Some(ctx.load_texture(
                                        "frame",
                                        self.frames[0].0.clone(),
                                        Default::default(),
                                    ));
                                    self.current_delay = self.frames[0].1;
                                    self.curr_frame_idx = 1;
                                    self.last_rendering_time = now;
                                }
                            }
                        }
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Press ESC to exit");
                ui.separator();
                ui.label(format!("Size: {}x{}", self.width, self.height));
                ui.separator();
            });

            ui.separator();

            // Display the framebuffer as a texture
            if let Some(texture) = &self.texture {
                ui.image(texture);
            }

            ui.separator();
        });

        // limit to 60fps
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }
}
