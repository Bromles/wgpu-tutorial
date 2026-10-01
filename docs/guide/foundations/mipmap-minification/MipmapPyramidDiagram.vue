<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 430" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Мип-пирамида и footprint</title>
    <desc id="desc">Пирамида уровней: 8 на 8 текселей шахматки, затем 4 на 4, 2 на 2 и 1 на 1. Каждый следующий уровень усредняет блоки 2 на 2 предыдущего в линейных значениях: уже уровень 1 становится равномерно серым. Справа footprint: один пиксель экрана накрывает около двух текселей уровня 0, что соответствует LOD около 1. Ниже последовательность фаз 0, 1/16, 1/8, 3/16, 1/4: без префильтрации цвет пикселя прыгает между чёрным и белым, с мипами остаётся серым.</desc>
    <g font-family="Arial, sans-serif" font-size="15" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="22" font-weight="bold" fill="var(--vp-c-text-1)">Пирамида 8 → 4 → 2 → 1 и зона покрытия пикселя</text>

      <!-- Level 0: 8x8 checkerboard (content colors: black and white texels) -->
      <g>
        <rect x="60" y="70" width="160" height="160" fill="#e6e6e6" stroke="var(--vp-c-text-3)"/>
        <g fill="#16161a">
          <rect x="80" y="70" width="20" height="20"/><rect x="120" y="70" width="20" height="20"/><rect x="160" y="70" width="20" height="20"/><rect x="200" y="70" width="20" height="20"/>
          <rect x="60" y="90" width="20" height="20"/><rect x="100" y="90" width="20" height="20"/><rect x="140" y="90" width="20" height="20"/><rect x="180" y="90" width="20" height="20"/>
          <rect x="80" y="110" width="20" height="20"/><rect x="120" y="110" width="20" height="20"/><rect x="160" y="110" width="20" height="20"/><rect x="200" y="110" width="20" height="20"/>
          <rect x="60" y="130" width="20" height="20"/><rect x="100" y="130" width="20" height="20"/><rect x="140" y="130" width="20" height="20"/><rect x="180" y="130" width="20" height="20"/>
          <rect x="80" y="150" width="20" height="20"/><rect x="120" y="150" width="20" height="20"/><rect x="160" y="150" width="20" height="20"/><rect x="200" y="150" width="20" height="20"/>
          <rect x="60" y="170" width="20" height="20"/><rect x="100" y="170" width="20" height="20"/><rect x="140" y="170" width="20" height="20"/><rect x="180" y="170" width="20" height="20"/>
          <rect x="80" y="190" width="20" height="20"/><rect x="120" y="190" width="20" height="20"/><rect x="160" y="190" width="20" height="20"/><rect x="200" y="190" width="20" height="20"/>
          <rect x="60" y="210" width="20" height="20"/><rect x="100" y="210" width="20" height="20"/><rect x="140" y="210" width="20" height="20"/><rect x="180" y="210" width="20" height="20"/>
        </g>
        <text x="140" y="252" text-anchor="middle">уровень 0: 8×8</text>
      </g>

      <!-- Level 1 -->
      <g>
        <rect x="290" y="110" width="80" height="80" fill="#7a7a7a" stroke="var(--vp-c-text-3)"/>
        <text x="330" y="214" text-anchor="middle">уровень 1: 4×4</text>
        <text x="330" y="234" text-anchor="middle" font-size="13">везде серый 188</text>
      </g>

      <!-- Level 2 -->
      <g>
        <rect x="430" y="130" width="40" height="40" fill="#7a7a7a" stroke="var(--vp-c-text-3)"/>
        <text x="450" y="194" text-anchor="middle">2×2</text>
      </g>

      <!-- Level 3 -->
      <g>
        <rect x="530" y="140" width="20" height="20" fill="#7a7a7a" stroke="var(--vp-c-text-3)"/>
        <text x="540" y="194" text-anchor="middle">1×1</text>
      </g>

      <g stroke="var(--vp-c-text-3)" stroke-width="1.5" fill="none" marker-end="url(#mm-a14)">
        <defs>
          <marker id="mm-a14" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 8 4 0 8" fill="none" stroke="var(--vp-c-text-3)"/></marker>
        </defs>
        <path d="M225 150 H286"/>
        <path d="M375 150 H426"/>
        <path d="M475 150 H526"/>
      </g>
      <text x="238" y="140" font-size="13">box 2×2 в linear</text>

      <!-- Footprint -->
      <text x="600" y="90" font-weight="bold">Footprint ≈ 2</text>
      <rect x="600" y="110" width="20" height="20" fill="none" stroke="var(--dg-red)" stroke-width="2"/>
      <rect x="620" y="110" width="20" height="20" fill="none" stroke="var(--dg-red)" stroke-width="2"/>
      <rect x="600" y="130" width="40" height="20" fill="none" stroke="var(--dg-curve)" stroke-dasharray="4 3" stroke-width="2"/>
      <text x="600" y="176" font-size="13">пиксель накрывает ~2 текселя</text>
      <text x="600" y="196" font-size="13">уровня 0 → LOD ≈ 1</text>

      <!-- Phase sequence -->
      <text x="24" y="300" font-weight="bold">Фазы 0, 1/16, 1/8, 3/16, 1/4 при сдвиге UV·k + (phase, 0)</text>
      <g font-family="monospace" font-size="13">
        <text x="24" y="326">LOD 0 (nearest):</text>
        <g>
          <rect x="180" y="312" width="24" height="20" fill="#16161a" stroke="var(--vp-c-text-3)"/><rect x="204" y="312" width="24" height="20" fill="#e6e6e6"/><rect x="228" y="312" width="24" height="20" fill="#16161a"/><rect x="252" y="312" width="24" height="20" fill="#e6e6e6"/><rect x="276" y="312" width="24" height="20" fill="#16161a"/>
        </g>
        <text x="330" y="326">пиксели прыгают чёрный/белый</text>
        <text x="24" y="366">LOD 1 (mips):</text>
        <g>
          <rect x="180" y="352" width="24" height="20" fill="#7a7a7a" stroke="var(--vp-c-text-3)"/><rect x="204" y="352" width="24" height="20" fill="#7a7a7a"/><rect x="228" y="352" width="24" height="20" fill="#7a7a7a"/><rect x="252" y="352" width="24" height="20" fill="#7a7a7a"/><rect x="276" y="352" width="24" height="20" fill="#7a7a7a"/>
        </g>
        <text x="330" y="366">предфильтрация держит серый</text>
      </g>
      <text x="24" y="410">Меньшая чувствительность к фазе — не восстановление деталей: клетки уже потеряны при усреднении.</text>
    </g>
  </svg>
</template>
