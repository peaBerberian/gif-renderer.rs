# gif-renderer #################################################################

This is a simple GIF decoder written in Rust, compatible to both 87a and 89a GIF
versions.

This tool should work as expected on Linux, Windows and macOS. Other platforms
might be supported. To display the decoded buffer, this tool relies on the
[minifb](https://github.com/emoon/rust_minifb) crate - a fairly minimalist
cross-platform window creator.


## How to use it? ##############################################################

After building it (the most straightforward way being to [use
Cargo](https://doc.rust-lang.org/cargo/) here), you can use the resulting binary
to display a GIF image by adding its path as argument:
```sh
gif-renderer images/some-gif-file.gif
```

## Is it finished? Can I use this? #############################################

Yes!

The huge majority of valid GIF files will be fully-rendered, and rarely
encountered features like interlacing or the "restore to previous" disposal
method should be well managed.

---

There is still some room for improvement:

--

The GIF 89a "Plain Text Extension" is ignored when encountered in a GIF file.

Plain Text Extension support is hard to do, never used and even web browsers do
not seem to handle it well.

As such I considered that I can safely ignore it.

--

On the performance side, we're good but we could be better.

For example, we begin rendering only when all images have been decoded.
We could do it in a much more "streaming" manner, where the first frame is being
rendered while the next one is still decoding.

--

Wrapping most parsing operation's return in a Rust's `Result` type seems the
cleaner and idiomatic way to do it, but I'm under the impression that it incurs
a small overhead that can amount to quite a few supplementary milliseconds for
multi-MB files.
