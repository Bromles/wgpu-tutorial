<template>
  <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 330" role="img" aria-labelledby="title desc" class="guide-diagram">
    <title id="title">Запись вершины: 32 байта, два атрибута</title>
    <desc id="desc">Горизонтальная лента из 32 байтов одной записи вершины. Байты 0-15 занимает атрибут position, четыре f32 по четыре байта; байты 16-31 занимает атрибут color, тоже четыре f32. Атрибут position имеет offset 0 и формат Float32x4, атрибут color — offset 16 и тот же формат. Стрелка stride показывает шаг записи 32 байта от начала одной записи до начала следующей. Формат вершины — конвенция приложения, буфер хранит только байты.</desc>
    <g font-family="Arial, sans-serif" font-size="15" fill="var(--vp-c-text-2)">
      <text x="24" y="34" font-size="22" font-weight="bold" fill="var(--vp-c-text-1)">Буфер хранит байты; смысл задаёт layout</text>

      <!-- Record 0 strip: 32 bytes, cell 18px -->
      <g>
        <rect x="60" y="80" width="144" height="40" fill="#24344e" stroke="#5a5a74"/>
        <rect x="204" y="80" width="144" height="40" fill="#1f4130" stroke="#5a5a74"/>
        <text x="132" y="105" text-anchor="middle" fill="#dcdcdc">position: 4 × f32</text>
        <text x="276" y="105" text-anchor="middle" fill="#dcdcdc">color: 4 × f32</text>
        <text x="52" y="105" text-anchor="end">запись 0</text>
      </g>
      <!-- Record 1 strip, dimmed -->
      <g opacity="0.45">
        <rect x="60" y="128" width="144" height="40" fill="#24344e" stroke="#5a5a74"/>
        <rect x="204" y="128" width="144" height="40" fill="#1f4130" stroke="#5a5a74"/>
        <text x="52" y="153" text-anchor="end">запись 1</text>
      </g>
      <g opacity="0.22">
        <rect x="60" y="176" width="144" height="40" fill="#24344e" stroke="#5a5a74"/>
        <rect x="204" y="176" width="144" height="40" fill="#1f4130" stroke="#5a5a74"/>
        <text x="52" y="201" text-anchor="end">запись 2</text>
      </g>

      <!-- Byte ruler -->
      <g font-family="monospace" font-size="13">
        <text x="60" y="66">0</text><text x="96" y="66">4</text><text x="132" y="66">8</text><text x="168" y="66">12</text>
        <text x="228" y="66">20</text><text x="264" y="66">24</text><text x="300" y="66">28</text><text x="348" y="66">31</text>
        <path d="M60 72 H348" stroke="var(--vp-c-divider)"/>
      </g>

      <!-- Attribute descriptors -->
      <g>
        <path d="M132 84 V232" stroke="var(--vp-c-text-3)" stroke-width="1.5" fill="none" marker-start="none"/>
        <path d="M132 226 V84" stroke="var(--vp-c-text-3)" stroke-width="1.5" fill="none"/>
        <text x="380" y="240" font-family="monospace">shader_location: 0</text>
        <text x="380" y="262" font-family="monospace">offset: 0   format: Float32x4</text>
        <path d="M276 168 V232 H376" stroke="var(--vp-c-text-3)" stroke-width="1.5" fill="none" marker-end="url(#vr-a2)"/>
        <defs>
          <marker id="vr-a2" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto"><path d="M0 0 8 4 0 8" fill="none" stroke="var(--vp-c-text-3)"/></marker>
        </defs>
      </g>
      <g>
        <text x="380" y="284" font-family="monospace">shader_location: 1</text>
        <text x="380" y="306" font-family="monospace">offset: 16  format: Float32x4</text>
      </g>

      <!-- Stride arrow -->
      <path d="M60 224 V246 M204 224 V246" stroke="var(--vp-c-text-3)"/>
      <text x="380" y="88" font-family="monospace" font-size="15">array_stride: 32 байта</text>
      <text x="380" y="112" font-family="monospace" font-size="15">step_mode: Vertex</text>
      <text x="380" y="136">шаг = одна запись на вершину;</text>
      <text x="380" y="158">следующая запись начинается</text>
      <text x="380" y="180">через 32 байта после текущей</text>

      <text x="24" y="268">Формат не хранится в буфере: буфер — диапазон байтов,</text>
      <text x="24" y="290">а layout интерпретирует его при fetch.</text>
    </g>
  </svg>
</template>
