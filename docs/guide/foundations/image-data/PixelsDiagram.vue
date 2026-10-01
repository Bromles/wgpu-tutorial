<template>
  <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 760 560"
      role="img"
      aria-labelledby="title desc"
      class="guide-diagram"
  >
    <title id="title">Четыре пикселя и шестнадцать байт</title>
    <desc id="desc">Начало сверху слева, x вправо, y вниз. Красный (0,0): 255,0,0,255 по адресу 0; зелёный (1,0): 0,255,0,255 по адресу 4; синий (0,1): 0,0,255,255 по адресу 8; белый (1,1): 255,255,255,255 по адресу 12. Обнуление только байта 12 делает последний пиксель голубым: 0,255,255,255. RGB показаны как sRGB, alpha везде 255.</desc>
    <defs>
      <marker id="px-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path
          d="M0 0 8 4 0 8"
          fill="none"
          stroke="var(--vp-c-text-3)"
      /></marker>
    </defs>
    <g font-family="Arial, sans-serif" font-size="16" fill="var(--vp-c-text-2)">
      <text x="24" y="32" font-size="21" font-weight="bold" fill="var(--vp-c-text-1)">2 × 2 RGBA8: до изменения</text>
      <text x="24" y="58">Увеличенная сетка данных, не снимок GPU</text>
      <path
          d="M64 85 H326 M42 104 V310"
          fill="none"
          stroke="var(--vp-c-text-3)"
          stroke-width="2"
          marker-end="url(#px-arrow)"
      />
      <text x="340" y="91">x</text><text x="34" y="335">y</text>
      <rect x="64" y="104" width="120" height="100" fill="#ff0000" />
      <rect x="184" y="104" width="120" height="100" fill="#00ff00" />
      <rect x="64" y="204" width="120" height="100" fill="#0000ff" />
      <rect x="184" y="204" width="120" height="100" fill="#ffffff" />
      <path
          d="M64 104 H304 V304 H64 Z M184 104 V304 M64 204 H304"
          fill="none"
          stroke="var(--vp-c-text-3)"
          stroke-width="2"
      />
      <g text-anchor="middle" font-weight="bold" fill="#1a1a2e">
        <rect x="79" y="138" width="90" height="32" rx="4" fill="#fff" />
        <rect x="199" y="138" width="90" height="32" rx="4" fill="#fff" />
        <rect x="79" y="238" width="90" height="32" rx="4" fill="#fff" />
        <text x="124" y="160">(0, 0)</text><text x="244" y="160">(1, 0)</text>
        <text x="124" y="260">(0, 1)</text><text x="244" y="260">(1, 1)</text>
      </g>
      <text x="358" y="120" font-weight="bold">Начало = 4 × (y × 2 + x)</text>
      <text x="358" y="153">Адрес</text><text x="448" y="153">Байты R, G, B, A</text>
      <g font-family="monospace" font-size="16">
        <text x="358" y="185">00–03</text><text x="448" y="185">255,   0,   0, 255</text>
        <text x="358" y="221">04–07</text><text x="448" y="221">  0, 255,   0, 255</text>
        <text x="358" y="257">08–11</text><text x="448" y="257">  0,   0, 255, 255</text>
        <text x="358" y="293">12–15</text><text x="448" y="293">255, 255, 255, 255</text>
      </g>
      <text x="64" y="337">Строка y=0, затем строка y=1; без промежутков</text>
      <path d="M24 359 H736" stroke="var(--vp-c-divider)" />
      <text x="24" y="393" font-size="20" font-weight="bold" fill="var(--vp-c-text-1)">После: только (1, 1).R = 0</text>
      <rect x="24" y="411" width="72" height="72" fill="#00ffff" stroke="var(--vp-c-text-3)" stroke-width="2" />
      <text x="116" y="436">Байт 12: 255 → 0; пиксель: [0, 255, 255, 255]</text>
      <text x="116" y="468">Остальные 15 байт не изменены; A = 255 везде.</text>
      <text x="24" y="519">Цвет квадратов задаёт RGB в sRGB; подписи не входят в массив.</text>
      <text x="24" y="544">Линейные 0.5 → sRGB ≈ 0.735 → байт 188 (не операция этого опыта).</text>
    </g>
  </svg>
</template>
