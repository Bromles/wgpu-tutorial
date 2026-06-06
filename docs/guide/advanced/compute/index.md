---
editLink: false
---

# Compute Passes

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/compute)

**Что уже должно быть понятно:**

- render-to-texture и постпроцессинг
- текстуры, bind groups, шейдеры
- HDR и `Rgba16Float`

**Что появится в этой главе:**

- compute pipeline — выполнение произвольных вычислений на GPU
- `texture_storage_2d` — текстура для записи из compute-шейдера
- `dispatch_workgroups` — запуск потоков
- `@compute @workgroup_size` — декораторы WGSL
- `textureLoad` / `textureStore` — прямой доступ к текселям

**Итог:** разделённый экран: слева — оригинальная сцена, справа — размытая версия, обработанная compute-шейдером

---

Все предыдущие главы использовали **графический конвейер**: вершины → растеризация → фрагменты.
Compute-шейдеры работают вне этого конвейера — это программы, которые GPU запускает как массив
параллельных потоков. Они не привязаны к треугольникам и могут читать/писать произвольные данные.

## Что делает compute-шейдер

Compute-шейдер — это функция WGSL с декораторами `@compute` и `@workgroup_size`:

```wgsl
@compute @workgroup_size(16, 16)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    // id.x, id.y — координаты текселя
}
```

GPU запускает N потоков, каждый получает свой `global_invocation_id` — уникальный индекс.
В данном случае каждый поток обрабатывает один пиксель изображения.

## Workgroup size

`@workgroup_size(16, 16)` означает, что потоки группируются в блоки 16×16 = 256 потоков.
Это важно для производительности: потоки внутри одной workgroup могут разделять память
(`var<workgroup>`), но в этой главе мы этого не используем. Значение 16×16 — стандартный
выбор для обработки изображений.

## Dispatch

CPU запускает compute-шейдер через `dispatch_workgroups(x, y, z)`:

```rust
let wg_x = (width + 15) / 16;
let wg_y = (height + 15) / 16;
cpass.dispatch_workgroups(wg_x, wg_y, 1);
```

Общее число потоков = `x * 16` × `y * 16`. Округление вверх (`+ 15`) гарантирует, что
каждый пиксель будет обработан. Шейдер проверяет границы:

```wgsl
let dims = textureDimensions(input_tex);
if (id.x >= dims.x || id.y >= dims.y) {
    return;
}
```

## Storage texture

Обычные текстуры (`texture_2d`) доступны для чтения через `textureLoad` в любых шейдерах,
но **не поддерживают запись**. Для записи GPU предоставляет **storage texture**:

```wgsl
@group(0) @binding(1)
var output_tex: texture_storage_2d<rgba16float, write>;
```

`write` — доступ только на запись. Для чтения-записи нужен `read_write` и соответствующий
GPU feature. Формат `rgba16float` совпадает с форматом текстуры — это обязательно. Обратите
внимание: мы используем 16-bit float, а не привычный `rgba8unorm`. При box blur каждый тексель
усредняется с соседями, и 8-bit точности (256 градаций на канал) привело бы к заметному
banding-эффекту — ступенчатым переходам вместо плавных градиентов. 16-bit float даёт
достаточную точность для многократного размытия.

Запись одного текселя:

```wgsl
textureStore(output_tex, vec2<i32>(id.xy), color / total);
```

На стороне Rust текстура создаётся с флагом `STORAGE_BINDING`:

```rust
let blur_texture = ctx.device.create_texture(&TextureDescriptor {
    format: TextureFormat::Rgba16Float,
    usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
    ...
});
```

`TEXTURE_BINDING` нужен, чтобы post-шейдер мог сэмплировать размытую текстуру.

## Blur: box filter

Размытие реализовано как простое усреднение по квадратному окну радиуса 4:

```wgsl
var color = vec4<f32>(0.0);
let radius = 4;
for (var dy: i32 = -radius; dy <= radius; dy++) {
    for (var dx: i32 = -radius; dx <= radius; dx++) {
        let coord = clamp(
            vec2<i32>(i32(id.x) + dx, i32(id.y) + dy),
            vec2<i32>(0, 0),
            vec2<i32>(i32(dims.x) - 1, i32(dims.y) - 1),
        );
        color += textureLoad(input_tex, coord, 0);
    }
}
let total = f32((2 * radius + 1) * (2 * radius + 1));
textureStore(output_tex, vec2<i32>(id.xy), color / total);
```

`textureLoad` читает конкретный тексель без фильтрации — в отличие от `textureSample`,
который использует сэмплер. Координаты — целые числа, не нормализованные.

::: info textureLoad vs textureSample

- **`textureLoad(tex, coords)`** — читает конкретный тексель по целочисленным координатам. Без фильтрации, сэмплер не нужен. Доступен **во всех** стадиях шейдера, включая compute.
- **`textureSample(tex, sampler, coords)`** — читает с фильтрацией/сэмплированием по float-координатам. Доступен **только** во фрагментных шейдерах.

Именно поэтому в compute-шейдерах всегда используется `textureLoad` — `textureSample` в них недоступен. Во фрагментных шейдерах `textureSample` предпочтительнее для фильтрованных чтений, а `textureLoad` — для точного доступа к текселям.

:::

Clamp гарантирует, что координаты не выйдут за границы текстуры — краевые пиксели
«размазываются» на соседей за пределами изображения.

## Три этапа рендеринга

```
Pass 1 (render):  scene → scene_texture (Rgba16Float)
Pass 2 (compute): scene_texture → blur_texture (box blur)
Pass 3 (render):  scene_texture + blur_texture → screen (split-screen)
```

### Pass 1: сцена

Обычный render pass, результат записывается в `Rgba16Float` offscreen-текстуру. Это та же
сцена с кубом и полом из предыдущих глав.

### Pass 2: compute blur

```rust
let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor {
    label: Some("Blur Pass"),
    timestamp_writes: None,
});
cpass.set_pipeline(&self.compute_pipeline);
cpass.set_bind_group(0, &self.compute_bind_group, &[]);
let wg_x = (ctx.surface_config.width + 15) / 16;
let wg_y = (ctx.surface_config.height + 15) / 16;
cpass.dispatch_workgroups(wg_x, wg_y, 1);
```

Compute pass записывается в тот же `CommandEncoder`, что и render passes. GPU выполняет
их в порядке записи.

### Pass 3: split-screen

Полноэкранный квад, где левая половина показывает оригинал, правая — размытую версию:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if (input.uv.x < 0.5) {
        return textureSample(scene_tex, tex_sampler, input.uv);
    } else {
        return textureSample(blur_tex, tex_sampler, input.uv);
    }
}
```

## Compute pipeline

Compute pipeline создаётся проще, чем render pipeline — нет вершинных данных,
растеризации, depth/stencil:

```rust
let compute_pipeline = ctx.device.create_compute_pipeline(&ComputePipelineDescriptor {
    label: Some("Blur Pipeline"),
    layout: Some(&compute_layout),
    module: &blur_shader,
    entry_point: Some("main"),
    compilation_options: PipelineCompilationOptions::default(),
});
```

## Bind groups для compute

Compute bind group содержит две текстуры:

| Binding | Тип | Доступ |
|---------|-----|--------|
| 0 | `texture_2d<f32>` | чтение (scene) |
| 1 | `texture_storage_2d<rgba16float, write>` | запись (blur) |

```rust
let compute_bgl = ctx.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    entries: &[
        BindGroupLayoutEntry { binding: 0, visibility: ShaderStages::COMPUTE,
            ty: BindingType::Texture { ... }, count: None },
        BindGroupLayoutEntry { binding: 1, visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::Rgba16Float,
                view_dimension: TextureViewDimension::D2 },
            count: None },
    ],
});
```

Обратите внимание: `visibility: ShaderStages::COMPUTE`, не `FRAGMENT`.

## Типичные ошибки

- **`TEXTURE_BINDING` без `STORAGE_BINDING`** — compute-шейдер не сможет писать в текстуру.
  Флаг `usage` обязан включать `STORAGE_BINDING`.
- **Несовпадение формата в `texture_storage_2d` и `TextureDescriptor`** — если в WGSL указан
  `rgba16float`, а текстура создана как `Rgba8Unorm`, pipeline не создастся.
- **Забыли `+ 15` при dispatch** — если размер текстуры не кратен 16, краевые пиксели не будут
  обработаны. Без проверки границ в шейдере — запись «за пределами».
- **`textureSample` вместо `textureLoad`** в compute-шейдере — `textureSample` использует
  сэмплер и недоступен в compute-шейдерах. Нужен `textureLoad` с целыми координатами.
- **`ShaderStages::FRAGMENT` вместо `COMPUTE`** — bind group layout для compute должен
  указывать `COMPUTE`, иначе ресурсы не будут доступны шейдеру.
- **Пишут и читают одну и ту же текстуру** — нельзя использовать одну текстуру как input
  и output одновременно. Нужны две отдельные текстуры.

## Итог

Разделённый экран: слева — оригинальная сцена с кубом и полом, справа — размытая версия, обработанная compute-шейдером. Мы познакомились с compute pipeline — новым типом конвейера, который выполняет произвольные вычисления на GPU вне графического конвейера. Compute-шейдер запускается через `dispatch_workgroups`, каждый поток получает свой `global_invocation_id`, а для записи результата используется `texture_storage_2d`. Compute pass записывается в тот же `CommandEncoder` и выполняется GPU в порядке записи — это позволяет комбинировать render и compute проходы произвольным образом.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Увеличить `radius` до 8 — более сильное размытие
- Изменить алгоритм на медианный фильтр — взять медиану вместо среднего
- Добавить второй compute pass: инверсия цветов размытой половины

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/compute)
