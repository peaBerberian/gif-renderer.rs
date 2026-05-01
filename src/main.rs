use eframe::egui::{self, ColorImage, TextureHandle, ViewportBuilder};
use gif_renderer::{
    decode_gif_with_header, frames_store::FramesStore, parse_header, DecodedGif, GifParsingError,
    GifReader,
};
use std::sync::mpsc::{channel, Receiver};

const WINDOW_TITLE: &str = "GIF Displayer (Esc key to exit)";

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

    GifRendererEframeApp::initialize(reader, header).unwrap_or_else(|err| {
        eprintln!("Error while running the native app: {}", err);
        std::process::exit(1);
    });
}

pub(crate) struct GifRendererEframeApp {
    frames: FramesStore<ColorImage>,
    texture: Option<TextureHandle>,
    width: usize,
    height: usize,
    decoder_receiver: Receiver<Result<DecodedGif, GifParsingError>>,
    decode_status: DecodeStatus,
}

enum DecodeStatus {
    Loading,
    Ready,
    Error(String),
}

impl GifRendererEframeApp {
    pub(crate) fn initialize(
        mut reader: GifReader<impl std::io::Read + std::io::Seek + Send + 'static>,
        header: gif_renderer::GifHeader,
    ) -> Result<(), eframe::Error> {
        let viewport = ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size((header.width as f32, header.height as f32));

        let options = eframe::NativeOptions {
            viewport,
            run_and_return: false,
            vsync: false,
            ..Default::default()
        };

        let (tx, rx) = channel::<Result<DecodedGif, GifParsingError>>();
        let width = header.width as usize;
        let height = header.height as usize;
        std::thread::spawn(move || {
            let result = decode_gif_with_header(&mut reader, header);
            let _ = tx.send(result);
        });

        let app = Self {
            frames: FramesStore::new(),
            texture: None,
            width,
            height,
            decoder_receiver: rx,
            decode_status: DecodeStatus::Loading,
        };

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

    fn load_decoded_gif(&mut self, decoded: DecodedGif) {
        self.frames.set_loop_iterations(decoded.loop_count);
        for frame in decoded.frames {
            let img =
                ColorImage::from_rgba_unmultiplied([self.width, self.height], &frame.pixels_rgba);
            self.frames.add_frame(img, frame.delay_cs);
        }
        self.frames.end_of_frames();
        self.decode_status = DecodeStatus::Ready;
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

        if let Ok(result) = self.decoder_receiver.try_recv() {
            match result {
                Ok(decoded) => self.load_decoded_gif(decoded),
                Err(err) => {
                    self.decode_status = DecodeStatus::Error(err.to_string());
                }
            }
        }

        if matches!(
            self.decode_status,
            DecodeStatus::Loading | DecodeStatus::Ready
        ) {
            let frame_change = self.frames.check();
            if let Some(delay) = frame_change.delay_before_recheck() {
                ctx.request_repaint_after(delay);
            }
            if let Some(new_frame) = frame_change.into_frame_data() {
                self.texture =
                    Some(ctx.load_texture("frame", new_frame.clone(), Default::default()));
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.add_space(5.0);
                    ui.label("Press ESC to exit");
                    ui.separator();
                    ui.label(format!("Size: {}x{}", self.width, self.height));
                    ui.add_space(5.0);
                });
                ui.add_space(3.0);

                match &self.decode_status {
                    DecodeStatus::Loading if self.texture.is_none() => {
                        ui.label("Decoding GIF...");
                    }
                    DecodeStatus::Error(err) => {
                        ui.colored_label(egui::Color32::RED, err);
                    }
                    _ => {
                        if let Some(texture) = &self.texture {
                            ui.image(texture);
                        }
                    }
                }
            });
    }
}
