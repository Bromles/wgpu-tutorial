<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 430" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Кусочная sRGB transfer function</title>
    <desc id="desc">График кодирования sRGB: по горизонтали линейное значение от 0 до 1, по вертикали кодированное значение от 0 до 1. Кривая проходит выше пунктирной прямой идентичности: линейные 0.5 кодируются в примерно 0.735, что квантуется в байт 188. Вблизи нуля кривая переходит в линейный участок. Нормализация байта 128 в 0.502 — это не половина света: decode даёт примерно 0.212.</desc>
    <g font-family="Arial, sans-serif" font-size="15" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="22" font-weight="bold" fill="var(--vp-c-text-1)">sRGB: кодирование не равно нормализации</text>

      <!-- Axes -->
      <g stroke="var(--vp-c-text-3)" stroke-width="1.5">
        <path d="M80 60 V340 H680"/>
      </g>
      <text x="360" y="372" text-anchor="middle">линейное значение (вклад света)</text>
      <text x="46" y="200" transform="rotate(-90 46 200)" text-anchor="middle">код sRGB</text>
      <g font-family="monospace" fill="var(--vp-c-text-3)">
        <text x="76" y="358" text-anchor="end">0.0</text>
        <text x="384" y="358" text-anchor="middle">0.5</text>
        <text x="684" y="358">1.0</text>
        <text x="72" y="344" text-anchor="end">0.0</text>
        <text x="72" y="138" text-anchor="end">0.735</text>
        <text x="72" y="64" text-anchor="end">1.0</text>
      </g>

      <!-- Identity dashed -->
      <path d="M80 340 L680 60" stroke="#5a5a74" stroke-dasharray="6 5" fill="none"/>
      <text x="620" y="300" fill="var(--vp-c-text-3)">если бы код = значение</text>

      <!-- sRGB curve -->
      <path d="M80 340 L80.9 339.9 C 140 242, 200 204, 260 176.5 S 320 153.7, 380 134.1 S 440 116.6, 500 100.8 S 560 86.2, 620 72.7 S 660 64, 680 60"
            stroke="var(--dg-curve)" stroke-width="3" fill="none"/>

      <!-- The 0.5 -> 0.735 point -->
      <g stroke="var(--vp-c-text-3)" stroke-dasharray="4 4" fill="none">
        <path d="M380 340 V134.1 M80 134.1 H380"/>
      </g>
      <circle cx="380" cy="134.1" r="5" fill="var(--dg-curve)" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <text x="394" y="128" fill="var(--dg-blue2)">linear 0.5 → код ≈ 0.735 → байт 188</text>

      <!-- 128 mark -->
      <circle cx="331" cy="134.1" r="4" fill="#dd5555" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <text x="180" y="120" fill="var(--dg-red)">байт 128 (0.502) → linear ≈ 0.212</text>
      <path d="M331 130 V116" stroke="var(--vp-c-text-3)" stroke-dasharray="3 3"/>

      <text x="24" y="404">Кусочная функция: вблизи нуля код = 12.92·x, дальше 1.055·x^(1/2.4) − 0.055.</text>
      <text x="24" y="424">Равные шаги кода — не равные прибавки света: тёмная часть диапазона плотнее.</text>
    </g>
  </svg>
</template>
