import './styles.css';
import { buildAppShell } from './appShell';
import { GifPlayer, type PlaybackDirection } from './player';
import { GifWasmModule } from './gifWasm';

type Theme = 'light' | 'dark';

const THEME_STORAGE_KEY = 'gif-renderer-theme';

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

function isSeekKeyboardEvent(event: KeyboardEvent): boolean {
  switch (event.key) {
    case 'ArrowLeft':
    case 'ArrowRight':
    case 'ArrowUp':
    case 'ArrowDown':
    case 'Home':
    case 'End':
    case 'PageUp':
    case 'PageDown':
      return true;
    default:
      return false;
  }
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
  let keyboardSeekEndTimerId: number | null = null;

  const clearKeyboardSeekEndTimer = () => {
    if (keyboardSeekEndTimerId !== null) {
      window.clearTimeout(keyboardSeekEndTimerId);
      keyboardSeekEndTimerId = null;
    }
  };

  const scheduleKeyboardSeekEnd = () => {
    clearKeyboardSeekEndTimer();
    keyboardSeekEndTimerId = window.setTimeout(() => {
      keyboardSeekEndTimerId = null;
      player.endSeekInteraction();
    }, 200);
  };

  const finishSeekInteraction = () => {
    clearKeyboardSeekEndTimer();
    player.endSeekInteraction();
  };

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

  shell.seekInput.addEventListener('pointerdown', () => {
    player.beginSeekInteraction();
  });

  shell.seekInput.addEventListener('input', () => {
    player.seek(Number(shell.seekInput.value));
  });

  shell.seekInput.addEventListener('keydown', (event) => {
    if (!isSeekKeyboardEvent(event)) {
      return;
    }
    player.beginSeekInteraction();
    scheduleKeyboardSeekEnd();
  });

  shell.seekInput.addEventListener('keyup', (event) => {
    if (!isSeekKeyboardEvent(event)) {
      return;
    }
    finishSeekInteraction();
  });

  shell.seekInput.addEventListener('pointerup', () => {
    finishSeekInteraction();
  });

  shell.seekInput.addEventListener('pointercancel', () => {
    finishSeekInteraction();
  });

  shell.seekInput.addEventListener('change', () => {
    finishSeekInteraction();
  });

  shell.seekInput.addEventListener('blur', () => {
    finishSeekInteraction();
  });

  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') {
      finishSeekInteraction();
    }
  });

  window.addEventListener('blur', () => {
    finishSeekInteraction();
  });

  window.addEventListener('beforeunload', () => {
    clearKeyboardSeekEndTimer();
    player.dispose();
  });

  player.setStatus('WebAssembly module loaded. Open or drop a GIF to begin.');
}

void main();
