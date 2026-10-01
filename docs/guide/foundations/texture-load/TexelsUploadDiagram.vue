<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 400" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Тексели, центры UV и загрузка рядов</title>
    <desc id="desc">Слева сетка 2 на 2 текселя: красный, зелёный в верхнем ряду, синий и белый в нижнем. Центры текселей имеют UV 0.25 и 0.75 по каждой оси и лежат в центрах нарисованных ячеек. Точка выборки с UV около 0.5 лежит между центрами и смешивает все четыре текселя. Справа схема загрузки: два ряда по 8 байтов лежат в массиве вплотную; очередь пишет их в текстуру как есть, без выравнивания шага.</desc>
    <g font-family="Arial, sans-serif" font-size="15" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="22" font-weight="bold" fill="var(--vp-c-text-1)">Тексели против пикселей; загрузка рядами</text>

      <!-- 2x2 texel grid -->
      <g stroke="var(--vp-c-text-3)" stroke-width="1.5">
        <rect x="80" y="80" width="160" height="160" fill="#4a2a2a"/>
        <rect x="240" y="80" width="160" height="160" fill="#1f4130"/>
        <rect x="80" y="240" width="160" height="160" fill="#24344e"/>
        <rect x="240" y="240" width="160" height="160" fill="#3a3a3a"/>
      </g>
      <g text-anchor="middle" fill="#dcdcdc">
        <text x="160" y="166">красный</text>
        <text x="320" y="166">зелёный</text>
        <text x="160" y="326">синий</text>
        <text x="320" y="326">белый</text>
      </g>
      <g fill="var(--vp-c-text-3)" font-family="monospace" font-size="13">
        <circle cx="160" cy="120" r="3" fill="var(--dg-red)"/><text x="168" y="112" fill="var(--vp-c-text-2)">(0.25, 0.25)</text>
        <circle cx="320" cy="120" r="3" fill="var(--dg-green)"/><text x="312" y="112" text-anchor="end" fill="var(--vp-c-text-2)">(0.75, 0.25)</text>
        <circle cx="160" cy="360" r="3" fill="var(--dg-blue)"/><text x="168" y="384" fill="var(--vp-c-text-2)">(0.25, 0.75)</text>
        <circle cx="320" cy="360" r="3" fill="var(--vp-c-text-2)"/><text x="312" y="384" text-anchor="end" fill="var(--vp-c-text-2)">(0.75, 0.75)</text>
      </g>
      <circle cx="240" cy="240" r="5" fill="var(--vp-c-text-2)" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <text x="252" y="236">выборка uv ≈ (0.5, 0.5):</text>
      <text x="252" y="256">смешивает все четыре линейных значения</text>

      <!-- Upload rows -->
      <text x="470" y="80" font-weight="bold">Upload: 2 ряда × 8 байтов</text>
      <g font-family="monospace" font-size="13">
        <rect x="470" y="100" width="220" height="28" fill="#2c2c44" stroke="#5a5a74"/>
        <text x="580" y="119" text-anchor="middle" fill="#dcdcdc">ряд 0: 8 байтов (red, green)</text>
        <rect x="470" y="136" width="220" height="28" fill="#2c2c44" stroke="#5a5a74"/>
        <text x="580" y="155" text-anchor="middle" fill="#dcdcdc">ряд 1: 8 байтов (blue, white)</text>
        <path d="M690 100 H730 M690 164 H730" stroke="var(--vp-c-text-3)" stroke-dasharray="3 3"/>
        <text x="700" y="140">шаг</text>
        <text x="470" y="196">bytes_per_row = 8:</text>
        <text x="470" y="216">ряды лежат вплотную,</text>
        <text x="470" y="236">без пустых байтов;</text>
        <text x="470" y="256">queue-загрузке не нужно</text>
        <text x="470" y="276">выравнивание 256.</text>
      </g>
    </g>
  </svg>
</template>
