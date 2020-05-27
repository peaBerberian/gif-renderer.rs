#![feature(vec_into_raw_parts)]
mod color;
mod decoder;
mod error;
mod event_loop;
mod gif_reader;
mod open_gl;
mod parser;
mod window;

use gif_reader::GifReader;

fn main() {
    // 1 - open file in argument
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Error: Missing file path in argument.");
        std::process::exit(1);
    }
    let f = std::fs::File::open(&args[1]).unwrap_or_else(|err| {
        eprintln!("Error: Error while opening {}: {}", &args[1], err);
        std::process::exit(1);
    });

    // 2 - parse GIF header to check validity and obtain dimensions
    let mut rdr = GifReader::new(std::io::BufReader::new(f));
    let header = parser::parse_header(&mut rdr).unwrap_or_else(|err| {
        eprintln!("Error while parsing the GIF header: {}", err);
        std::process::exit(1);
    });

    // 3 - Initialize window and rendering loop
    // TODO there might be too much steps here, we could reduce them as there
    // is only one type of renderer, event loop and window.
    let el = event_loop::EventLoop::new();
    let window = window::Window::new(&el, header.width, header.height);
    let renderer = open_gl::GlRenderer::new(window);
    let proxy = el.create_proxy();

    // 4 - decode GIF in another thread
    std::thread::spawn(move || {
        if let Err(x) = parser::decode(&mut rdr, &header, proxy) {
            eprintln!("Error while decoding: {}", x);
            std::process::exit(1);
        }
    });

    // 5 - run event loop
    el.run(renderer);
}
