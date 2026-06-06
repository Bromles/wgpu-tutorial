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

Для случайных начальных скоростей и времени жизни понадобится крейт `rand`. Добавим его
в зависимости (`Cargo.toml`):

```toml
[dependencies]
framework = { path = "../../../framework" }
wgpu.workspace = true
winit.workspace = true
encase.workspace = true
glam.workspace = true
rand = "0.9" # [!code ++]
```

Обратите внимание: `rand` — единственная зависимость с фиксированной версией, а не workspace-алиас.
Остальные крейты (`wgpu`, `winit`, `encase`, `glam`) управляются через workspace.

## Данные частицы

Каждая частица хранит позицию, скорость и оставшееся время жизни:

```wgsl
struct Particle {
    pos: vec3<f32>,
    _pad0: f32,
    vel: vec3<f32>,
    life: f32,
}
```

`_pad0` между `pos` и `vel` нужна из-за выравнивания WGSL: `vec3<f32>` имеет
выравнивание 16 байт, а поле `pos` занимает только 12 — 4 байта паддинга
компенсируют разницу.

На стороне Rust используется `encase::ShaderType` — библиотека сама добавляет
всё необходимое выравнивание при записи в буфер:

```rust
#[derive(ShaderType, Clone, Copy)]
struct ParticleData {
    pos: Vec3,
    vel: Vec3,
    life: f32,
}
```

Никакого ручного паддинга — `encase` знает правила WGSL и раскладывает
поля так же, как GPU ожидает.

## Storage buffer

До сих пор мы передавали данные в шейдеры через uniform-буферы (`var<uniform>`). Uniform-буферы
ограничены по размеру и доступны только для чтения. Для массива из 2048 частиц нужен другой тип
ресурса — **storage buffer** (`var<storage>`).

Storage buffers:
- не имеют жёсткого ограничения на размер (в отличие от uniform-буферов с лимитом ~16 КБ)
- могут быть доступны для чтения и записи из шейдера (`var<storage, read_write>`)
- используются с `BufferBindingType::Storage` вместо `Uniform`
- работают в compute и vertex/fragment шейдерах

Буфер создаётся с флагом `STORAGE` (доступ из compute-шейдера) и `COPY_DST` (CPU может писать
начальные данные через `queue.write_buffer`). Начальные данные записываются через `encase::StorageBuffer`:

```rust
let mut init_data = encase::StorageBuffer::new(Vec::new());
init_data.write(&initial_particles).unwrap();

let particle_buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    contents: &init_data.into_inner(),
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

CPU каждые ~100 мс перезаписывает часть буфера новыми частицами через `write_buffer`.
Чтобы не перезаписывать одни и те же слоты, оффсет сдвигается после каждого спауна:

```rust
fn spawn_particles(buffer: &Buffer, ctx: &GpuContext, count: u32, offset: u32) {
    let new: Vec<ParticleData> = (0..count)
        .map(|_| {
            let angle = rng.random_range(0.0..TAU);
            let speed = rng.random_range(1.0..4.0);
            ParticleData {
                pos: Vec3::ZERO,
                vel: Vec3::new(
                    angle.cos() * speed * 0.5,
                    rng.random_range(2.0..6.0),
                    angle.sin() * speed * 0.5,
                ),
                life: rng.random_range(1.5..3.5),
            }
        })
        .collect();

    let mut data = encase::StorageBuffer::new(Vec::new());
    data.write(&new).unwrap();
    ctx.queue.write_buffer(
        buffer,
        (offset as u64) * ParticleData::min_size().get(),
        &data.into_inner(),
    );
}
```

`write_buffer` принимает три параметра: целевой буфер, смещение в байтах и данные. Смещение
`(offset as u64) * ParticleData::min_size().get()` — это номер первого слота, умноженный на
размер одной частицы. Например, при `offset = 128` и размере частицы 32 байта запись начнётся
с байта 4096 — середины буфера из 2048 частиц.

Вызов в `render`: оффсет сдвигается на `count` и заворачивается по модулю `NUM_PARTICLES`:

```rust
if spawn_timer > 0.1 {
    spawn_timer = 0.0;
    spawn_particles(&particle_buffer, ctx, 64, spawn_offset);
    spawn_offset = (spawn_offset + 64) % NUM_PARTICLES;
}
```

Модуль `% NUM_PARTICLES` гарантирует, что оффсет никогда не выйдет за пределы буфера.
Каждые 3.2 секунды (2048 / 64 × 0.1 с) оффсет проходит полный круг — весь буфер обновляется.
Мёртвые частицы получают новые данные, а живые не затрагиваются, поскольку каждый спаун
пишет в свою область.

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

Аддитивный blending выбран не случайно. Классический alpha blending (`dst_factor: OneMinusSrcAlpha`)
требует сортировки полупрозрачных объектов от дальних к ближним — иначе дальний объект
перезапишет ближний. Сортировка на GPU невозможна, пришлось бы читать данные на CPU.
Аддитивный blending не зависит от порядка рисования — результат одинаковый независимо от того,
какая частица нарисована первой. Это идеально для систем частиц.

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
- **Ручной паддинг вместо `encase`** — WGSL `vec3<f32>` имеет выравнивание 16, а не 12.
  `encase::ShaderType` автоматически раскладывает поля по правилам WGSL, избавляя
  от ошибок выравнивания. Не нужно считать байты вручную.

## Итог

Фонтан из 2048 частиц, летящих вверх и падающих под действием гравитации. Каждая частица —
полупрозрачный квад (billboard), всегда повёрнутый к камере. Compute-шейдер обновляет физику
всех частиц параллельно: данные живут на GPU в storage buffer и не копируются на CPU.
Пересекающиеся частицы становятся ярче благодаря аддитивному смешиванию — это создаёт
эффект свечения. CPU периодически «респавнит» мёртвые частицы через `write_buffer`.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить `gravity` на 0.5 — частицы будут падать медленнее
- Увеличить количество частиц до 4096 — по-прежнему один dispatch
- Изменить направление начальной скорости — фонтан будет бить в сторону
- Изменить цвета частиц — подставить другие значения в шейдере

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/particles)
