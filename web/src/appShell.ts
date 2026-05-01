type Child = Node | string;

export type AppShell = {
  root: HTMLElement;
  canvas: HTMLCanvasElement;
  dropzone: HTMLElement;
  placeholder: HTMLElement;
  canvasWrap: HTMLElement;
  themeButton: HTMLButtonElement;
  githubStars: HTMLElement;
  fileInput: HTMLInputElement;
  exampleButton: HTMLButtonElement;
  playButton: HTMLButtonElement;
  reverseButton: HTMLButtonElement;
  prevButton: HTMLButtonElement;
  nextButton: HTMLButtonElement;
  exportButton: HTMLButtonElement;
  seekInput: HTMLInputElement;
  frameLabel: HTMLElement;
  status: HTMLElement;
  widthValue: HTMLElement;
  heightValue: HTMLElement;
  framesValue: HTMLElement;
  loopValue: HTMLElement;
};

function appendChildren(parent: HTMLElement | SVGElement, children: Child[]): void {
  for (const child of children) {
    parent.append(child);
  }
}

function element<K extends keyof HTMLElementTagNameMap>(
  tagName: K,
  options: {
    className?: string;
    textContent?: string;
    attributes?: Record<string, string>;
    children?: Child[];
  } = {},
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tagName);
  if (options.className) {
    node.className = options.className;
  }
  if (options.textContent !== undefined) {
    node.textContent = options.textContent;
  }
  if (options.attributes) {
    for (const [name, value] of Object.entries(options.attributes)) {
      node.setAttribute(name, value);
    }
  }
  if (options.children) {
    appendChildren(node, options.children);
  }
  return node;
}

function svgElement<K extends keyof SVGElementTagNameMap>(
  tagName: K,
  options: {
    className?: string;
    attributes?: Record<string, string>;
    children?: SVGElement[];
  } = {},
): SVGElementTagNameMap[K] {
  const node = document.createElementNS('http://www.w3.org/2000/svg', tagName);
  if (options.className) {
    node.setAttribute('class', options.className);
  }
  if (options.attributes) {
    for (const [name, value] of Object.entries(options.attributes)) {
      node.setAttribute(name, value);
    }
  }
  if (options.children) {
    appendChildren(node, options.children);
  }
  return node;
}

function icon(pathData: string, className?: string): SVGSVGElement {
  return svgElement('svg', {
    className,
    attributes: { viewBox: '0 0 24 24', 'aria-hidden': 'true' },
    children: [svgElement('path', { attributes: { d: pathData } })],
  });
}

function iconButton(
  iconNodes: SVGSVGElement[],
  options: {
    className: string;
    ariaLabel: string;
    title: string;
    disabled?: boolean;
    dataset?: Record<string, string>;
  },
): HTMLButtonElement {
  const button = element('button', {
    className: options.className,
    attributes: {
      'aria-label': options.ariaLabel,
      title: options.title,
      type: 'button',
    },
    children: iconNodes,
  });
  button.disabled = options.disabled ?? false;
  for (const [name, value] of Object.entries(options.dataset ?? {})) {
    button.dataset[name] = value;
  }
  return button;
}

function metaCard(
  label: string,
  value: string,
  dataName: string,
): { card: HTMLElement; value: HTMLElement } {
  const valueElement = element('span', {
    textContent: value,
    attributes: { [`data-${dataName}`]: '' },
  });
  return {
    card: element('div', {
      className: 'meta-card',
      children: [element('strong', { textContent: label }), valueElement],
    }),
    value: valueElement,
  };
}

export function buildAppShell(): AppShell {
  const frameLabel = element('strong', {
    textContent: '0 / 0',
    attributes: { 'data-frame-label': '' },
  });
  const dropzone = element('div', {
    className: 'dropzone',
    attributes: { 'data-dropzone': '' },
  });

  const placeholder = element('div', {
    className: 'placeholder',
    attributes: { 'data-placeholder': '' },
    children: [
      element('h2', { textContent: 'Drop a GIF here' }),
      element('p', {
        textContent:
          'It gets decoded in WebAssembly, then each frame is exposed as raw RGBA bytes for scrubbing, reverse playback, and frame export.',
      }),
    ],
  });

  const canvas = element('canvas');
  const canvasWrap = element('div', {
    className: 'canvas-wrap',
    children: [canvas],
  });
  canvasWrap.hidden = true;
  appendChildren(dropzone, [placeholder, canvasWrap]);

  const prevButton = iconButton([icon('M7 5h2v14H7zM18 6v12l-8-6z')], {
    className: 'transport-button icon-button',
    ariaLabel: 'Previous frame',
    title: 'Previous frame',
    disabled: true,
  });
  prevButton.dataset.prev = '';

  const playButton = iconButton(
    [icon('M8 6v12l10-6z', 'icon-play'), icon('M7 6h4v12H7zM13 6h4v12h-4z', 'icon-pause')],
    {
      className: 'transport-button icon-button primary play-toggle',
      ariaLabel: 'Play playback',
      title: 'Play playback',
      disabled: true,
      dataset: { play: '', state: 'play' },
    },
  );

  const nextButton = iconButton([icon('M15 5h2v14h-2zM6 6l8 6-8 6z')], {
    className: 'transport-button icon-button',
    ariaLabel: 'Next frame',
    title: 'Next frame',
    disabled: true,
  });
  nextButton.dataset.next = '';

  const reverseButton = iconButton(
    [
      icon('M5 7l6 5-6 5zM11 7l6 5-6 5z', 'icon-forward'),
      icon('M19 7l-6 5 6 5zM13 7l-6 5 6 5z', 'icon-reverse'),
    ],
    {
      className: 'transport-button icon-button direction-toggle',
      ariaLabel: 'Direction: forward',
      title: 'Direction: forward',
      disabled: true,
      dataset: { reverse: '', direction: 'forward' },
    },
  );

  const seekInput = element('input', {
    attributes: {
      id: 'frame-seek',
      type: 'range',
      min: '0',
      max: '0',
      value: '0',
      'data-seek': '',
    },
  });
  seekInput.disabled = true;

  const fileInput = element('input', {
    attributes: { type: 'file', accept: 'image/gif' },
  });

  const exportButton = element('button', {
    textContent: 'export frame',
    attributes: { type: 'button', 'data-export': '' },
  });
  exportButton.disabled = true;

  const exampleButton = element('button', {
    className: 'subtle-action',
    textContent: 'earth.gif (example)',
    attributes: { type: 'button', 'data-example': '' },
  });

  const githubStars = element('span', {
    className: 'github-stars-count',
    textContent: '...',
  });

  const githubLink = element('a', {
    className: 'github-link',
    attributes: {
      href: 'https://github.com/peaberberian/gif-renderer.rs',
      target: '_blank',
      rel: 'noreferrer',
      'aria-label': 'View gif-renderer.rs on GitHub',
    },
    children: [
      icon(
        'M12 2C6.48 2 2 6.58 2 12.24c0 4.52 2.87 8.36 6.84 9.71.5.1.68-.22.68-.48 0-.24-.01-1.05-.01-1.91-2.78.62-3.37-1.21-3.37-1.21-.45-1.18-1.11-1.49-1.11-1.49-.91-.64.07-.63.07-.63 1 .08 1.53 1.06 1.53 1.06.9 1.57 2.35 1.12 2.92.85.09-.67.35-1.12.64-1.37-2.22-.26-4.56-1.14-4.56-5.08 0-1.12.39-2.03 1.03-2.75-.1-.26-.45-1.3.1-2.71 0 0 .84-.28 2.75 1.05A9.4 9.4 0 0 1 12 6.84c.85 0 1.71.12 2.51.35 1.91-1.33 2.75-1.05 2.75-1.05.55 1.41.2 2.45.1 2.71.64.72 1.03 1.63 1.03 2.75 0 3.95-2.34 4.81-4.57 5.07.36.32.68.93.68 1.88 0 1.36-.01 2.45-.01 2.79 0 .27.18.59.69.48A10.25 10.25 0 0 0 22 12.24C22 6.58 17.52 2 12 2z',
        'github-mark',
      ),
      element('span', { textContent: 'GitHub' }),
      element('span', {
        className: 'github-stars',
        children: [
          icon(
            'M12 17.27L18.18 21l-1.64-7.03L22 9.24l-7.19-.61L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21z',
            'star-mark',
          ),
          githubStars,
        ],
      }),
    ],
  });

  const themeButton = iconButton(
    [
      icon(
        'M21 12.79A9 9 0 0 1 11.21 3c0 .13-.01.26-.01.39A9 9 0 1 0 20.61 13.8c.13 0 .26 0 .39-.01z',
        'theme-icon theme-icon-moon',
      ),
      icon(
        'M6.76 4.84l-1.8-1.79-1.41 1.41 1.79 1.8 1.42-1.42zm10.45 0l1.41 1.42 1.8-1.8-1.42-1.41-1.79 1.79zM12 4h1V1h-2v3h1zm7 8h3v-2h-3v2zm-7 7h-1v3h2v-3h-1zm-7-7H2v2h3v-2zm1.05 6.54l-1.79 1.8 1.41 1.41 1.8-1.79-1.42-1.42zm12.02 1.42l1.42 1.79 1.41-1.41-1.79-1.8-1.04 1.42zM12 6a6 6 0 1 0 0 12 6 6 0 0 0 0-12z',
        'theme-icon theme-icon-sun',
      ),
    ],
    {
      className: 'icon-button theme-toggle',
      ariaLabel: 'Toggle theme',
      title: 'Toggle theme',
    },
  );

  const widthMeta = metaCard('width', '-', 'width');
  const heightMeta = metaCard('height', '-', 'height');
  const framesMeta = metaCard('frames', '-', 'frames');
  const loopMeta = metaCard('loop count', '-', 'loop');

  const status = element('p', {
    className: 'status',
    textContent: 'Loading WebAssembly module...',
    attributes: { 'data-status': '' },
  });

  const root = element('div', {
    className: 'shell',
    children: [
      element('header', {
        className: 'topbar panel',
        children: [
          element('div', {
            className: 'topbar-brand',
            children: [
              element('h1', { className: 'eyebrow', textContent: 'gif-renderer.rs' }),
              element('p', {
                className: 'topnote',
                textContent: '.gif player based on the Rust decoder, using WebAssembly',
              }),
            ],
          }),
          element('div', {
            className: 'topbar-actions',
            children: [githubLink, themeButton],
          }),
        ],
      }),
      element('section', {
        className: 'layout',
        children: [
          element('div', {
            className: 'stage panel',
            children: [
              element('div', {
                className: 'stage-head',
                children: [
                  element('div', {
                    children: [element('p', { className: 'section-tag', textContent: 'player' })],
                  }),
                  element('div', {
                    className: 'frame-pill',
                    children: [element('span', { textContent: 'frame' }), frameLabel],
                  }),
                ],
              }),
              dropzone,
              element('div', {
                className: 'timeline',
                children: [
                  element('div', {
                    className: 'timeline-row',
                    children: [
                      element('label', {
                        className: 'timeline-label',
                        attributes: { for: 'frame-seek' },
                        children: [
                          element('span', { textContent: 'frame strip' }),
                          element('span', {
                            className: 'timeline-note',
                            textContent: 'drag the playhead',
                          }),
                        ],
                      }),
                      seekInput,
                      element('div', {
                        className: 'transport-row',
                        children: [prevButton, playButton, nextButton, reverseButton],
                      }),
                    ],
                  }),
                ],
              }),
            ],
          }),
          element('aside', {
            className: 'panel controls',
            children: [
              element('div', {
                className: 'controls-head',
                children: [
                  element('div', {
                    children: [element('p', { className: 'section-tag', textContent: 'metadata' })],
                  }),
                ],
              }),
              element('div', {
                className: 'button-row primary-row',
                children: [
                  element('label', {
                    className: 'file-button loud',
                    children: ['load gif', fileInput],
                  }),
                  exportButton,
                ],
              }),
              element('div', {
                className: 'aux-actions',
                children: [exampleButton],
              }),
              element('div', {
                className: 'meta-grid',
                children: [widthMeta.card, heightMeta.card, framesMeta.card, loopMeta.card],
              }),
              element('div', {
                className: 'logbook',
                children: [
                  element('p', { className: 'section-tag', textContent: 'status' }),
                  status,
                ],
              }),
            ],
          }),
        ],
      }),
    ],
  });

  return {
    root,
    canvas,
    dropzone,
    placeholder,
    canvasWrap,
    themeButton,
    githubStars,
    fileInput,
    exampleButton,
    playButton,
    reverseButton,
    prevButton,
    nextButton,
    exportButton,
    seekInput,
    frameLabel,
    status,
    widthValue: widthMeta.value,
    heightValue: heightMeta.value,
    framesValue: framesMeta.value,
    loopValue: loopMeta.value,
  };
}
