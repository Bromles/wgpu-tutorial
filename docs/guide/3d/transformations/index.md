---
editLink: false
---

# Трансформации MVP

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/transformations)

**Что уже должно быть понятно:**

- векторы, матрицы, умножение матриц ([Векторы и матрицы](/guide/math/vectors-matrices/))
- путь вершины через системы координат ([Система координат WebGPU](/guide/math/coordinate-system/))
- uniform-буферы и bind groups
- вершинные и индексные буферы

**Что появится в этой главе:**

- model, view, projection матрицы на практике
- 3D-геометрия: куб из 8 вершин и 36 индексов
- backface culling
- обновление uniform-буфера каждый кадр

**Итог:** три куба с разными трансформациями и отсечением задних граней

---

До сих пор мы работали в 2D — координаты вершин от -1 до 1 попадали на экран напрямую. В реальных приложениях объекты
находятся в трёхмерном мире: они сдвинуты, повёрнуты, расположены на разной глубине, а камера смотрит на них под
определённым углом. Чтобы превратить 3D-координаты в 2D-позиции на экране, используются три матрицы — model, view и
projection. Вместе они называются **MVP**.

Для работы с матрицами добавим крейт `glam` в зависимости (`Cargo.toml`):

```toml
[dependencies]
framework = { path = "../../../framework" }
wgpu.workspace = true
winit.workspace = true
bytemuck.workspace = true
encase.workspace = true
glam.workspace = true # [!code ++]
```

`glam` — библиотека линейной алгебры для графики. Предоставляет `Vec3`, `Mat4`, операции с матрицами и кватернионами.
Интегрирована с `encase` через фичу `encase`/`glam` — типы `glam` можно напрямую использовать в `#[derive(ShaderType)]`
структурах.

## Три матрицы

Каждая вершина умножается на три матрицы последовательно:

```
clip_position = projection × view × model × vertex_position
```

Порядок справа налево: сначала model, потом view, потом projection. Поскольку умножение матриц
ассоциативно ($(A \cdot B) \cdot C = A \cdot (B \cdot C)$), мы можем объединить все три в одну — `mvp` — и передать её в шейдер:

```rust
let mvp = projection * view * model;
```

### Model matrix

Переводит координаты из пространства объекта (object space) в мировые координаты (world space). Сдвигает, поворачивает
и масштабирует объект:

```rust
let model = Mat4::from_rotation_y(time);
```

В нашем примере куб просто вращается вокруг оси Y. В общем случае model-матрица — это композиция
сдвига, поворота и масштаба:

```rust
let model = Mat4::from_scale(Vec3::splat(0.5))
    * Mat4::from_rotation_y(angle)
    * Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0));
```

### View matrix

Перестраивает мировые координаты так, чтобы камера оказалась в начале координат и смотрела вдоль -Z:

```rust
let view = Mat4::look_at_rh(
    Vec3::new(4.0, 3.0, 4.0),    // позиция камеры (eye)
    Vec3::ZERO,                   // точка, куда смотрит камера (target)
    Vec3::Y,                      // вектор «вверх»
);
```

Камера расположена в точке (4, 3, 4) и смотрит в начало координат — туда, где находятся кубы. В отличие от
`look_to_rh`, принимающего направление взгляда, `look_at_rh` принимает целевую точку — удобнее, когда мы знаем,
куда хотим смотреть.

### Projection matrix

Проецирует 3D-пространство на 2D-плоскость с учётом перспективы — далёкие объекты выглядят меньше:

```rust
let projection = Mat4::perspective_rh(
    FRAC_PI_4,  // угол обзора (FOV) — 45°
    aspect,     // соотношение сторон окна
    0.1,        // ближняя плоскость отсечения
    100.0,      // дальняя плоскость отсечения
);
```

Объекты ближе 0.1 или дальше 100.0 от камеры не будут видны — они отсекаются.

## Геометрия куба

Куб — это 6 граней, каждая из которых состоит из 2 треугольников. Всего 12 треугольников и 36 индексов.

<img src="/diagrams/wireframe-cube.svg" alt="Куб с пронумерованными вершинами" style="width: 100%;" />

8 вершин — по одной на каждый угол куба, с уникальным цветом:

```rust
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5,  0.5], color: [1.0, 0.0, 0.0] }, // 0 — передний нижний левый
    Vertex { position: [ 0.5, -0.5,  0.5], color: [0.0, 1.0, 0.0] }, // 1 — передний нижний правый
    Vertex { position: [ 0.5,  0.5,  0.5], color: [0.0, 0.0, 1.0] }, // 2 — передний верхний правый
    Vertex { position: [-0.5,  0.5,  0.5], color: [1.0, 1.0, 0.0] }, // 3 — передний верхний левый
    Vertex { position: [-0.5, -0.5, -0.5], color: [1.0, 0.0, 1.0] }, // 4 — задний нижний левый
    Vertex { position: [ 0.5, -0.5, -0.5], color: [0.0, 1.0, 1.0] }, // 5 — задний нижний правый
    Vertex { position: [ 0.5,  0.5, -0.5], color: [0.5, 0.5, 0.5] }, // 6 — задний верхний правый
    Vertex { position: [-0.5,  0.5, -0.5], color: [1.0, 0.5, 0.0] }, // 7 — задний верхний левый
];
```

36 индексов — по 6 на грань (2 треугольника × 3 вершины). Порядок обхода — против часовой стрелки (CCW),
чтобы передняя грань была обращена к наблюдателю:

```rust
const INDICES: &[u16] = &[
    0, 1, 2, 2, 3, 0, // передняя грань
    1, 5, 6, 6, 2, 1, // правая грань
    5, 4, 7, 7, 6, 5, // задняя грань
    4, 0, 3, 3, 7, 4, // левая грань
    3, 2, 6, 6, 7, 3, // верхняя грань
    4, 5, 1, 1, 0, 4, // нижняя грань
];
```

Структура вершины теперь содержит 3D-позицию вместо 2D:

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],  // [!code --]
    position: [f32; 3],  // [!code ++]
    color: [f32; 3],
}
```

## Uniform с MVP-матрицей

MVP-матрица передаётся в вершинный шейдер через uniform-буфер:

```rust
#[derive(ShaderType)]
struct ShaderUniforms {
    mvp: Mat4,
}
```

`Mat4` из glam реализует `encase::ShaderType`, поэтому сериализация автоматическая — 64 байта (16 × f32),
выравнивание корректное.

Bind group layout и bind group создаются так же, как в [главе про uniform-буферы](/guide/gpu-data-model/uniform-bind-groups/). Единственное отличие —
`visibility: ShaderStages::VERTEX` вместо `FRAGMENT`, потому что матрицу использует вершинный шейдер:

```rust
BindGroupLayoutEntry {
    binding: 0,
    visibility: ShaderStages::FRAGMENT,  // [!code --]
    visibility: ShaderStages::VERTEX,    // [!code ++]
    ty: BindingType::Buffer { ... },
    count: None,
},
```

## Шейдер

Вершинный шейдер умножает позицию вершины на MVP-матрицу:

```wgsl
struct Uniforms {
    mvp: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = uniforms.mvp * vec4<f32>(input.position, 1.0);
    output.color = input.color;
    return output;
}
```

Позиция вершины — `vec3`, но матрица 4×4 требует `vec4`. Добавляем `w = 1.0`, чтобы показать что это точка
(для направлений `w = 0`).
Результат — clip-space позиция, которую GPU преобразует в экранные координаты.

Фрагментный шейдер не изменился — просто возвращает цвет:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
```

## Несколько моделей: массив uniform-буферов

Вместо одного куба нарисуем три — каждый со своей model-матрицей. Для этого создаём по отдельному
uniform-буферу и bind group на каждую модель:

```rust
struct RotatingCube {
    pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    uniform_buffers: [Buffer; 3],
    bind_groups: [BindGroup; 3],
    start_time: Instant,
}
```

При инициализации — три uniform-буфера и три bind group через `std::array::from_fn`:

```rust
let uniform_size = ShaderUniforms::min_size();

let uniform_buffers: [Buffer; 3] = std::array::from_fn(|i| {
    ctx.device.create_buffer(&BufferDescriptor {
        label: Some(&format!("Uniform Buffer {i}")),
        size: uniform_size.into(),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
});

let bind_groups: [BindGroup; 3] = std::array::from_fn(|i| {
    ctx.device.create_bind_group(&BindGroupDescriptor {
        label: Some(&format!("Bind Group {i}")),
        layout: &bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: uniform_buffers[i].as_entire_binding(),
        }],
    })
});
```

Все bind groups используют один и тот же layout — это важно. Pipeline layout описывает формат (какие привязки
и какого типа), а bind group привязывает конкретные ресурсы. Один layout, много групп — это нормальная практика.

## Обновление матриц каждый кадр

Каждый кадр пересчитываем MVP для каждой модели. View-projection вычисляется один раз, а model-матрицы —
в цикле:

```rust
let time = self.start_time.elapsed().as_secs_f32();
let aspect = ctx.surface_config.width as f32 / ctx.surface_config.height as f32;

let projection = Mat4::perspective_rh(FRAC_PI_4, aspect, 0.1, 100.0);
let view_mat = Mat4::look_at_rh(
    Vec3::new(4.0, 3.0, 4.0),
    Vec3::ZERO,
    Vec3::Y,
);
let vp = projection * view_mat;

let models = [
    Mat4::from_rotation_y(time),
    Mat4::from_translation(Vec3::new(
        2.0 * time.cos(),
        0.0,
        2.0 * time.sin(),
    )) * Mat4::from_rotation_y(time * 2.0),
    Mat4::from_translation(Vec3::new(-1.5, 1.0, -1.0))
        * Mat4::from_scale(Vec3::splat(0.5))
        * Mat4::from_rotation_x(time * 1.5),
];
```

Три model-матрицы:
- **Вращающийся куб** — `from_rotation_y(time)`, стоит в начале координат
- **Орбитальный куб** — `from_translation` по кругу радиуса 2 с удвоенной скоростью вращения
- **Маленький куб** — сдвинут влево-вверх-назад, уменьшен в 2 раза, вращается вокруг X

`vp` — это `projection * view_mat`, предвычисленная один раз. В цикле домножаем на model:

```rust
for (i, model) in models.iter().enumerate() {
    let mvp = vp * *model;
    let mut uniform_data = encase::UniformBuffer::new(Vec::new());
    uniform_data.write(&ShaderUniforms { mvp }).unwrap();
    ctx.queue
        .write_buffer(&self.uniform_buffers[i], 0, &uniform_data.into_inner());

    rpass.set_bind_group(0, &self.bind_groups[i], &[]);
    rpass.draw_indexed(0..36, 0, 0..1);
}
```

Перед отрисовкой каждого куба переключаем bind group — `set_bind_group` меняет привязку для последующих
вызовов `draw_indexed`. Vertex buffer, index buffer и pipeline общие для всех.

Projection зависит от соотношения сторон окна и пересчитывается каждый кадр — при resize изображение не будет
растянуто. Для камеры используется `look_at_rh` — он принимает позицию глаза и **целевую точку**, на которую
смотрим. Есть и другой вариант:

```rust
// Целевая точка — удобно для статичных сцен
let view = Mat4::look_at_rh(eye, target, up);

// Направление взгляда — удобно для свободной камеры (глава «Камера»)
let view = Mat4::look_to_rh(eye, direction, up);
```

Обе функции создают одну и ту же view-матрицу, отличаются только способом задания ориентации камеры.

## Backface culling

В этом примере мы впервые включаем отсечение задних граней:

```rust
primitive: PrimitiveState {
    cull_mode: Some(wgpu::Face::Back),
    ..Default::default()
},
```

Задние грани (обход вершин по часовой стрелке) отбрасываются до растеризации. Для замкнутого объекта вроде куба
они не видны — это экономит GPU-время. Без culling задние грани всё равно были бы нарисованы и затем перекрыты
передними благодаря depth buffer, но это лишняя работа.

GPU определяет переднюю грань через векторное произведение (cross product) двух рёбер треугольника в screen space.
Если результат указывает на наблюдателя — грань передняя (CCW), если от наблюдателя — задняя (CW).

<div class="warning custom-block" style="padding-top: 8px">
<p class="custom-block-title">Куб выглядит «вывернутым наизнанку»</p>

Пока у нас нет depth buffer (буфера глубины), поэтому задние грани, оказавшиеся ближе к камере, рисуются поверх
передних. Из-за этого при определённых углах поворота куб может выглядеть странно. Это нормально — depth buffer будет
добавлен в следующей главе, и тогда отображение станет корректным.

</div>

<div class="info custom-block" style="padding-top: 8px">
<p class="custom-block-title">Почему у куба 24 вершины, а не 8?</p>

Наш куб использует 8 вершин — по одной на угол. Это работает, потому что мы не используем нормали (векторы,
определяющие направление поверхности). Когда мы дойдём до главы про освещение, каждой грани потребуется своя
нормаль, а нормаль хранится в данных вершины. На каждом углу куба сходятся 3 грани с разными нормалями —
поэтому каждый угол будет существовать в трёх экземплярах. 8 углов × 3 грани = 24 вершины.

</div>

## Что получилось

::: warning Типичные ошибки
- `near = 0` в `perspective_rh` вызовет panic или деление на ноль — ближняя плоскость всегда > 0
- Порядок умножения матриц: `projection * view * model` — не `model * view * projection`
- `look_at_rh` panics если eye = target — целевая точка должна отличаться от позиции камеры
:::

Три куба с разными трансформациями (вращение, орбита, масштаб). Backface culling убирает задние грани, цвета граней — интерполяция между вершинами.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Убрать `cull_mode: Some(Face::Back)` — увидеть задние грани
- Изменить позицию камеры в `look_at_rh` — посмотреть с другой стороны
- Добавить поворот вокруг X или Z: `Mat4::from_rotation_x(time * 0.7)`
- Уменьшить дальнюю плоскость отсечения до 1.0 — куб будет частично обрезан

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/transformations)
