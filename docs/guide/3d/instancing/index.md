---
editLink: false
---

# Instancing

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/instancing)

**Что уже должно быть понятно:**

- камера, view/projection матрицы
- depth buffer, bind groups
- вершинные и индексные буферы

**Что появится в этой главе:**

- instance buffer с model-матрицами
- `VertexStepMode::Instance` — данные на экземпляр, а не на вершину
- один `draw_indexed` для сотни объектов
- разделение uniform: `view_proj` общий, `model` — на экземпляр

**Итог:** сетка 5×5×5 (125) кубов, нарисованных одним вызовом draw

---

В главе про depth buffer мы рисовали три куба тремя bind groups и тремя `draw_indexed`. Каждый куб имел свой
uniform-буфер с MVP-матрицей. Это работает, но при сотнях или тысячах объектов приводит к двум проблемам:
много bind groups (каждая — аллокация на GPU) и много вызовов отрисовки (каждый — накладные расходы на CPU).

Instancing решает обе проблемы: один bind group, один `draw_indexed`, а данные для каждого экземпляра передаются
через отдельный буфер — instance buffer.

## Идея: разделяем MVP

До сих пор мы передавали одну матрицу `mvp = projection × view × model`. При instancing view и projection общие
для всех экземпляров, а model-матрица у каждого своя. Разделим их:

- Uniform-буфер: `view_proj = projection × view` — одни на весь кадр
- Instance buffer: `model` — на каждый экземпляр

Шейдер домножает:

```wgsl
output.position = uniforms.view_proj * model * vec4<f32>(input.position, 1.0);
```

## Instance buffer

Instance buffer похож на вершинный — те же данные, другое значение `step_mode`:

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InstanceData {
    model: [[f32; 4]; 4],
}
```

`[[f32; 4]; 4]` — это 4×4 матрица в column-major порядке. `Mat4::to_cols_array_2d()` из glam возвращает именно его.

Почему не `Mat4` напрямую? `InstanceData` используется с `bytemuck::cast_slice` для записи в GPU-буфер.
Для этого нужен трейт `Pod` (plain old data), а `Mat4` из glam не реализует `Pod`. `[[f32; 4]; 4]` — массив
простых типов, `Pod` реализован автоматически. Для uniform-буферов, где `Pod` не нужен, используется
`encase::ShaderType` — и там `Mat4` работает напрямую.

Описание буфера для pipeline:

```rust
fn desc() -> VertexBufferLayout<'static> {
    VertexBufferLayout {
        array_stride: size_of::<InstanceData>() as BufferAddress,
        step_mode: VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    }
}
```

`VertexStepMode::Instance` — GPU считывает один элемент instance buffer на каждый экземпляр, а не на каждую вершину.
Для 125 экземпляров будет считано 125 элементов, каждый из которых используется для всех 36 вершин куба.

<img src="/diagrams/vertex-instance-step.svg" alt="Паттерн чтения vertex и instance буферов" style="width: 100%;" />

## Данные экземпляров

Генерируем сетку 5×5×5 из 125 кубов:

```rust
const GRID_SIZE: usize = 5;
const NUM_INSTANCES: usize = GRID_SIZE * GRID_SIZE * GRID_SIZE;

fn generate_instances() -> Vec<InstanceData> {
    let mut instances = Vec::with_capacity(NUM_INSTANCES);
    let offset = GRID_SIZE as f32 * 0.5;

    for x in 0..GRID_SIZE {
        for y in 0..GRID_SIZE {
            for z in 0..GRID_SIZE {
                let index = x * GRID_SIZE * GRID_SIZE + y * GRID_SIZE + z;

                let pos = Vec3::new(
                    x as f32 - offset + 0.5,
                    y as f32 - offset + 0.5,
                    z as f32 - offset + 0.5,
                );

                let rotation = Mat4::from_rotation_y(index as f32 * 0.5);
                let scale = 0.8 + 0.4 * ((index as f32) % 5.0) / 5.0;
                let scale_mat = Mat4::from_scale(Vec3::splat(scale));

                let model = Mat4::from_translation(pos) * rotation * scale_mat;
                instances.push(InstanceData {
                    model: model.to_cols_array_2d(),
                });
            }
        }
    }
    instances
}
```

Сдвиг `x - offset + 0.5` центрирует сетку вокруг начала координат. Каждый экземпляр получает:
- **`rotation`** — поворот вокруг Y, угол зависит от линейного индекса (`index * 0.5` радиан), чтобы кубы были
  повёрнуты по-разному
- **`scale`** — размер от 0.8 до 1.2, чередующийся по шаблону: `0.8 + 0.4 * (index % 5) / 5`
- **`scale_mat`** — матрица масштаба из скаляра через `Vec3::splat`

Итоговая model-матрица: `translation * rotation * scale` — порядок стандартный (масштаб → поворот → сдвиг при
чтении справа налево).

Буфер создаётся с флагом `COPY_DST`, чтобы обновлять данные экземпляров каждый кадр:

```rust
let instance_buffer = ctx
    .device
    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instances),
        usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
    });
```

## Uniform с view_proj

Вместо MVP храним только `view_proj`:

```rust
#[derive(ShaderType)]
struct ShaderUniforms {
    view_proj: Mat4,
}
```

Uniform-буфер обновляется каждый кадр — view_proj зависит от позиции камеры и соотношения сторон:

```rust
let view_proj = projection * view_mat;

let mut uniform_data = encase::UniformBuffer::new(Vec::new());
uniform_data.write(&ShaderUniforms { view_proj }).unwrap();
ctx.queue
    .write_buffer(&self.uniform_buffer, 0, &uniform_data.into_inner());
```

## Шейдер

Вершинный шейдер принимает данные экземпляра как дополнительные атрибуты со `location(2)` по `location(5)`:

```wgsl
struct InstanceInput {
    @location(2) model_col0: vec4<f32>,
    @location(3) model_col1: vec4<f32>,
    @location(4) model_col2: vec4<f32>,
    @location(5) model_col3: vec4<f32>,
};
```

WGSL не позволяет передать `mat4x4` через атрибуты напрямую, поэтому передаём по столбцам и собираем:

```wgsl
@vertex
fn vs_main(input: VertexInput, instance: InstanceInput) -> VertexOutput {
    let model = mat4x4<f32>(
        instance.model_col0,
        instance.model_col1,
        instance.model_col2,
        instance.model_col3,
    );
    var output: VertexOutput;
    output.position = uniforms.view_proj * model * vec4<f32>(input.position, 1.0);
    output.uv = input.uv;
    return output;
}
```

Два параметра у `vs_main`: `input` — данные вершины (шаг Vertex), `instance` — данные экземпляра (шаг Instance).
GPU автоматически комбинирует их.

## Pipeline: два vertex buffer layout

Pipeline принимает два описания буферов:

```rust
vertex: VertexState {
    module: &shader_module,
    entry_point: Some("vs_main"),
    buffers: &[Vertex::desc(), InstanceData::desc()],
    compilation_options: PipelineCompilationOptions::default(),
},
```

`Vertex::desc()` — slot 0 (вершины), `InstanceData::desc()` — slot 1 (экземпляры). Slot определяется
порядком в массиве, а не полем `shader_location` — `shader_location` указывает на `@location(N)` в шейдере.

## Отрисовка

Один bind group, один вызов:

```rust
rpass.set_pipeline(&self.pipeline);
rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
rpass.set_vertex_buffer(1, self.instance_buffer.slice(..));
rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
rpass.set_bind_group(0, &self.bind_group, &[]);
rpass.draw_indexed(0..36, 0, 0..NUM_INSTANCES as u32);
```

Третий параметр `draw_indexed` — количество экземпляров. `0..36` — индексы одного куба,
`0..NUM_INSTANCES` — 125 экземпляров. GPU нарисует 36 × 125 = 4500 треугольников за один вызов.

## Что получилось

::: warning Типичные ошибки
- `step_mode: VertexStepMode::Instance` забыли — GPU будет читать instance buffer как вершинный, данные сместятся
- `draw_indexed(0..36, 0, 0..0)` — третий параметр 0 = 0 экземпляров = ничего не нарисуется
- `[[f32; 4]; 4]` в column-major — `to_cols_array_2d()` даёт правильный порядок, `to_rows_array_2d()` — нет
:::

125 кубов в виде сетки 5×5×5. Камера свободно перемещается между ними — WASD, мышь (правая кнопка),
Space/Shift. Все кубы нарисованы одним `draw_indexed`.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Увеличить `GRID_SIZE` до 10 — 1000 кубов, по-прежнему один draw call
- Добавить поворот в model-матрицу: `Mat4::from_translation(pos) * Mat4::from_rotation_y(angle)`
- Изменить масштаб части экземпляров: `Mat4::from_scale(Vec3::splat(0.5))`

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/instancing)
