# gif-renderer #################################################################

This is a minimal GIF (pronounced "jif"!) file decoder written in Rust,
compatible to both 87a and 89a GIF versions.

Written for educational purposes, this tool should work as expected on Linux,
Windows and macOS. Other platforms might be supported.

It both contains:

- a `src/main.rs` entry point allowing it to run natively.

  Here it displays a GIF in a native window built thanks to the low-level crates:
  - [`winit`](https://github.com/rust-windowing/winit) for window creation and
    event handing (resizes, keys...)
  - [`pixels`](https://github.com/parasyte/pixels) to "draw" the gif on screen
    in a pixel-based frame buffer.

  To compile it for a native target, just use the usual
  [`cargo`](https://doc.rust-lang.org/cargo/) commands.
  Then run the produced binary with the path to your gif as an argument.

- and a `src/lib.rs` to allow importing it as a crate/library. WebAssembly
  targets are also supported.

A demo of it running on the web thanks to WebAssembly is available here:
https://peaberberian.github.io/gif-renderer.rs/

This demo allows to play/pause a gif, navigate frame-by-frame, reverse playback
and export each frame as a standalone png. More features may be coming if I feel
like it (or if you propose it)!


## Is it finished? Can I use this? #############################################

Yes!

The huge majority of valid GIF files will be fully-rendered instantly.
Rarely encountered features like interlacing or the "restore to previous"
disposal method should be well managed.

--

The only feature which has not been implemented is the "Plain Text Extension".
When encountered, this extension is just ignored.
This is because this feature is very (VERY) rarely used and is pretty hard to
implement from scratch.
As I encountered no GIF besides Plain Text Extension demos, with that feature,
and that not even web browsers seem to handle it well, I figured it was the
sensible thing to do to ignore it.
