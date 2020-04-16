// As all errors here will make us unable to do anything at all, there's no point
// in defining an Error struct, with the overhead and boilerplate code that comes
// with it.
// Just output to stderr and exit with an exit code of 1.

pub fn fail_on_truncated_block(block_name : &str) {
    eprintln!("Error: Truncated GIF file.\n \
        A \"{}\" block appear to be incomplete.", block_name);
    std::process::exit(1);
}

pub fn fail_on_no_gif_header() {
    eprintln!("Error: No \"GIF\" header found. Are you sure this is a GIF file?");
    std::process::exit(1);
}

pub fn fail_on_invalid_version(version : Option<String>) {
    match version {
        Some(version_number) =>
            eprintln!("Error: Version not recognized: {}", version_number),
        None => eprintln!("Error: Cannot read the current version."),
    }
    std::process::exit(1);
}

pub fn fail_on_block_invalid_length(block_name : &str) {
    eprintln!("Error: Invalid GIF file. A {} block has an unexpected length.", block_name);
}

pub fn fail_on_expected_block_terminator(block_name : Option<&str>) {
    match block_name {
        Some(name) =>
            eprintln!("Error: Expected a block terminator at the end of a {} block.",
                name),
        None =>
          eprintln!("Error: Expected a block terminator but found nothing"),
    }
    std::process::exit(1);
}

