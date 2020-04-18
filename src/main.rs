mod color;
mod decoder;
mod error;
mod gif_reader;
mod parser;
mod render;

use gif_reader::GifReader;

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
    let mut rdr = GifReader::new(std::io::BufReader::new(f));
    if let Err(x) = parser::decode_and_render(&mut rdr) {
        eprintln!("Error: {}", x);
        std::process::exit(1);
    }
    std::process::exit(0);
}
