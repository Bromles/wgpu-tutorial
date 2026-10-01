<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 430" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Барицентрические координаты через площади</title>
    <desc id="desc">Треугольник с красной, зелёной и синей вершинами. Внутренняя точка P соединена с вершинами и делит треугольник на три подтреугольника. Подтреугольник напротив вершины A закрашен красным, напротив B — зелёным, напротив C — синим. Вес каждой вершины равен отношению площади противоположного подтреугольника к общей площади; сумма весов равна единице. В центроиде все три площади равны, поэтому веса равны по одной трети.</desc>
    <defs>
      <marker id="bc-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 8 4 0 8" fill="none" stroke="var(--vp-c-text-3)"/></marker>
    </defs>
    <g font-family="Arial, sans-serif" font-size="16" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="22" font-weight="bold" fill="var(--vp-c-text-1)">Три веса = три площади</text>
      <text x="24" y="60">Цвет вершины входит в точку P пропорционально площади напротив неё.</text>

      <polygon points="380,247 680,340 380,60" fill="#4a2a2a" stroke="#5a5a74"/>
      <polygon points="380,247 380,60 80,340" fill="#1f4130" stroke="#5a5a74"/>
      <polygon points="380,247 80,340 680,340" fill="#24344e" stroke="#5a5a74"/>
      <polygon points="80,340 680,340 380,60" fill="none" stroke="var(--vp-c-text-2)" stroke-width="2"/>
      <path d="M380 247 L80 340 M380 247 L680 340 M380 247 L380 60" stroke="var(--vp-c-text-3)" stroke-width="1.5"/>

      <circle cx="80" cy="340" r="6" fill="#dd5555" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <circle cx="680" cy="340" r="6" fill="#44aa66" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <circle cx="380" cy="60" r="6" fill="#5588aa" stroke="var(--vp-c-bg)" stroke-width="2"/>
      <circle cx="380" cy="247" r="5" fill="var(--vp-c-text-2)"/>

      <text x="94" y="336">A: красный (1, 0, 0)</text>
      <text x="520" y="336">B: зелёный (0, 1, 0)</text>
      <text x="398" y="52">C: синий (0, 0, 1)</text>
      <text x="392" y="252">P</text>

      <text x="430" y="170" fill="var(--dg-red)">S(P,B,C) → w_A</text>
      <text x="150" y="170" fill="var(--dg-green)">S(P,C,A) → w_B</text>
      <text x="300" y="312" fill="var(--dg-blue)">S(P,A,B) → w_C</text>

      <text x="24" y="386">w_A + w_B + w_C = 1: три площади в сумме дают весь треугольник.</text>
      <text x="24" y="410">В центроиде P все три площади равны — веса по 1/3 как следствие площадей.</text>
    </g>
  </svg>
</template>
