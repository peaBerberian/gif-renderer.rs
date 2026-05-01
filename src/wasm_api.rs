use crate::{decode_gif_bytes, DecodedGif};
use std::sync::Mutex;

static DECODED_GIFS: Mutex<Vec<Option<DecodedGif>>> = Mutex::new(Vec::new());
static LAST_ERROR: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn set_last_error(message: String) {
    let mut last_error = LAST_ERROR.lock().expect("last error mutex poisoned");
    *last_error = message.into_bytes();
}

fn clear_last_error() {
    let mut last_error = LAST_ERROR.lock().expect("last error mutex poisoned");
    last_error.clear();
}

fn store_gif(decoded: DecodedGif) -> u32 {
    let mut gifs = DECODED_GIFS.lock().expect("decoded gifs mutex poisoned");
    if let Some((index, slot)) = gifs.iter_mut().enumerate().find(|(_, slot)| slot.is_none()) {
        *slot = Some(decoded);
        return (index as u32) + 1;
    }
    gifs.push(Some(decoded));
    gifs.len() as u32
}

fn with_gif<T>(handle: u32, f: impl FnOnce(&DecodedGif) -> T) -> Option<T> {
    let gifs = DECODED_GIFS.lock().expect("decoded gifs mutex poisoned");
    let index = handle.checked_sub(1)? as usize;
    gifs.get(index)?.as_ref().map(f)
}

#[no_mangle]
pub extern "C" fn gif_alloc_input_buffer(len: usize) -> *mut u8 {
    let mut buf = vec![0_u8; len];
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn gif_free_input_buffer(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(ptr, len, len));
    }
}

#[no_mangle]
pub extern "C" fn gif_decode_from_ptr(ptr: *const u8, len: usize) -> u32 {
    if ptr.is_null() {
        set_last_error("Input pointer was null.".to_owned());
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
    match decode_gif_bytes(bytes) {
        Ok(decoded) => {
            clear_last_error();
            store_gif(decoded)
        }
        Err(err) => {
            set_last_error(err.to_string());
            0
        }
    }
}

#[no_mangle]
pub extern "C" fn gif_free_decoded(handle: u32) -> bool {
    let Some(index) = handle.checked_sub(1).map(|x| x as usize) else {
        return false;
    };
    let mut gifs = DECODED_GIFS.lock().expect("decoded gifs mutex poisoned");
    let Some(slot) = gifs.get_mut(index) else {
        return false;
    };
    slot.take().is_some()
}

#[no_mangle]
pub extern "C" fn gif_get_width(handle: u32) -> u32 {
    with_gif(handle, |gif| gif.width as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn gif_get_height(handle: u32) -> u32 {
    with_gif(handle, |gif| gif.height as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn gif_get_frame_count(handle: u32) -> u32 {
    with_gif(handle, |gif| gif.frames.len() as u32).unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn gif_get_loop_count(handle: u32) -> i32 {
    with_gif(handle, |gif| gif.loop_count.map(|x| x as i32).unwrap_or(-1)).unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn gif_get_frame_delay_cs(handle: u32, frame_index: u32) -> i32 {
    with_gif(handle, |gif| {
        gif.frames
            .get(frame_index as usize)
            .and_then(|frame| frame.delay_cs)
            .map(|x| x as i32)
            .unwrap_or(-1)
    })
    .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn gif_get_frame_rgba_ptr(handle: u32, frame_index: u32) -> *const u8 {
    with_gif(handle, |gif| {
        gif.frames
            .get(frame_index as usize)
            .map(|frame| frame.pixels_rgba.as_ptr())
            .unwrap_or(std::ptr::null())
    })
    .unwrap_or(std::ptr::null())
}

#[no_mangle]
pub extern "C" fn gif_get_frame_rgba_len(handle: u32, frame_index: u32) -> usize {
    with_gif(handle, |gif| {
        gif.frames
            .get(frame_index as usize)
            .map(|frame| frame.pixels_rgba.len())
            .unwrap_or(0)
    })
    .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn gif_get_last_error_ptr() -> *const u8 {
    let last_error = LAST_ERROR.lock().expect("last error mutex poisoned");
    last_error.as_ptr()
}

#[no_mangle]
pub extern "C" fn gif_get_last_error_len() -> usize {
    let last_error = LAST_ERROR.lock().expect("last error mutex poisoned");
    last_error.len()
}
