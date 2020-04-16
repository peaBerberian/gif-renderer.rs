use minifb::{Window, WindowOptions};

pub fn render_image(
    frames : &[(Vec<u32>, Option<u16>)],
    nb_loop : Option<u16>,
    image_width : usize,
    image_height : usize
) {
    let mut window = Window::new(
        "GIF Displayer",
        image_width as usize,
        image_height as usize,
        WindowOptions::default(),
    ).unwrap_or_else(|e| { panic!("{}", e); });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    let nb_images = frames.len();
    if nb_images == 0 {
        return;
    }
    let mut img_idx : usize = 0;
    let mut curr_loop : u16 = 1;

    while window.is_open() {
        let (curr_image, curr_delay) = &frames[img_idx];
        window
            .update_with_buffer(curr_image, image_width as usize, image_height as usize)
            .unwrap();

        match curr_delay {
            Some(delay) => std::thread::sleep(std::time::Duration::from_millis(10 * (*delay) as u64)),
            None => {},
        }

        if img_idx != nb_images -1 {
            img_idx += 1;
        } else {
            if let Some(x) = nb_loop {
                if x == 0 {
                    img_idx = 0;
                } else if curr_loop < x {
                    img_idx = 0;
                    curr_loop += 1;
                }
            }
        }
    }
}
