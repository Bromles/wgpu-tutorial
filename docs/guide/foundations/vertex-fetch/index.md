---
title: "Геометрия в буферах: байты и vertex fetch"
description: Buffer как диапазон байтов, запись вершины с явным ABI, VertexFormat, offsets/stride и загрузка через write_buffer.
prev:
  text: "Интерполяция"
  link: ../vertex-colors/
next:
  text: "Индексированная геометрия"
  link: ../indexed-geometry/
---

<script setup>
import VertexRecordDiagram from './VertexRecordDiagram.vue'
</script>


# Геометрия в буферах: байты и vertex fetch

::: info Только native
Продолжаем через [фреймворк](../framework-triangle/); меняется только источник геометрии.
:::

Цвета вершин пока были константами шейдера: любое изменение — пересборка.
Первый шаг к управляемым данным — перенести вершины в **буфер**, ресурс из главы [«CPU, GPU и очередь»](../cpu-gpu-queue/).

**Эксперимент:** тот же треугольник с теми же цветами из [«Интерполяции»](../vertex-colors/), но позиции и цвета приезжают из буфера.
Кадр обязан совпасть байт в байт: меняется только путь данных.

## Buffer — это байты

`wgpu::Buffer` — диапазон байтов с размером и заявленными **usages**.
Буфер не знает, лежат в нём вершины, цвета или числа: любой смысл появляется только из того, кто и как его читает.

Создание и загрузка в `Sample::init`:

```rust
let vertex_buffer = gpu.device.create_buffer(&BufferDescriptor {
    label: Some("Triangle vertices"),
    size: size_of_val(&VERTICES) as u64,
    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    mapped_at_creation: false,
});
gpu.queue
    .write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&VERTICES));
```

`BufferUsages::VERTEX` разрешает использовать буфер как источник атрибутов вершин; `COPY_DST` — быть целью загрузки.
`mapped_at_creation: false` — прямой доступ к памяти буфера не запрашиваем; отображение (mapping) — отдельный механизм.
`queue.write_buffer` — первая встреча с загрузкой в этой форме: CPU готовит байты, очередь доставляет их в ресурс.
Здесь она выполняется один раз при инициализации: статические данные.

## Запись вершины и её ABI

Одна **вершина** — это набор атрибутов. Наша запись:

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}
```

Rust не обещает компоновку структур по умолчанию; `#[repr(C)]` фиксирует порядок полей и правила выравнивания C.
Два атрибута по четыре `f32`: `position` занимает байты 0–15, `color` — 16–31, размер записи 32 байта, неявного padding нет.
Трейты `Pod`/`Zeroable` из bytemuck — проверяемое обещание «эта структура — просто байты»; `cast_slice` превращает `&[Vertex]` в `&[u8]` без копирования.

Это и есть **ABI записи**: договор между Rust-структурой и тем, как GPU разберёт байты.

<VertexRecordDiagram />

## Layout: формат, offset, stride

Каким байтам какой атрибут соответствует, описывает `VertexBufferLayout`:

```rust
const LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
    array_stride: size_of::<Vertex>() as BufferAddress,
    step_mode: VertexStepMode::Vertex,
    attributes: &[
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 0,
            shader_location: 0,
        },
        VertexAttribute {
            format: VertexFormat::Float32x4,
            offset: 16,
            shader_location: 1,
        },
    ],
};
```

- **format** `Float32x4` — четыре 32-битных float подряд. Формат описывает хранение; fetch не только читает, но и приводит данные к `vec4<f32>` шейдера, если хранятся они иначе.
- **offset** — смещение атрибута внутри записи в байтах: 0 и 16.
- **array_stride** — шаг между записями: 32 байта от начала одной записи до начала следующей.
- **step_mode** `Vertex` — на каждую вершину читается следующая запись (второй режим, по экземплярам, появится в ответвлении об instancing).
- **shader_location** связывает атрибут с входом шейдера.

## Шейдер: входы вместо констант

```wgsl
struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = input.position;
    output.color = input.color.rgb;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
```

`@location(0)` и `@location(1)` на входе вершинного шейдера — те же номера, что `shader_location` в layout: так байты находят свои параметры.
Константы `POSITIONS` и `COLORS` исчезли: теперь их содержимое живёт в `VERTICES` на стороне CPU.
Интерполяция `@location(0)` на выходе работает как в прошлой главе.

## Draw с буфером

В командной записи появились привязка буфера и тот же вызов на три вершины:

```rust
pass.set_pipeline(&self.pipeline);
pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
pass.draw(0..3, 0..1);
```

`set_vertex_buffer(0, ...)` привязывает срез буфера к слоту 0 — позиции в списке `buffers` дескриптора pipeline: наш первый и единственный элемент. Именно этот список соединяет `LAYOUT` с исполнением; в [главе «От трёх точек к треугольнику»](../first-triangle/) он был пуст:

```rust
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(Vertex::LAYOUT)],
                compilation_options: PipelineCompilationOptions::default(),
            },
```

Диапазон `0..3` по-прежнему три вершины: изменился источник значений.

Кадр не изменился — автоматическая проверка сравнивает его с эталоном главы [«Интерполяция»](../vertex-colors/) с нулевым допуском:

```sh
cargo run -p vertex-fetch
cargo test -p verify
```

## Проверьте модель

1. Сколько байтов занимает буфер трёх вершин? Какой offset будет у `color` второй записи?
2. Что изменится в кадре, если задать второму атрибуту `offset: 0`? Почему это не ошибка загрузки, а другая интерпретация тех же байтов?
3. Зачем нужны `#[repr(C)]` и трейты bytemuck? Что именно проверяет компилятор, а что остаётся договорённостью программиста?
4. Замените `Float32x4` позиции на `Float32x3` с отдельным атрибутом `w` — какие offset/stride получатся? (Упражнение на бумаге, не на GPU.)

::: details Решения и контрольные выводы

1. `3 × 32 = 96` байтов; у второй записи `color` начинается с `32 + 16 = 48`.
2. Оба атрибута станут читать одни и те же байты 0–15 записи: цвет примет значения позиции. Валидация пройдёт — layout корректен формально, ошибочен по смыслу. Ошибки такого рода ищут по байтам, а не по сообщениям wgpu.
3. `#[repr(C)]` фиксирует раскладку полей; `Pod`/`Zeroable` проверяют, что тип — сплошные байты без скрытых полей. Компилятор не сверяет layout с WGSL: соответствие `shader_location` и полей — наша ответственность.
4. position { offset 0, Float32x3 }, w { offset 12, Float32 }, color { offset 16, Float32x4 }; stride остаётся 32, если w уместить в байты 12–15.

:::

В буфере накопится повторяющаяся геометрия: у прямоугольника два треугольника требуют шесть записей, хотя уникальных точек четыре.
[Следующая страница](../indexed-geometry/) вводит индексный буфер — обращение к записям по номеру.
