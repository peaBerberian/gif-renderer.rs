# gif-renderer #################################################################

This is a simple GIF decoder written in Rust, compatible to both 87a and 89a GIF
versions.

On the rendering side, this decoder use the
[minifb](https://github.com/emoon/rust_minifb) crate - a fairly minimalist
cross-platform window creator exposing a framebuffer interface.


## How to use it? ##############################################################

After building it, you can use the resulting binary to display a GIF image by
adding its path as argument:
```sh
gif-renderer images/some-gif-file.gif
```

## Is it finished? Can I use this? #############################################

Short answer, yes!

The huge majority of valid GIF files will be fully-rendered, and rarely
encountered features like interlacing or the "restore to previous" disposal
method should be well managed.

---

Long answer, there's still some work to do:

--

For example, invalid gif files will just trigger what rust calls a "panic"
which will result in a difficult-to-read error report.

I still have to work on a better error management to write a clean error to
stderr.

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

As an optional but useful feature, HTTP(s) addresses could also be handled.
With the rendering happening as soon as the first frame data is received.