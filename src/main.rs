mod color;
mod decoder;
mod error;
mod frames_store;
mod gif_reader;
mod parser;

use eframe::egui;
use egui::{ColorImage, TextureHandle, ViewportBuilder};
use frames_store::FramesStore;
use gif_reader::{GifRead, GifReader};
use std::sync::mpsc::{channel, Receiver};

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
    frames: FramesStore<ColorImage>,
    texture: Option<TextureHandle>,

    width: usize,
    height: usize,
    receiver: Receiver<GifEvent>,
}

impl GifRendererEframeApp {
    pub(crate) fn initialize(mut rdr: impl GifRead + Send + 'static) -> Result<(), eframe::Error> {
        let header = parser::parse_header(&mut rdr).unwrap_or_else(|err| {
            eprintln!("Error while parsing the GIF header: {}", err);
            std::process::exit(1);
        });
        let viewport = ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size((header.width as f32, header.height as f32));

        let options = eframe::NativeOptions {
            viewport,
            run_and_return: false,
            vsync: false,
            ..Default::default()
        };

        let width = header.width as usize;
        let height = header.height as usize;
        let (tx, rx) = channel::<GifEvent>();
        let app = Self {
            frames: FramesStore::new(),
            texture: None,
            width,
            height,
            receiver: rx,
        };
        // 4 - decode GIF in another thread
        std::thread::spawn(move || {
            if let Err(x) = parser::decode(&mut rdr, &header, tx) {
                eprintln!("Error while decoding: {}", x);
                std::process::exit(1);
            }
        });
        eframe::run_native(
            WINDOW_TITLE,
            options,
            Box::new(|cc| {
                cc.egui_ctx.set_style(egui::Style {
                    spacing: egui::style::Spacing {
                        window_margin: egui::Margin::ZERO,
                        ..Default::default()
                    },
                    ..Default::default()
                });
                Ok(Box::new(app))
            }),
        )
    }
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
                    // I used [u32] initially, but egui wants [u8].
                    // I could be transmuting and stuff for max efficiency, but I'm in the middle
                    // of changing the gui so I'm focusing on other things here
                    let mut data_u8 = Vec::with_capacity(data.len() * std::mem::size_of::<u32>());
                    for num in data {
                        data_u8.extend_from_slice(&num.to_ne_bytes()); // Slice is fine here
                    }
                    let img = egui::ColorImage::from_rgba_unmultiplied(
                        [self.width, self.height],
                        &data_u8,
                    );
                    self.frames.add_frame(img, duration);
                }
                GifEvent::LoopingInfo(looping_info) => {
                    self.frames.set_loop_iterations(looping_info)
                }
                GifEvent::FrameEnd => self.frames.end_of_frames(),
            }
        }

        let frame_change = self.frames.check();
        if let Some(new_frame) = frame_change.new_frame {
            self.texture = Some(ctx.load_texture("frame", new_frame, Default::default()));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE) // No margins or padding
            .show(ctx, |ui| {
                ui.add_space(5.0); // top padding
                ui.horizontal(|ui| {
                    ui.add_space(5.0); // left padding
                    ui.label("Press ESC to exit");
                    ui.separator();
                    ui.label(format!("Size: {}x{}", self.width, self.height));
                    // TODO: next and prev buttons?
                    ui.add_space(5.0); // right padding
                });
                ui.add_space(3.0); // bottom padding

                if let Some(texture) = &self.texture {
                    ui.image(texture);
                }
            });

        if let Some(delay) = frame_change.delay_before_next_frame {
            ctx.request_repaint_after(delay);
        }
    }
}
