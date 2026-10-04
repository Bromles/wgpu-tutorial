---
title: "Текстуры и sampling: тексели и upload"
description: "Текстура, view и формат, загрузка рядов с выравниванием, textureLoad целых текселей."
prev:
  text: "Цвет и его кодирование"
  link: ../srgb-mixing/
next:
  text: "Текстуры и sampling: UV и фильтр"
  link: ../texture-sampling/
---

<script setup>
import TexelsUploadDiagram from './TexelsUploadDiagram.vue'
</script>


# Текстуры и sampling: тексели и upload

::: info Только native
Продолжаем через [фреймворк](../framework-triangle/); новый ресурс — текстура.
:::

Изображение курса до сих пор было либо clear-цветом, либо аналитическим цветом вершин.
Самый прямой носитель «картинки» — **текстура**: ресурс с данными, организованными по координатам и формату.
Начнём с изображения, знакомого по [первой главе](../image-data/).

**Эксперимент:** прямоугольник показывает массив 2×2 RGBA8 — красный, зелёный, синий, белый.
Цвет каждого пикселя читается из текстуры целиком, без смешивания: четыре квадранта четырёх известных цветов.

## Тексель и пиксель

**Тексель** — элемент текстуры, аналог пикселя изображения из главы 01; слово «пиксель» прибережём для элементов кадра.
Наша текстура — те самые 16 байтов: два текселя в верхней строке, два в нижней, от левого верхнего угла.

<TexelsUploadDiagram />

## Создание текстуры и загрузка

Текстура, как и буфер, описывается дескриптором: размер, формат, usage'ы, число mip-уровней (пока 1).

```rust
let texture = gpu.device.create_texture(&TextureDescriptor {
    label: Some("2x2 sRGB texture"),
    size: Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2,
    format: TextureFormat::Rgba8UnormSrgb,
    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
    view_formats: &[],
});
```

Формат `Rgba8UnormSrgb` — та же пара «байты + sRGB-кривая», что у поверхности: чтение текселя шейдером возвращает **линейное** значение — декодирование делает формат.
`TEXTURE_BINDING` разрешает привязку к шейдеру, `COPY_DST` — загрузку.

Загрузка идёт напрямую через очередь — байты кладутся в текстуру так же, как `write_buffer` кладёт их в буфер:

```rust
// A queue upload needs no 256-aligned pitch - the tight 8-byte row is
// fine; only encoder buffer<->texture copies require the alignment.
gpu.queue.write_texture(
    TexelCopyTextureInfo {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: TextureAspect::All,
    },
    &TEXELS,
    TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * WIDTH),
        rows_per_image: None,
    },
    Extent3d {
        width: WIDTH,
        height: HEIGHT,
        depth_or_array_layers: 1,
    },
);
```

`bytes_per_row` описывает шаг ряда в самих данных — здесь честные 8 байтов, без паддинга.
Отдельное правило существует для копирования через command encoder (`copy_texture_to_buffer`/`copy_buffer_to_texture`): там шаг ряда обязан быть кратен 256, и маленькие изображения дописывают пустыми байтами до него.
Загрузка через очередь — другой путь: это удобная запись «маленьких» данных, ей выравнивание не нужно; с ней мы пока и работаем.

**View** — представление текстуры для привязки; создаётся явно, чтобы увидеть его диагностическую метку:

```rust
let view = texture.create_view(&TextureViewDescriptor {
    label: Some("2x2 texture view"),
    ..TextureViewDescriptor::default()
});
```

View совместим по формату с текстурой: переинтерпретировать `Rgba8UnormSrgb` как произвольный другой формат нельзя — допустимая смена формата view ограничена парой `Unorm`/`UnormSrgb` одного базового формата и требует перечислить её в `view_formats` текстуры.
Привязка использует только view, и это безопасно: view держит ссылку на свою текстуру, поэтому хранить сам `Texture` рядом не обязательно.

## UV-координаты

Позиции вершин по-прежнему приходят из 32-байтовой записи, но второй атрибут заменила белая константа, а **UV** приехали отдельным буфером по 8 байт на вершину.
UV — координаты в единичном квадрате текстуры: `(0, 0)` — левый верхний тексель, `(1, 1)` — правый нижний; `v` растёт вниз по экрану, как и строки массива:

```rust
// Top-left maps to (0, 0), bottom-right to (1, 1): v grows downward on
// screen, matching the top-to-bottom row order of the uploaded array.
const UVS: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
```

```rust
const UV_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: 8,
    step_mode: VertexStepMode::Vertex,
    attributes: &[VertexAttribute {
        format: VertexFormat::Float32x2,
        offset: 0,
        shader_location: 2,
    }],
};
```

Интерполируются они тем же механизмом `@location`, что цвета в [«Интерполяции»](../vertex-colors/).

## textureLoad

Шейдер читает целые тексели:

```wgsl
@group(0) @binding(0) var TEX: texture_2d<f32>;

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // UV in [0, 1] scaled to texel indices; textureLoad reads whole texels
    // and the sRGB view decodes them to linear values.
    let extent = vec2<i32>(textureDimensions(TEX));
    let texel = vec2<i32>(input.uv * vec2<f32>(extent));
    return textureLoad(TEX, texel, 0);
}
```

`texture_2d<f32>` — тип текстуры с float-значениями; sRGB-декодирование выполняет формат.
`textureDimensions` возвращает размер (2×2), UV масштабируется в индексы текселей, `textureLoad` берёт ровно один тексель без сэмплера.
Привязка во фрагментной стадии, `filterable: false`: фильтрацию (смешение соседних текселей) вводит следующая глава, и для чтения целых текселей она не нужна.

Кадр — четыре квадранта известных цветов:

![Эталонный кадр texture-load: красный, зелёный, синий и белый квадранты на сером фоне.](/results/texture-load.png)

Проверка на точное равенство кодов: texel с координатами кадра обязан вернуться теми же байтами, что были загружены.

```sh
cargo run -p texture-load
cargo test -p verify
```

## Проверьте модель

1. Почему цвета квадрантов — точные коды 255/0, а не результат декодирования?
2. Чему равен UV центра верхнего левого текселя и почему не (0, 0)?
3. Что изменится, если `uv` нижних вершин задать (0, 2)/(1, 2)? Сработает ли `textureLoad` с текселем за границей?
4. Сколько байтов занимает массив загрузки и как они разложены по рядам?

::: details Решения и контрольные выводы

1. Загружены коды; textureLoad возвращает декодированные линейные значения, а sRGB-цель кодирует их обратно при записи — симметричные операции возвращают исходные коды.
2. (0.25, 0.25): центр текселя — это середина его UV-диапазона [0, 0.5]. Углы (0,0), (1,0), (0,1), (1,1) — это углы UV-квадрата, не центры текселей.
3. `trunc(uv · extent)` при v = 2 и extent = 2 даёт `trunc(2 · 2) = 4` — строка 4 при стороне текстуры 2, далеко за границей массива. WebGPU определяет такое чтение: значение не обещено — может вернуться любой тексель внутри текстуры или нулевой вектор, без паники и без ошибки валидации. address-режимы сэмплера на `textureLoad` не действуют — это тема следующей страницы.
4. Ровно 16: два ряда по 8 полезных байтов лежат вплотную (`bytes_per_row = 8`), без staging-массива и пустых дополнений. Напоминание: encoder-копиям буфер↔текстура нужен кратный 256 шаг (см. выше) — queue-загрузке он не нужен, с ней мы этого и избегаем.

:::

Целые тексели — только половина механизма.
[Дальше](../texture-sampling/) — сэмплер: дробные UV, веса билинейной фильтрации и режимы адресации за границей единичного квадрата.
