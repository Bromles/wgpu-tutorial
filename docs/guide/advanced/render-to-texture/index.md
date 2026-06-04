---
editLink: false
---

# Render-to-texture и постпроцессинг

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/render-to-texture)

**Что уже должно быть понятно:**

- нормали, направленный свет
- instancing, camera, depth buffer
- bind groups, uniform-буферы

**Что появится в этой главе:**

- offscreen-текстура: рендер сцены не на экран, а в текстуру
- два render pass в одном кадре
- полноэкранный квад без вершинного буфера (`vertex_index`)
- постпроцессинг: оттенки серого, инверсия
- несколько render pipeline в одном приложении

**Итог:** сцена с освещёнными кубами, поверх которой можно переключать эффекты постпроцессинга (1 — норма, 2 — Ч/Б, 3 — инверсия)

---

До сих пор мы рендерили сцену прямо на экран — в `TextureView`, полученный из surface. Но GPU может рисовать
и в обычную текстуру. Это называется **render-to-texture** или offscreen rendering. Результат можно использовать
как входные данные для следующего прохода — например, для постпроцессинга.

## Общая схема

Рендер разбит на два прохода:

1. **Scene pass** — рисуем кубы с освещением в offscreen-текстуру
2. **Post pass** — рисуем полноэкранный квад, сэмплируя offscreen-текстуру и применяя эффект

Оба прохода выполняются в одном `CommandEncoder` — GPU выполнит их последовательно при `queue.submit`.

## Offscreen-текстура

Текстура создаётся с двумя флагами: `RENDER_ATTACHMENT` (чтобы рисовать в неё) и `TEXTURE_BINDING`
(чтобы сэмплировать из неё в следующем проходе):

```rust
let texture = ctx.device.create_texture(&TextureDescriptor {
    label: Some("Offscreen Texture"),
    size: Extent3d { width: size.width, height: size.height, depth_or_array_layers: 1 },
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2,
    format: TextureFormat::Rgba8UnormSrgb,
    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    view_formats: &[],
});
```

Формат совпадает с форматом surface — `Rgba8UnormSrgb`. Размер — текущий размер окна. При resize текстуру
нужно пересоздавать, как и depth-текстуру.

Для offscreen-рендера нужна своя depth-текстура — она не связана с surface.

## Scene pass

Первый render pass рисует в offscreen-текстуру:

```rust
let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
    label: Some("Scene Pass"),
    color_attachments: &[Some(RenderPassColorAttachment {
        view: &self.offscreen_view,    // offscreen, не surface
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(Color::BLACK),
            store: StoreOp::Store,
        },
        depth_slice: None,
    })],
    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
        view: &self.offscreen_depth_view,
        depth_ops: Some(Operations {
            load: LoadOp::Clear(1.0),
            store: StoreOp::Store,
        }),
        stencil_ops: None,
    }),
    // ...
});
```

Всё, что рисуется в этот pass, попадает в offscreen-текстуру, а не на экран. Scene pipeline использует
формат offscreen-текстуры (`Rgba8UnormSrgb`), а не формат surface.

## Полноэкранный квад

Для постпроцессинга нужно нарисовать прямоугольник, закрывающий весь экран, и наложить на него текстуру.
Вместо создания отдельного вершинного буфера можно генерировать вершины прямо в шейдере через
`@builtin(vertex_index)`:

```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(1.0, 0.0),
    );
    var output: VertexOutput;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = uvs[vertex_index];
    return output;
}
```

6 вершин — два треугольника, образующие прямоугольник от (−1, −1) до (1, 1). В clip space это весь экран.
UV-координаты перевёрнуты по Y: `(0, 1)` внизу, `(0, 0)` вверху — текстуры в GPU имеют начало координат
в левом верхнем углу.

Post pipeline не имеет вершинных буферов (`buffers: &[]`) и depth/stencil (`depth_stencil: None`).

## Постпроцессинг

Фрагментный шейдер сэмплирует offscreen-текстуру и применяет эффект в зависимости от `mode`:

```wgsl
let color = textureSample(scene_tex, scene_sampler, input.uv);
if (post.mode == 1u) {
    let gray = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    return vec4<f32>(vec3<f32>(gray), 1.0);
}
if (post.mode == 2u) {
    return vec4<f32>(1.0 - color.rgb, 1.0);
}
return color;
```

- `mode == 0` — без эффекта, текстура как есть
- `mode == 1` — оттенки серого (luminance по стандартным весам BT.601)
- `mode == 2` — инверсия цветов

Режим переключается клавишами 1, 2, 3.

## Bind group для постпроцессинга

Post bind group содержит три ресурса: offscreen-текстуру, сэмплер и uniform с режимом:

```rust
let post_bgl = ctx.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    entries: &[
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture { ... },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer { ... },
            count: None,
        },
    ],
});
```

Сэмплер использует `ClampToEdge` и `FilterMode::Nearest` — для постпроцессинга не нужна фильтрация
и повторение текстуры.

## Post pass

Второй render pass рисует полноэкранный квад на surface:

```rust
let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
    color_attachments: &[Some(RenderPassColorAttachment {
        view,  // surface view
        // ...
    })],
    depth_stencil_attachment: None,
    // ...
});
rpass.set_pipeline(&self.post_pipeline);
rpass.set_bind_group(0, &self.post_bind_group, &[]);
rpass.draw(0..6, 0..1);
```

`draw` без индексов — 6 вершин, 1 экземпляр. Depth/stencil не нужен: квад всегда на переднем плане.

## Что получилось

Сцена из 125 освещённых кубов. Нажмите 1 — обычный вид, 2 — оттенки серого, 3 — инверсия цветов.
Камера перемещается как обычно.

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Уменьшить размер offscreen-текстуры вдвое — увидеть пикселизацию
- Добавить свой эффект: сепия, виньетирование, хроматическая аберрация
- Нарисовать offscreen-текстуру на часть экрана (уменьшить квад)

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/render-to-texture)
