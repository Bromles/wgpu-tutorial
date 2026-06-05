---
editLink: false
---

# Depth buffer

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/depth-buffer)

**Что уже должно быть понятно:**

- MVP-трансформации, вращающийся куб
- backface culling
- uniform-буферы, bind groups

**Что появится в этой главе:**

- почему несколько объектов рисуются неправильно без depth buffer
- создание depth-текстуры
- depth test в render pipeline и render pass
- несколько bind groups для разных объектов

**Итог:** три куба, стоящие в ряд — ближний корректно перекрывает дальний

---

В прошлой главе мы нарисовали один куб. Если нарисовать несколько, возникает проблема: GPU рисует треугольники
в том порядке, в котором мы их отправляем. Если дальний куб рисуется после ближнего — он нарисуется поверх него.
Результат выглядит так, будто объекты проникают друг в друга.

## Проблема: порядок отрисовки

Три куба стоят в ряд на расстоянии 1.2 друг от друга — достаточно далеко, чтобы не пересекаться.
Но камера смотрит чуть сбоку, и дальние кубы частично перекрываются ближними. Без depth buffer порядок
отрисовки будет определять, какой куб окажется «поверх».

Решение — **depth buffer**: дополнительная текстура, хранящая глубину каждого пикселя. Перед тем как закрасить
пиксель, GPU сравнивает глубину нового фрагмента с уже записанной. Если новый фрагмент дальше — он отбрасывается.

<img src="/diagrams/depth-buffer-comparison.svg" alt="Depth buffer: сравнение без и с тестом глубины" style="width: 100%;" />

## Несколько объектов: по bind group на куб

Каждый куб имеет свою позицию и, соответственно, свою MVP-матрицу. Нам нужно обновлять uniform-буфер для каждого
куба отдельно. Для этого создаём отдельный uniform-буфер и bind group на каждый куб:

```rust
struct CubeDraw {
    uniform_buffer: Buffer,
    bind_group: BindGroup,
    position: Vec3,
}
```

При инициализации:

```rust
let cubes: Vec<CubeDraw> = positions
    .iter()
    .map(|position| {
        let uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
            label: Some("Uniform Buffer"),
            size: ShaderUniforms::min_size().into(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        CubeDraw {
            uniform_buffer,
            bind_group,
            position: *position,
        }
    })
    .collect();
```

Все bind groups используют один и тот же layout — это важно. Pipeline layout описывает формат (какие привязки
и какого типа), а bind group привязывает конкретные ресурсы. Один layout, много групп — это нормальная практика.

## Depth-текстура

Depth buffer — это обычная текстура с особым форматом, используемая как вложение рендера:

```rust
let depth_texture = ctx.device.create_texture(&TextureDescriptor {
    label: Some("Depth Texture"),
    size: Extent3d { width, height, depth_or_array_layers: 1 },
    mip_level_count: 1,
    sample_count: 1,
    dimension: TextureDimension::D2,
    format: TextureFormat::Depth32Float,
    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    view_formats: &[],
});
```

Формат `Depth32Float` — один `f32` на пиксель. Значения от 0.0 (ближняя плоскость) до 1.0 (дальняя). Существует
также `Depth24Plus` — 24 бита на глубину, иногда быстрее на мобильных GPU, но `Depth32Float` точнее и поддерживается
везде. Флаг `TEXTURE_BINDING` добавлен для совместимости с shadow mapping в будущих главах — для чистого depth test
он не нужен.

::: info
**Z-fighting.** Если два объекта находятся на почти одинаковой глубине, depth buffer не может надёжно
определить, какой ближе — на границе возникает мерцание. Решения: немного сдвинуть объекты, увеличить ближнюю
плоскость (не ставить `near = 0.001`), или использовать `Depth32Float` для большей точности.
:::

Размер depth-текстуры совпадает с размером поверхности. При изменении размера окна текстуру нужно пересоздавать —
старая не подходит по размеру.

## Depth test в pipeline

В render pipeline включаем depth test:

```rust
depth_stencil: None,  // [!code --]
depth_stencil: Some(DepthStencilState {  // [!code ++]
    format: TextureFormat::Depth32Float,
    depth_write_enabled: Some(true),
    depth_compare: Some(wgpu::CompareFunction::Less),
    stencil: StencilState::default(),
    bias: DepthBiasState::default(),
}),  // [!code ++]
```

- `format` — должен совпадать с форматом depth-текстуры
- `depth_write_enabled` — записывать глубину новых фрагментов в depth buffer
- `depth_compare: Less` — рисовать фрагмент только если его глубина меньше уже записанной (то есть он ближе)

## Depth attachment в render pass

В render pass привязываем depth-текстуру:

```rust
depth_stencil_attachment: None,  // [!code --]
depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {  // [!code ++]
    view: &self.depth_texture_view,
    depth_ops: Some(Operations {
        load: LoadOp::Clear(1.0),
        store: StoreOp::Store,
    }),
    stencil_ops: None,
}),  // [!code ++]
```

`LoadOp::Clear(1.0)` — начальное значение глубины максимально (1.0 = дальняя плоскость). Поэтому первый
фрагмент в каждом пикселе гарантированно пройдёт тест: любая реальная глубина меньше 1.0.

`StoreOp::Store` сохраняет глубину после прохода. `StoreOp::Discard` сэкономит память — GPU отбросит depth данные
после pass. Используйте `Store`, если depth нужен после render pass (shadow mapping, deferred rendering), и `Discard`
в остальных случаях.

## Отрисовка нескольких кубов

Перед render pass обновляем MVP-матрицы всех кубов:

```rust
for cube in &self.cubes {
    let model = Mat4::from_translation(cube.position)
        * Mat4::from_rotation_y(time + cube.position.x);
    let mvp = projection * view_mat * model;

    let mut uniform_data = encase::UniformBuffer::new(Vec::new());
    uniform_data.write(&ShaderUniforms { mvp }).unwrap();
    ctx.queue
        .write_buffer(&cube.uniform_buffer, 0, &uniform_data.into_inner());
}
```

Каждый куб вращается со своим сдвигом по фазе (`time + cube.position.x`), чтобы вращение выглядело
разнообразно. `write_buffer` ставит команду в очередь — сами записи выполняются при `queue.submit`.

В render pass переключаем bind group перед отрисовкой каждого куба:

```rust
rpass.set_pipeline(&self.pipeline);
rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);

for cube in &self.cubes {
    rpass.set_bind_group(0, &cube.bind_group, &[]);
    rpass.draw_indexed(0..36, 0, 0..1);
}
```

Vertex buffer, index buffer и pipeline общие для всех кубов — меняется только bind group с MVP-матрицей.
`set_bind_group` переключает привязку для последующих вызовов `draw_indexed`.

## Что получилось

::: warning Типичные ошибки
- Формат depth в pipeline и в текстуре **должен совпадать** — иначе runtime ошибка
- `LoadOp::Clear(1.0)` — очищаем в максимум дальности; `Clear(0.0)` сделает все фрагменты «ближайшими»
- Размер depth-текстуры не совпадает с размером surface — мусор или panic
- Depth testing работает только для **непрозрачных** объектов. Полупрозрачные объекты требуют особого подхода —
  мы познакомимся с ним в главе про particles, где используется alpha blending
:::

Три вращающихся куба на разной глубине. Ближний куб корректно перекрывает дальний — depth buffer гарантирует,
что каждый пиксель экрана принадлежит ближайшему объекту.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Убрать `depth_stencil` из pipeline (вернуть `None`) — увидеть артефакты перекрытия
- Изменить `depth_compare` на `Greater` — дальние объекты будут перекрывать ближние
- Поставить `depth_write_enabled: Some(false)` — depth test работает, но глубина не записывается; результат
  зависит от порядка отрисовки

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/depth-buffer)
