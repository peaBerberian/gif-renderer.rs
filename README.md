# gif-renderer #################################################################

This is a GIF file decoder written in Rust, compatible to both 87a and 89a GIF
versions.

This tool should work as expected on Linux, Windows and macOS. Other platforms
might be supported. To display the decoded buffer, this tool relies on the
[minifb](https://github.com/emoon/rust_minifb) crate - a fairly minimalist
cross-platform window creator.

Please note that it needs nightly Rust to be built, as it make use of the
`vec_into_raw_parts` feature.


## How to use it? ##############################################################

After building it (the most straightforward way being to [use
Cargo](https://doc.rust-lang.org/cargo/) here), you can use the resulting binary
to display a GIF image by adding its path as argument:
```sh
gif-renderer images/some-gif-file.gif
```

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


## What's left to do? ##########################################################

This tool works well and is pretty fast, but there is still some room for
improvement:

  - The Plain Text Extension could be completely handled.

  - For now we render each frame once it has been completely decoded.
    A nice improvement would be to be able to progressively display each pixels
    as they are being decoded. This would rarely be useful though, and might
    only be visible when either file access or LZW decoding is remarkably slow.

  - Some errors could be ignored (e.g. ignoring when too much color data is
    found instead of exiting) and logged.
