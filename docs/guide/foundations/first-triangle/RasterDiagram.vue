<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 980 560" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">От трёх вершин к фрагментам</title>
    <desc id="desc">Слева сетка пикселей 12 на 9 с треугольником: вершины (-0.75, -0.75), (0.75, -0.75), (0, 0.75) в clip-пространстве при w = 1. Залитые ячейки — покрытые выборки по модели «центр выборки внутри треугольника». Справа цепочка: три вызова вершинного шейдера по vertex_index, сборка одного треугольника списком TriangleList, покрытие выборок и запуск фрагментного шейдера для каждого кандидата, запись цвета в attachment. Отдельно условия clip: -w не больше x и y не больше w, z от 0 до w. Пограничные пиксели между GPU различаются.</desc>
    <defs>
      <marker id="rt-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 8 4 0 8" fill="none" stroke="var(--vp-c-text-3)"/></marker>
    </defs>
    <g font-family="Arial, sans-serif" font-size="16" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="24" font-weight="bold" fill="var(--vp-c-text-1)">Три вершины → покрытие → фрагменты</text>
      <text x="24" y="62">Модель: ячейка покрыта, если центр её выборки лежит внутри треугольника.</text>

      <!-- Pixel grid -->
      <g stroke="var(--vp-c-divider)" stroke-width="1">
        <path d="M48 88 V448 M88 88 V448 M128 88 V448 M168 88 V448 M208 88 V448 M248 88 V448 M288 88 V448 M328 88 V448 M368 88 V448 M408 88 V448 M448 88 V448 M488 88 V448 M528 88 V448"/>
        <path d="M48 88 H528 M48 128 H528 M48 168 H528 M48 208 H528 M48 248 H528 M48 288 H528 M48 328 H528 M48 368 H528 M48 408 H528 M48 448 H528"/>
      </g>

      <!-- Covered samples (center inside the triangle) -->
      <g fill="#24344e">
        <rect x="248" y="168" width="40" height="40"/><rect x="288" y="168" width="40" height="40"/>
        <rect x="208" y="208" width="40" height="40"/><rect x="248" y="208" width="40" height="40"/><rect x="288" y="208" width="40" height="40"/><rect x="328" y="208" width="40" height="40"/>
        <rect x="208" y="248" width="40" height="40"/><rect x="248" y="248" width="40" height="40"/><rect x="288" y="248" width="40" height="40"/><rect x="328" y="248" width="40" height="40"/>
        <rect x="168" y="288" width="40" height="40"/><rect x="208" y="288" width="40" height="40"/><rect x="248" y="288" width="40" height="40"/><rect x="288" y="288" width="40" height="40"/><rect x="328" y="288" width="40" height="40"/><rect x="368" y="288" width="40" height="40"/>
        <rect x="128" y="328" width="40" height="40"/><rect x="168" y="328" width="40" height="40"/><rect x="208" y="328" width="40" height="40"/><rect x="248" y="328" width="40" height="40"/><rect x="288" y="328" width="40" height="40"/><rect x="328" y="328" width="40" height="40"/><rect x="368" y="328" width="40" height="40"/><rect x="408" y="328" width="40" height="40"/>
        <rect x="128" y="368" width="40" height="40"/><rect x="168" y="368" width="40" height="40"/><rect x="208" y="368" width="40" height="40"/><rect x="248" y="368" width="40" height="40"/><rect x="288" y="368" width="40" height="40"/><rect x="328" y="368" width="40" height="40"/><rect x="368" y="368" width="40" height="40"/><rect x="408" y="368" width="40" height="40"/>
      </g>
      <g fill="#cfd9ea">
        <circle cx="268" cy="188" r="3"/><circle cx="308" cy="188" r="3"/>
        <circle cx="228" cy="228" r="3"/><circle cx="268" cy="228" r="3"/><circle cx="308" cy="228" r="3"/><circle cx="348" cy="228" r="3"/>
        <circle cx="228" cy="268" r="3"/><circle cx="268" cy="268" r="3"/><circle cx="308" cy="268" r="3"/><circle cx="348" cy="268" r="3"/>
        <circle cx="188" cy="308" r="3"/><circle cx="228" cy="308" r="3"/><circle cx="268" cy="308" r="3"/><circle cx="308" cy="308" r="3"/><circle cx="348" cy="308" r="3"/><circle cx="388" cy="308" r="3"/>
        <circle cx="148" cy="348" r="3"/><circle cx="188" cy="348" r="3"/><circle cx="228" cy="348" r="3"/><circle cx="268" cy="348" r="3"/><circle cx="308" cy="348" r="3"/><circle cx="348" cy="348" r="3"/><circle cx="388" cy="348" r="3"/><circle cx="428" cy="348" r="3"/>
        <circle cx="148" cy="388" r="3"/><circle cx="188" cy="388" r="3"/><circle cx="228" cy="388" r="3"/><circle cx="268" cy="388" r="3"/><circle cx="308" cy="388" r="3"/><circle cx="348" cy="388" r="3"/><circle cx="388" cy="388" r="3"/><circle cx="428" cy="388" r="3"/>
      </g>

      <!-- The triangle itself -->
      <path d="M108 403 L468 403 L288 133 Z" fill="none" stroke="var(--vp-c-text-1)" stroke-width="2.5"/>
      <g fill="var(--vp-c-bg)" stroke="var(--vp-c-text-2)" stroke-width="2">
        <circle cx="108" cy="403" r="5"/><circle cx="468" cy="403" r="5"/><circle cx="288" cy="133" r="5"/>
      </g>
      <g font-family="monospace" font-size="14">
        <text x="76" y="428">(-0.75, -0.75)</text>
        <text x="392" y="428">(0.75, -0.75)</text>
        <text x="262" y="122">(0, 0.75)</text>
      </g>

      <!-- Screen axes -->
      <g fill="none" stroke="var(--vp-c-text-3)" stroke-width="1.5" marker-end="url(#rt-arrow)">
        <path d="M48 76 H520"/><path d="M36 88 V436"/>
      </g>
      <text x="524" y="80">x</text><text x="28" y="456">y</text>
      <text x="24" y="482">Экранный y растёт вниз, поэтому вершина с NDC y = +0.75 оказывается сверху.</text>
      <text x="24" y="506">Точки — центры выборок; пограничные ячейки у рёбер между GPU различаются.</text>
      <text x="24" y="530">Кадр этой главы: 768 × 576, ячейок 12 × 9 — иллюстрация, не точный масштаб.</text>

      <!-- Right column: pipeline stages -->
      <g stroke="#5a5a74" stroke-width="1.5">
        <rect x="580" y="88" width="376" height="88" rx="8" fill="#4a3d1f"/>
        <rect x="580" y="224" width="376" height="64" rx="8" fill="#2c2c44"/>
        <rect x="580" y="336" width="376" height="64" rx="8" fill="#24344e"/>
        <rect x="580" y="448" width="376" height="64" rx="8" fill="#1f4130"/>
      </g>
      <g fill="#dcdcdc">
        <text x="596" y="114" font-weight="bold">Clip-условия при w = 1:</text>
        <text x="596" y="140">−w ≤ x, y ≤ w;  0 ≤ z ≤ w. Здесь z = 0.5.</text>
        <text x="596" y="164">Вершина вне диапазона отсекается вместе с частью треугольника.</text>
        <text x="596" y="250" font-weight="bold">vs_main — 3 вызова</text>
        <text x="596" y="274">vertex_index = 0, 1, 2 выбирает позицию.</text>
        <text x="596" y="362" font-weight="bold">Сборка и покрытие</text>
        <text x="596" y="386">TriangleList: один треугольник; covered samples — кандидаты.</text>
        <text x="596" y="474" font-weight="bold">fs_main и attachment</text>
        <text x="596" y="498">Кандидат даёт цвет; запись — в байты кадра.</text>
      </g>
      <g fill="none" stroke="var(--vp-c-text-3)" stroke-width="2" marker-end="url(#rt-arrow)">
        <path d="M768 176 V220"/><path d="M768 288 V332"/><path d="M768 400 V444"/>
      </g>
    </g>
  </svg>
</template>
