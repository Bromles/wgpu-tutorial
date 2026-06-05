---
editLink: false
---

# Particles

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/particles)

**Что уже должно быть понятно:**

- compute passes, storage buffers, dispatch
- instancing, камера, vertex/fragment шейдеры

**Что появится в этой главе:**

- storage buffer с `var<storage, read_write>` — массив частиц
- compute-шейдер для физики: гравитация, скорость, время жизни
- billboard-рендеринг — квады, всегда повёрнутые к камере
- alpha blending — частицы полупрозрачные

**Итог:** фонтан из 2048 частиц, летящих вверх и падающих под действием гравитации

---

Compute-шейдеры идеально подходят для систем частиц: каждый поток обрабатывает одну частицу,
все 2048 частиц обновляются параллельно. GPU читает и пишет в storage buffer — данные
остаются в видеопамяти, копирования между CPU и GPU нет.

## Данные частицы

Каждая частица хранит позицию, скорость и оставшееся время жизни:

```wgsl
struct Particle {
    pos: vec3<f32>,
    vel: vec3<f32>,
    life: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
```

Паддинг до 32 байт (8 × `f32`) нужен для выравнивания в storage buffer. WGSL выравнивает
структуры так же, как uniform: каждая структура начинается на границе 16 байт.

На стороне Rust:

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ParticleData {
    pos: [f32; 3],
    vel: [f32; 3],
    life: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}
```

## Storage buffer

Буфер создаётся с флагом `STORAGE` (доступ из compute-шейдера) и `COPY_DST` (CPU может писать
начальные данные через `queue.write_buffer`):

```rust
let particle_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: bytemuck::cast_slice(&initial_particles),
    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
});
```

Bind group layout для read-write storage:

```rust
BindingType::Buffer {
    ty: BufferBindingType::Storage { read_only: false },
    ...
}
```

`read_only: false` соответствует `var<storage, read_write>` в WGSL.

## Compute: симуляция

Каждый вызов compute-шейдера обновляет одну частицу:

```wgsl
@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let idx = id.x;
    if (idx >= arrayLength(&particles)) { return; }

    var p = particles[idx];
    p.life -= params.dt;
    if (p.life <= 0.0) {
        // Мёртвая частица — обнуляем
        p.pos = vec3<f32>(0.0);
        p.vel = vec3<f32>(0.0);
        p.life = 0.0;
        particles[idx] = p;
        return;
    }
    p.vel.y -= params.gravity * params.dt;
    p.pos += p.vel * params.dt;
    particles[idx] = p;
}
```

Гравитация уменьшает `vel.y` каждый кадр — частицы летят по параболе. Мёртвые частицы
обнуляются, а CPU периодически «респавнит» их новыми данными.

Запуск: `dispatch_workgroups(NUM_PARTICLES / 256, 1, 1)` = 8 workgroups по 256 потоков.

## Респаун частиц

CPU каждые ~100 мс перезаписывает часть буфера новыми частицами через `write_buffer`:

```rust
fn spawn_particles(buffer: &Buffer, ctx: &GpuContext, count: u32) {
    let mut rng = rand::rng();
    let mut new = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let angle = rng.random_range(0.0..TAU);
        let speed = rng.random_range(1.0..4.0);
        new.push(ParticleData {
            pos: [0.0, 0.0, 0.0],
            vel: [angle.cos() * speed * 0.5, rng.random_range(2.0..6.0), angle.sin() * speed * 0.5],
            life: rng.random_range(1.5..3.5),
            ...
        });
    }
    ctx.queue.write_buffer(buffer, 0, bytemuck::cast_slice(&new));
}
```

Это записывает первые `count` частиц буфера — «живые» частицы при этом не затрагиваются,
поскольку compute-шейдер использует тот же буфер. Порядок выполнения гарантирует,
что compute pass завершится до следующего кадра.

## Billboard-рендеринг

Каждая частица — маленький квад (billboard), всегда повёрнутый к камере. Вместо
отдельного вершинного буфера квады генерируются прямо в вершинном шейдере:

```wgsl
@vertex
fn vs_main(@builtin(vertex_index) vid: u32, @builtin(instance_index) iid: u32) -> VertexOutput {
    let p = particles[iid];
    let quad_pos = array<vec2<f32>, 6>( /* 6 вершин двух треугольников */ );
    let offset = quad_pos[vid % 6u];
    let world_pos = p.pos
        + camera.camera_right.xyz * offset.x
        + camera.camera_up.xyz * offset.y;
    ...
}
```

`instance_index` — индекс частицы в storage buffer. `vertex_index` — 0–5 для шести вершин
квадрата. Позиция каждой вершины = центр частицы + смещение вдоль векторов `right` и `up` камеры.

Это даёт «billboard» эффект: квад всегда параллелен экрану, независимо от угла обзора.

## Alpha blending

Частицы полупрозрачные — по мере уменьшения `life` альфа уменьшается:

```wgsl
let t = clamp(p.life / 3.0, 0.0, 1.0);
output.color = vec4<f32>(1.0, 0.6 * t + 0.2, 0.1, t);
```

Для корректного отображения включён alpha blending:

```rust
blend: Some(BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::One,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
}),
```

`dst_factor: One` для color означает аддитивное смешивание — пересекающиеся частицы
становятся ярче, что создаёт эффект свечения.

## Instancing

Вызов отрисовки: `rpass.draw(0..6, 0..NUM_PARTICLES)` — 6 вершин × 2048 экземпляров.
GPU вызывает вершинный шейдер 12 288 раз — каждый вызов получает `vertex_index` (0–5)
и `instance_index` (0–2047).

Никакого вершинного буфера не требуется — все данные берутся из `@builtin` и storage buffer.

## Типичные ошибки

- **`read_only: true` для read-write storage** — compute-шейдер не сможет писать в буфер.
  Для `var<storage, read_write>` нужно `read_only: false`.
- **Забыли bounds check в compute** — `if (idx >= arrayLength(&particles))` обязателен,
  иначе последний workgroup может выйти за границы.
- **Пишут все частицы каждый кадр** — `write_buffer` копирует данные с CPU на GPU.
  Лучше писать только мёртвые частицы, а не весь буфер.
- **Отрисовка мёртвых частиц** — частицы с `life <= 0` записываются с `alpha = 0`,
  но всё равно отправляются через конвейер. Можно оптимизировать через indirect draw,
  но для демо это допустимо.
- **Неправильный workgroup_size** — `@workgroup_size(256)` должен делить количество частиц
  без остатка, или bounds check в шейдере обязателен.

## Итог

- **Storage buffer** (`var<storage, read_write>`) — GPU-массив данных, доступный compute-шейдеру
  для чтения и записи.
- **Compute-шейдер** обновляет физику каждой частицы параллельно — все 2048 за один dispatch.
- **Billboard** — квад, ориентированный по векторам камеры (`right`, `up`), без отдельного
  вершинного буфера.
- **Alpha blending** — аддитивное смешивание создаёт эффект свечения при пересечении частиц.
- CPU «респавнит» мёртвые частицы через `write_buffer`, данные живут на GPU.

Система частиц — классический пример, где compute-шейдеры дают значительное преимущество:
данные не покидают GPU, и тысячи частиц обновляются параллельно.
