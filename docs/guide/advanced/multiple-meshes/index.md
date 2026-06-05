---
editLink: false
---

# Несколько мешей

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/multiple-meshes)

**Что уже должно быть понятно:**

- нормали, освещение, текстуры
- camera, depth buffer, bind groups
- vertex и index buffers

**Что появится в этой главе:**

- процедурная генерация геометрии: сфера из параметрических уравнений
- несколько мешей, каждый со своими буферами и bind group
- один uniform на меш: model, normal matrix, цвет
- переключение между мешами в render pass

**Итог:** три сферы с разными текстурами, каждая — отдельный mesh со своим bind group

---

До сих пор все объекты использовали одну и ту же геометрию (куб). В реальных сценах разные объекты
имеют разную форму, текстуры и материалы. Эта глава показывает, как работать с несколькими мешами.

## Процедурная сфера

Сфера генерируется параметрически: два угла (phi, theta) пробегают поверхность:

<img src="/diagrams/sphere-stacks-slices.svg" alt="Генерация сферы: stacks и slices" style="width: 100%;" />

```rust
fn generate_sphere(stacks: u32, slices: u32, radius: f32) -> (Vec<Vertex>, Vec<u16>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for stack in 0..=stacks {
        let phi = PI * stack as f32 / stacks as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for slice in 0..=slices {
            let theta = 2.0 * PI * slice as f32 / slices as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = cos_theta * sin_phi;
            let y = cos_phi;
            let z = sin_theta * sin_phi;

            vertices.push(Vertex {
                position: [x * radius, y * radius, z * radius],
                normal: [x, y, z],
                uv: [slice as f32 / slices as f32, stack as f32 / stacks as f32],
            });
        }
    }
    // indices: каждый quad сетки → 2 треугольника
    (vertices, indices)
}
```

- `stacks` — горизонтальных срезов (от полюса до полюса)
- `slices` — вертикальных сегментов
- Нормаль совпадает с направлением от центра — это свойство сферы

Индексы связывают соседние вершины в треугольники. Каждая ячейка сетки — два треугольника.

## Структура MeshDraw

Каждый mesh хранит свои буферы и bind group:

```rust
struct MeshDraw {
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    index_count: u32,
    bind_group: wgpu::BindGroup,
    model: Mat4,
    normal_matrix: [[f32; 3]; 3],
    uniform_buffer: Buffer,
    base_color: Vec4,
}
```

Один pipeline используется для всех мешей — формат вершин и состояние рендера одинаковые.
Меняются только данные: геометрия (vertex/index buffers) и параметры (uniform + texture).

## Uniform на меш

Шейдер получает model-матрицу, normal matrix, направление света и базовый цвет через один uniform:

```wgsl
struct Uniforms {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    normal_matrix: mat3x3<f32>,
    light_dir: vec3<f32>,
    ambient: f32,
    base_color: vec4<f32>,
};
```

Model и normal matrix индивидуальны для каждого меша — каждый объект может быть сдвинут,
повёрнут, масштабирован. `view_proj` общий, но передаётся через uniform каждого меша.

::: info
`view_proj` дублируется в uniform каждого меша — это не оптимально. Альтернатива — вынести
`view_proj` в отдельный bind group (camera bind group), общий для всех мешей. Мы использовали
этот подход в главе про [instancing](../../3d/instancing/). Здесь каждый меш имеет полный uniform
для простоты — один bind group на меш вместо двух.
:::

## Создание меша

Функция `create_mesh` инкапсулирует создание буферов, uniform и bind group:

```rust
let create_mesh = |vertices, indices, tex_view, color, model| {
    let vertex_buffer = ctx.device.create_buffer_init(...);
    let index_buffer = ctx.device.create_buffer_init(...);
    let uniform_buffer = ctx.device.create_buffer(...);
    let bind_group = ctx.device.create_bind_group(...);
    MeshDraw { vertex_buffer, index_buffer, ..., base_color: color }
};
```

Три сферы — три вызова `create_mesh` с разными позициями и текстурами:

```rust
let meshes = vec![
    create_mesh(&sphere.0, &sphere.1, &tex1, white, Mat4::from_translation(Vec3::new(-3.0, 0.0, 0.0))),
    create_mesh(&sphere.0, &sphere.1, &tex2, white, Mat4::from_translation(Vec3::new(0.0, 0.0, 0.0))),
    create_mesh(&sphere.0, &sphere.1, &tex3, white, Mat4::from_translation(Vec3::new(3.0, 0.0, 0.0))),
];
```

Все три используют одну и ту же геометрию (sphere), но могли бы использовать разную — например,
куб + сфера + плоскость.

## Отрисовка

В render pass переключаемся между мешами:

```rust
rpass.set_pipeline(&self.pipeline);
for mesh in &self.meshes {
    rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    rpass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint16);
    rpass.set_bind_group(0, &mesh.bind_group, &[]);
    rpass.draw_indexed(0..mesh.index_count, 0, 0..1);
}
```

Для каждого меша устанавливаем свои буферы и bind group. Pipeline общий.

## Что получилось

## Что получилось

::: warning Типичные ошибки
- Каждый mesh должен иметь свой bind group — общий bind group не работает при разных uniform-данных
- `index_count` в `draw_indexed` должен соответствовать реальному количеству индексов — иначе мусор или crash
- Sphere generation: `(stacks+1) × (slices+1)` вершин, не `stacks × slices` — каждый стек имеет +1 вершину для замыкания
:::

Три сферы с разными текстурами, стоящие в ряд. Камера свободно перемещается.

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить `base_color` для одной из сфер — tint текстуры цветом
- Добавить куб как четвёртый mesh — разные типы геометрии в одной сцене
- Изменить `stacks` и `slices` — увидеть, как меняется детализация сферы
- Добавить поворот в model-матрицу: `Mat4::from_rotation_y(angle)`

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/multiple-meshes)
