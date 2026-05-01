type GifRendererExports = WebAssembly.Exports & {
  memory: WebAssembly.Memory;
  gif_alloc_input_buffer(len: number): number;
  gif_free_input_buffer(ptr: number, len: number): void;
  gif_decode_from_ptr(ptr: number, len: number): number;
  gif_free_decoded(handle: number): number;
  gif_get_width(handle: number): number;
  gif_get_height(handle: number): number;
  gif_get_frame_count(handle: number): number;
  gif_get_loop_count(handle: number): number;
  gif_get_frame_delay_cs(handle: number, frameIndex: number): number;
  gif_get_frame_rgba_ptr(handle: number, frameIndex: number): number;
  gif_get_frame_rgba_len(handle: number, frameIndex: number): number;
  gif_get_last_error_ptr(): number;
  gif_get_last_error_len(): number;
};

async function instantiateWasm(url: string): Promise<WebAssembly.Instance> {
  const response = await fetch(url);
  const bytes = await response.arrayBuffer();
  const module = await WebAssembly.instantiate(bytes, {});
  return module.instance;
}

export class WasmDecodedGif {
  readonly width: number;
  readonly height: number;
  readonly frameCount: number;
  readonly loopCount: number | null;

  constructor(
    private readonly exports: GifRendererExports,
    readonly handle: number,
  ) {
    this.width = exports.gif_get_width(handle);
    this.height = exports.gif_get_height(handle);
    this.frameCount = exports.gif_get_frame_count(handle);
    const loopCount = exports.gif_get_loop_count(handle);
    this.loopCount = loopCount >= 0 ? loopCount : null;
  }

  getFrameDelayMs(frameIndex: number): number | null {
    const delayCs = this.exports.gif_get_frame_delay_cs(this.handle, frameIndex);
    return delayCs >= 0 ? delayCs * 10 : null;
  }

  getFrameRgba(frameIndex: number): Uint8ClampedArray<ArrayBuffer> {
    const ptr = this.exports.gif_get_frame_rgba_ptr(this.handle, frameIndex);
    const len = this.exports.gif_get_frame_rgba_len(this.handle, frameIndex);
    const view = new Uint8ClampedArray(this.exports.memory.buffer, ptr, len);
    return new Uint8ClampedArray(view);
  }

  destroy(): void {
    this.exports.gif_free_decoded(this.handle);
  }
}

export class GifWasmModule {
  private constructor(private readonly exports: GifRendererExports) {}

  static async load(url: string): Promise<GifWasmModule> {
    const instance = await instantiateWasm(url);
    return new GifWasmModule(instance.exports as GifRendererExports);
  }

  decode(bytes: Uint8Array): WasmDecodedGif {
    const ptr = this.exports.gif_alloc_input_buffer(bytes.length);
    const wasmBytes = new Uint8Array(this.exports.memory.buffer, ptr, bytes.length);
    wasmBytes.set(bytes);

    const handle = this.exports.gif_decode_from_ptr(ptr, bytes.length);
    this.exports.gif_free_input_buffer(ptr, bytes.length);

    if (handle === 0) {
      throw new Error(this.readLastError() || 'Unknown GIF decoding error.');
    }

    return new WasmDecodedGif(this.exports, handle);
  }

  private readLastError(): string {
    const ptr = this.exports.gif_get_last_error_ptr();
    const len = this.exports.gif_get_last_error_len();
    if (len === 0) {
      return '';
    }
    const bytes = new Uint8Array(this.exports.memory.buffer, ptr, len);
    return new TextDecoder().decode(bytes);
  }
}
