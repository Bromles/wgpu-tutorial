---
editLink: false
---

# Bloom

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/bloom)

**Что уже должно быть понятно:**

- compute passes, storage textures, `textureLoad`/`textureStore`
- HDR и tone mapping
- render-to-texture

**Что появится в этой главе:**

- bright extraction — выделение пикселей ярче порога
- separable Gaussian blur — размытие в два прохода (горизонтальный + вертикальный)
- аддитивное наложение: `scene + bloom`
- пять проходов: scene, bright extract, H-blur, V-blur, composite

**Итог:** ярко освещённые грани кубов «светятся» — ореол разливается за их границы

---

Bloom — эффект, при котором яркие объекты испускают ореол света, «разливаясь» за свои границы.
В реальности это происходит из-за рассеивания света в линзе камеры или глазу.

## Алгоритм bloom

Bloom состоит из трёх этапов:

1. **Bright extraction** — выделить из HDR-изображения пиксели с яркостью выше порога
2. **Blur** — размыть выделенные яркие области (Gaussian blur)
3. **Composite** — добавить размытое свечение к оригинальному изображению

```
Scene (HDR) ──→ Bright Extract ──→ H-Blur ──→ V-Blur ──┐
       │                                                 │
       └──────────────── scene + bloom ──── tone map ────┘→ Screen
```

## Bright extraction

Compute-шейдер проверяет каждый пиксель: если яркость (luminance) выше порога — пиксель
попадает в «яркую» текстуру, иначе записывается чёрный:

```wgsl
let brightness = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
if (brightness > params.threshold) {
    textureStore(output_tex, vec2<i32>(id.xy), color);
} else {
    textureStore(output_tex, vec2<i32>(id.xy), vec4<f32>(0.0));
}
```

Luminance — стандартный Rec. 709: $L = 0.2126 R + 0.7152 G + 0.0722 B$. Это веса для sRGB/HDR
контента, рекомендованные ITU. В главе про [render-to-texture](/guide/advanced/render-to-texture/) мы
использовали BT.601 ($0.299, 0.587, 0.114$) — стандарт для SD-видео. Оба подхода корректны,
Rec. 709 лучше подходит для рендера, BT.601 — для видео. Порог `threshold = 1.0`
означает, что выбираются только HDR-значения (те, что не влезли бы в LDR).

## Separable Gaussian blur

Полный 2D Gaussian blur с ядром $9\times9$ требует 81 сэмпл на пиксель.
Separable blur разбивает его на два 1D прохода: горизонтальный ($9$ сэмплов) и вертикальный ($9$ сэмплов) —
всего 18 вместо 81.

Веса для 9-точечного 1D Gaussian (5 уникальных весов из-за симметрии):

```wgsl
let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
```

Для каждого пикселя суммируются взвешенные значения из соседних текселей:

```wgsl
var result = textureLoad(input_tex, vec2<i32>(id.xy), 0) * weights[0];
for (var i: i32 = 1; i < 5; i++) {
    let offset = params.direction * f32(i);
    // сэмпл в +offset и -offset
    result += textureLoad(input_tex, coord1, 0) * weights[i];
    result += textureLoad(input_tex, coord2, 0) * weights[i];
}
```

Направление передаётся через uniform-буфер: `(1/width, 0)` для горизонтального, `(0, 1/height)`
для вертикального. Два прохода используют один и тот же шейдер — разница только в bind group.

## Ping-pong между двумя текстурами

Два прохода размытия чередуются между двумя текстурами:

| Pass | Input | Output |
|------|-------|--------|
| Bright extraction | scene | bright |
| H-blur | bright | blur |
| V-blur | blur | bright |

После V-blur результат оказывается в текстуре `bright`. Это позволяет не создавать третью текстуру.

<img src="/diagrams/bloom-ping-pong.svg" alt="Bloom: ping-pong между текстурами" style="width: 100%;" />

## Composite

Финальный полноэкранный квад складывает оригинальную сцену и bloom:

```wgsl
let combined = scene.rgb + bloom.rgb;
let mapped = aces(combined);
return vec4<f32>(mapped, 1.0);
```

Аддитивное сложение (`+`) — ключевой момент: bloom не заменяет сцену, а добавляет свечение.
Tone mapping (ACES) сжимает результат для вывода на монитор.

## Uniform-буферы для параметров

Bright extraction и blur используют uniform-буферы для передачи параметров. С `encase`
не нужно считать паддинг вручную — `ShaderType` автоматически выравнивает поля по правилам WGSL:

```rust
#[derive(ShaderType)]
struct BrightParams { threshold: f32 }

#[derive(ShaderType)]
struct BlurParams { direction: glam::Vec2 }
```

Обе структуры благодаря uniform-выравниванию WGSL занимают в буфере 16 байт, хотя
полезных данных в них меньше. `encase::UniformBuffer` записывает байты корректно:

```rust
let mut d = encase::UniformBuffer::new(Vec::new());
d.write(&BlurParams { direction: glam::Vec2::new(1.0 / width, 0.0) }).unwrap();
ctx.queue.write_buffer(&blur_params_ub, 0, &d.into_inner());
```

Два bind group для blur создаются с разными направлениями:

```rust
// Horizontal: bright → blur
let hblur_bg = ... direction: [1.0/width, 0.0] ...
// Vertical: blur → bright
let vblur_bg = ... direction: [0.0, 1.0/height] ...
```

## Пять проходов в render

Порядок в `render()`:

1. **Render pass**: сцена → `scene_texture` (HDR)
2. **Compute pass**: bright extraction → `bright_texture`
3. **Compute pass**: H-blur: `bright` → `blur`
4. **Compute pass**: V-blur: `blur` → `bright`
5. **Render pass**: composite + ACES → screen

Все пять проходов записываются в один `CommandEncoder` и выполняются GPU последовательно.

## Типичные ошибки

::: warning Забыли tone mapping
Без ACES аддитивное сложение `scene + bloom` легко даёт значения $> 3.0$, и на LDR-мониторе всё будет белым.
:::

::: warning Используют одну текстуру для input/output
Compute-шейдер может читать и писать одну и ту же текстуру только с `read_write` доступом и соответствующим GPU feature. Проще использовать ping-pong.
:::

::: warning Неправильные веса Gaussian
Веса должны суммироваться ≈ 1.0, иначе яркость изменится.
:::

::: warning Порог 0.0
Всё изображение попадёт в bloom, и размытие «завалит» контраст. Начинайте с `threshold = 1.0`.
:::

## Что получилось

Ярко освещённые грани кубов испускают ореол света, «разливаясь» за свои границы. Bloom состоит
из трёх этапов: выделение ярких пикселей, separable Gaussian blur (два 1D-прохода вместо одного 2D)
и аддитивное наложение на оригинальную сцену. Вся обработка выполняется пятью GPU-проходами
(2 render + 3 compute) в одном `CommandEncoder`. Tone mapping обязателен — без него аддитивное
сложение HDR-значений даст белый экран.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить `threshold` на 0.5 — больше пикселей попадёт в bloom, свечение станет сильнее
- Увеличить радиус blur (больше 4) — более размытое свечение
- Поставить `threshold` на 2.0 — только самые яркие участки будут светиться

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/bloom)
