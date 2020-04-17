use std::sync::mpsc;
use minifb::{Window, WindowOptions};

fn create_window(image_width : usize, image_height : usize) -> Window {
    let mut window = Window::new(
        "GIF Renderer",
        image_width as usize,
        image_height as usize,
        WindowOptions::default(),
    ).unwrap_or_else(|e| {
        eprintln!("Error: Impossible to create a new window: {}", e);
        std::process::exit(1);
    });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));
    window
}

pub fn create_rendering_thread(
    image_width : usize,
    image_height : usize,
    frame_rx : mpsc::Receiver<Option<(Vec<u32>, Option<u16>)>>,
    nb_loop_rx : mpsc::Receiver<Option<u16>>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut window = create_window(image_width, image_height);

        // Store every frames and the corresponding delays to the next frame, if one.
        // This will be needed if the GIF has to loop
        let mut frames : Vec<(Vec<u32>, Option<u16>)> = vec![];

        while window.is_open() {
            match frame_rx.try_recv() {
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                },
                Ok(Some((buf, dispose_after))) => {
                    display_buffer(&mut window, buf.as_slice(), image_width, image_height);
                    frames.push((buf, dispose_after));
                    if let Some(delay) = dispose_after {
                        std::thread::sleep(std::time::Duration::from_millis(10 * (delay) as u64));
                    }
                }
                Ok(None) => {
                    // The end has been reached.
                    // Depending on if the GIF must loop, either do that or do nothing at all.
                    let loop_opt = nb_loop_rx.recv().unwrap_or_else(|e| {
                        eprintln!("Error: error while getting the number of loop from the main thread: {}", e);
                        std::process::exit(1);
                    });
                    match loop_opt {
                        None => {
                            while window.is_open() {}
                            break;
                        },
                        Some(nb_loop) => {
                            let mut img_idx : usize = 0;
                            let mut curr_loop : u16 = 1;
                            let nb_images = frames.len();
                            loop {
                                let (buf, dispose_after) = &frames[img_idx];
                                display_buffer(&mut window, buf.as_slice(), image_width, image_height);
                                if let Some(delay) = dispose_after {
                                    std::thread::sleep(std::time::Duration::from_millis(10 * (*delay) as u64));
                                }
                                if img_idx != nb_images -1 {
                                    img_idx += 1;
                                } else if nb_loop == 0 {
                                    img_idx = 0;
                                } else if curr_loop < nb_loop {
                                    img_idx = 0;
                                    curr_loop += 1;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    })
}

fn display_buffer(window : &mut Window, buffer : &[u32], image_width : usize, image_height : usize) {
    window
        .update_with_buffer(buffer, image_width as usize, image_height as usize)
        .unwrap_or_else(|e| {
            eprintln!("Error: Impossible to display the next frame: {}", e);
            std::process::exit(1);
        });
}
