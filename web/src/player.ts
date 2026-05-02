import { type WasmDecodedGif } from './gifWasm';

export type PlaybackDirection = 1 | -1;

type PlayerButtons = {
  playButton: HTMLButtonElement;
  reverseButton: HTMLButtonElement;
  previousFrame: HTMLButtonElement;
  nextFrame: HTMLButtonElement;
  exportButton: HTMLButtonElement;
};

export class GifPlayer {
  private decodedGif: WasmDecodedGif | null = null;
  private frameIndex = 0;
  private playbackDirection: PlaybackDirection = 1;
  private playing = false;
  private timerId: number | null = null;
  private playbackSession = 0;
  private seeking = false;
  private resumePlaybackAfterSeek = false;
  private readonly imageDataCache = new Map<number, ImageData>();

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly status: HTMLElement,
    private readonly frameLabel: HTMLElement,
    private readonly seekInput: HTMLInputElement,
    private readonly metaWidth: HTMLElement,
    private readonly metaHeight: HTMLElement,
    private readonly metaFrames: HTMLElement,
    private readonly metaLoop: HTMLElement,
    private readonly buttons: PlayerButtons,
  ) {}

  load(decodedGif: WasmDecodedGif): void {
    this.stopPlayback();
    this.disposeCurrentGif();
    this.decodedGif = decodedGif;
    this.frameIndex = 0;
    this.playbackDirection = 1;
    this.imageDataCache.clear();
    this.canvas.width = decodedGif.width;
    this.canvas.height = decodedGif.height;
    this.seekInput.max = String(Math.max(decodedGif.frameCount - 1, 0));
    this.seekInput.value = '0';
    this.metaWidth.textContent = String(decodedGif.width);
    this.metaHeight.textContent = String(decodedGif.height);
    this.metaFrames.textContent = String(decodedGif.frameCount);
    this.metaLoop.textContent =
      decodedGif.loopCount === null ? 'Not specified' : String(decodedGif.loopCount);
    this.renderFrame(0);
    this.setStatus('GIF decoded. You can seek, reverse playback, or export frames.');
    this.canvas.classList.add('is-interactive');
    this.canvas.title = 'Click to pause or play';
    this.seekInput.disabled = false;
    this.buttons.playButton.disabled = false;
    this.buttons.reverseButton.disabled = false;
    this.buttons.previousFrame.disabled = false;
    this.buttons.nextFrame.disabled = false;
    this.buttons.exportButton.disabled = false;
  }

  setStatus(message: string): void {
    this.status.textContent = message;
  }

  togglePlay(): void {
    if (!this.decodedGif) {
      return;
    }
    if (this.playing) {
      this.stopPlayback();
      return;
    }
    this.resumePlayback();
  }

  setDirection(direction: PlaybackDirection): void {
    this.playbackDirection = direction;
    this.buttons.reverseButton.dataset.direction = direction === -1 ? 'reverse' : 'forward';
    this.buttons.reverseButton.setAttribute(
      'aria-label',
      direction === -1 ? 'Direction: reverse' : 'Direction: forward',
    );
    this.buttons.reverseButton.title =
      direction === -1 ? 'Direction: reverse' : 'Direction: forward';
  }

  showNextFrame(step: PlaybackDirection): void {
    if (!this.decodedGif) {
      return;
    }
    this.stopPlayback();
    this.frameIndex = this.wrapFrameIndex(this.frameIndex + step);
    this.renderFrame(this.frameIndex);
  }

  seek(frameIndex: number): void {
    if (!this.decodedGif) {
      return;
    }
    this.frameIndex = Math.max(0, Math.min(frameIndex, this.decodedGif.frameCount - 1));
    this.renderFrame(this.frameIndex);
  }

  beginSeekInteraction(): void {
    if (this.seeking) {
      return;
    }
    this.seeking = true;
    this.resumePlaybackAfterSeek = this.playing;
    if (this.playing) {
      this.pausePlayback();
    }
  }

  endSeekInteraction(): void {
    if (!this.seeking) {
      return;
    }
    this.seeking = false;
    const shouldResume = this.resumePlaybackAfterSeek;
    this.resumePlaybackAfterSeek = false;
    if (shouldResume) {
      this.resumePlayback();
    }
  }

  exportCurrentFrame(): void {
    if (!this.decodedGif) {
      return;
    }
    const fileName = `frame-${String(this.frameIndex).padStart(4, '0')}.png`;
    this.canvas.toBlob((blob) => {
      if (!blob) {
        this.setStatus('Could not export the current frame.');
        return;
      }
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = fileName;
      link.click();
      URL.revokeObjectURL(url);
      this.setStatus(`Exported ${fileName}.`);
    }, 'image/png');
  }

  dispose(): void {
    this.disposeCurrentGif();
    this.clearTimer();
  }

  private scheduleNextTick(): void {
    if (!this.decodedGif || !this.playing) {
      return;
    }
    const playbackSession = this.playbackSession;
    const delayMs = this.decodedGif.getFrameDelayMs(this.frameIndex) ?? 100;
    this.timerId = window.setTimeout(
      () => {
        if (!this.decodedGif || !this.playing || playbackSession !== this.playbackSession) {
          return;
        }
        this.frameIndex = this.wrapFrameIndex(this.frameIndex + this.playbackDirection);
        this.renderFrame(this.frameIndex);
        this.scheduleNextTick();
      },
      Math.max(delayMs, 20),
    );
  }

  private renderFrame(frameIndex: number): void {
    const decodedGif = this.decodedGif;
    if (!decodedGif) {
      return;
    }
    const imageData = this.getImageData(frameIndex);
    const context = this.canvas.getContext('2d');
    if (!context) {
      throw new Error('2D canvas context is unavailable.');
    }
    context.putImageData(imageData, 0, 0);
    this.seekInput.value = String(frameIndex);
    this.frameLabel.textContent = `${frameIndex + 1} / ${decodedGif.frameCount}`;
  }

  private getImageData(frameIndex: number): ImageData {
    const cached = this.imageDataCache.get(frameIndex);
    if (cached) {
      return cached;
    }
    if (!this.decodedGif) {
      throw new Error('No GIF loaded.');
    }
    const rgba = this.decodedGif.getFrameRgba(frameIndex);
    const imageData = new ImageData(rgba, this.decodedGif.width, this.decodedGif.height);
    this.imageDataCache.set(frameIndex, imageData);
    return imageData;
  }

  private wrapFrameIndex(nextFrameIndex: number): number {
    if (!this.decodedGif) {
      return 0;
    }
    const { frameCount } = this.decodedGif;
    if (nextFrameIndex < 0) {
      return frameCount - 1;
    }
    if (nextFrameIndex >= frameCount) {
      return 0;
    }
    return nextFrameIndex;
  }

  private clearTimer(): void {
    if (this.timerId !== null) {
      window.clearTimeout(this.timerId);
      this.timerId = null;
    }
  }

  private stopPlayback(): void {
    this.resumePlaybackAfterSeek = false;
    this.pausePlayback();
    this.syncPlayButton();
  }

  private pausePlayback(): void {
    this.playing = false;
    this.playbackSession += 1;
    this.clearTimer();
    this.syncPlayButton();
  }

  private resumePlayback(): void {
    if (!this.decodedGif) {
      return;
    }
    this.playing = true;
    this.playbackSession += 1;
    this.syncPlayButton();
    this.scheduleNextTick();
  }

  private syncPlayButton(): void {
    const effectivelyPlaying = this.playing || (this.seeking && this.resumePlaybackAfterSeek);
    this.buttons.playButton.dataset.state = effectivelyPlaying ? 'pause' : 'play';
    this.buttons.playButton.setAttribute(
      'aria-label',
      effectivelyPlaying ? 'Pause playback' : 'Play playback',
    );
    this.buttons.playButton.title = effectivelyPlaying ? 'Pause playback' : 'Play playback';
  }

  private disposeCurrentGif(): void {
    if (this.decodedGif) {
      this.decodedGif.destroy();
      this.decodedGif = null;
    }
  }
}
