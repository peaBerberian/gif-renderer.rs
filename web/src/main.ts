import './styles.css';
import { buildAppShell } from './appShell';
import { GifWasmModule, type WasmDecodedGif } from './gifWasm';

type PlaybackDirection = 1 | -1;
type Theme = 'light' | 'dark';

const THEME_STORAGE_KEY = 'gif-renderer-theme';

class GifPlayer {
  private decodedGif: WasmDecodedGif | null = null;
  private frameIndex = 0;
  private playbackDirection: PlaybackDirection = 1;
  private playing = false;
  private timerId: number | null = null;
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
    private readonly buttons: {
      playButton: HTMLButtonElement;
      reverseButton: HTMLButtonElement;
      previousFrame: HTMLButtonElement;
      nextFrame: HTMLButtonElement;
      exportButton: HTMLButtonElement;
    },
  ) {}

  load(decodedGif: WasmDecodedGif): void {
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
    this.playing = !this.playing;
    this.buttons.playButton.dataset.state = this.playing ? 'pause' : 'play';
    this.buttons.playButton.setAttribute(
      'aria-label',
      this.playing ? 'Pause playback' : 'Play playback',
    );
    this.buttons.playButton.title = this.playing ? 'Pause playback' : 'Play playback';
    if (this.playing) {
      this.scheduleNextTick();
    } else {
      this.clearTimer();
    }
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
    this.clearTimer();
    this.playing = false;
    this.buttons.playButton.dataset.state = 'play';
    this.buttons.playButton.setAttribute('aria-label', 'Play playback');
    this.buttons.playButton.title = 'Play playback';
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
    const delayMs = this.decodedGif.getFrameDelayMs(this.frameIndex) ?? 100;
    this.timerId = window.setTimeout(
      () => {
        if (!this.decodedGif) {
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

  private disposeCurrentGif(): void {
    if (this.decodedGif) {
      this.decodedGif.destroy();
      this.decodedGif = null;
    }
  }
}

async function readFileBytes(file: File): Promise<Uint8Array> {
  return new Uint8Array(await file.arrayBuffer());
}

async function readAssetBytes(path: string): Promise<Uint8Array> {
  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(`Could not fetch ${path} (${response.status})`);
  }
  return new Uint8Array(await response.arrayBuffer());
}

function getActiveTheme(): Theme {
  return document.documentElement.getAttribute('data-theme') === 'light' ? 'light' : 'dark';
}

function applyTheme(theme: Theme): void {
  document.documentElement.setAttribute('data-theme', theme);
  window.localStorage.setItem(THEME_STORAGE_KEY, theme);
}

function updateThemeButton(button: HTMLButtonElement, theme: Theme): void {
  const nextTheme = theme === 'dark' ? 'light' : 'dark';
  button.dataset.theme = theme;
  button.setAttribute('aria-label', `Switch to ${nextTheme} theme`);
  button.title = `Switch to ${nextTheme} theme`;
}

async function loadGithubStars(target: HTMLElement): Promise<void> {
  try {
    const response = await fetch('https://api.github.com/repos/peaberberian/gif-renderer.rs');
    if (!response.ok) {
      throw new Error(`GitHub API responded with ${response.status}`);
    }
    const payload = (await response.json()) as { stargazers_count?: number };
    const stars = payload.stargazers_count;
    target.textContent =
      typeof stars === 'number' ? Intl.NumberFormat('en-US').format(stars) : '--';
  } catch {
    target.textContent = '--';
  }
}

async function main(): Promise<void> {
  const root = document.querySelector('#app');
  if (!(root instanceof HTMLElement)) {
    throw new Error('Missing app root.');
  }

  const shell = buildAppShell();
  root.append(shell.root);
  updateThemeButton(shell.themeButton, getActiveTheme());
  void loadGithubStars(shell.githubStars);

  const wasm = await GifWasmModule.load('./pkg/gif_renderer.wasm');
  const player = new GifPlayer(
    shell.canvas,
    shell.status,
    shell.frameLabel,
    shell.seekInput,
    shell.widthValue,
    shell.heightValue,
    shell.framesValue,
    shell.loopValue,
    {
      playButton: shell.playButton,
      reverseButton: shell.reverseButton,
      previousFrame: shell.prevButton,
      nextFrame: shell.nextButton,
      exportButton: shell.exportButton,
    },
  );

  const loadGif = async (bytes: Uint8Array, label: string) => {
    player.setStatus(`Decoding ${label}...`);
    const decoded = wasm.decode(bytes);
    player.load(decoded);
    shell.placeholder.hidden = true;
    shell.canvasWrap.hidden = false;
  };

  shell.fileInput.addEventListener('change', async () => {
    const file = shell.fileInput.files?.[0];
    if (!file) {
      return;
    }
    try {
      await loadGif(await readFileBytes(file), file.name);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      player.setStatus(`Decode failed: ${message}`);
    } finally {
      shell.fileInput.value = '';
    }
  });

  shell.exampleButton.addEventListener('click', async () => {
    try {
      await loadGif(await readAssetBytes('./assets/earth.gif'), 'earth.gif');
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      player.setStatus(`Example load failed: ${message}`);
    }
  });

  shell.themeButton.addEventListener('click', () => {
    const nextTheme = getActiveTheme() === 'dark' ? 'light' : 'dark';
    applyTheme(nextTheme);
    updateThemeButton(shell.themeButton, nextTheme);
  });

  shell.dropzone.addEventListener('dragover', (event) => {
    event.preventDefault();
    shell.dropzone.classList.add('drag-over');
  });

  shell.dropzone.addEventListener('dragleave', () => {
    shell.dropzone.classList.remove('drag-over');
  });

  shell.dropzone.addEventListener('drop', async (event) => {
    event.preventDefault();
    shell.dropzone.classList.remove('drag-over');
    const file = event.dataTransfer?.files?.[0];
    if (!file) {
      return;
    }
    try {
      await loadGif(await readFileBytes(file), file.name);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      player.setStatus(`Decode failed: ${message}`);
    }
  });

  shell.playButton.addEventListener('click', () => {
    player.togglePlay();
  });

  shell.canvas.addEventListener('click', () => {
    player.togglePlay();
  });

  shell.reverseButton.addEventListener('click', () => {
    const nextDirection: PlaybackDirection =
      shell.reverseButton.dataset.direction === 'reverse' ? 1 : -1;
    player.setDirection(nextDirection);
  });

  shell.prevButton.addEventListener('click', () => {
    player.showNextFrame(-1);
  });

  shell.nextButton.addEventListener('click', () => {
    player.showNextFrame(1);
  });

  shell.exportButton.addEventListener('click', () => {
    player.exportCurrentFrame();
  });

  shell.seekInput.addEventListener('input', () => {
    player.seek(Number(shell.seekInput.value));
  });

  window.addEventListener('beforeunload', () => {
    player.dispose();
  });

  player.setStatus('WebAssembly module loaded. Open or drop a GIF to begin.');
}

void main();
