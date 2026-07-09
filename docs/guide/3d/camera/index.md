---
editLink: false
---

# Камера

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/camera)

**Что уже должно быть понятно:**

- MVP-трансформации, view-матрица, `look_at_rh`
- depth buffer, несколько bind groups
- uniform-буферы, текстуры, сэмплеры

**Что появится в этой главе:**

- структура камеры с углами yaw и pitch
- вектор направления из углов Эйлера
- управление камерой: мышь для обзора, WASD для перемещения
- movement-векторы: forward и right в горизонтальной плоскости

**Итог:** три куба, по которым можно летать с помощью мыши и клавиатуры

---

До сих пор view-матрица была фиксированной — мы задавали её один раз через `look_at_rh` и не меняли.
В реальном приложении камера двигается: игрок управляет ею мышью и клавиатурой.
Для этого нужна структура, хранящая состояние камеры и пересчитывающая view-матрицу каждый кадр.

## Фреймворк: update и Input

Начиная с этой главы, наш трейт `Example` получает новый метод:

```rust
pub trait Example: 'static {
    fn init(ctx: &GpuContext) -> Self;
    fn resize(&mut self, _ctx: &GpuContext, _new_size: PhysicalSize<u32>) {}
    fn update(&mut self, _ctx: &GpuContext, _dt: Duration, _input: &Input) {}
    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder);
}
```

`update` вызывается каждый кадр **до** `render`. Каркас передаёт:
- `dt` — время, прошедшее с предыдущего кадра (`Duration`). Нужен для frame-rate-independent движения.
- `input` — структура `framework::Input`, хранящая состояние ввода: нажатые клавиши,
  смещение мыши (`mouse_delta`), нажатые кнопки мыши. Каркас обновляет её каждый кадр.

До этой главы `update` был пустым (реализация по умолчанию — ничего не делать). Теперь мы
будем обрабатывать ввод в `update` и использовать результаты в `render`.

## Углы yaw и pitch

Камеру описывают три параметра: позиция и два угла поворота.

- **yaw** — поворот вокруг оси Y (горизонтальный обзор). При `yaw = 0` камера смотрит вдоль −Z.
- **pitch** — наклон вверх/вниз. При `pitch = 0` камера смотрит горизонтально, при положительном — вверх.

```rust
struct Camera {
    position: Vec3,
    yaw: f32,
    pitch: f32,
    speed: f32,
    sensitivity: f32,
}
```

Углы задаются в радианах. `speed` — скорость перемещения (единиц в секунду), `sensitivity` —
чувствительность мыши (радиан на пиксель).

<img src="/diagrams/camera-yaw-pitch.svg" alt="Yaw и pitch камеры: направление из углов Эйлера" style="width: 100%;" />

## Вектор направления

Чтобы построить view-матрицу, нужно знать, куда смотрит камера. Направление вычисляется из yaw и pitch:

```rust
fn direction(&self) -> Vec3 {
    Vec3::new(
        -self.yaw.sin() * self.pitch.cos(),
        self.pitch.sin(),
        -self.yaw.cos() * self.pitch.cos(),
    )
}
```

При `yaw = 0, pitch = 0` направление — `(0, 0, −1)`, то есть вдоль −Z. Yaw поворачивает вектор
в горизонтальной плоскости, pitch поднимает или опускает. Множитель `cos(pitch)` у x- и z-компонент
обеспечивает, что вектор всегда единичной длины.

Формула выводится из сферических координат. В стандартной форме:

$$
x = \sin\theta \cdot \cos\phi, \quad y = \cos\theta, \quad z = \sin\theta \cdot \sin\phi
$$

Но камера по соглашению смотрит вдоль −Z, поэтому x и z инвертированы через минус:
`-yaw.sin()` вместо `yaw.sin()`, `-yaw.cos()` вместо `yaw.cos()`.

## Векторы движения

Для перемещения клавишами WASD нужны горизонтальные векторы — без вертикальной составляющей.
Иначе камера «летела бы вверх» при нажатии W и взгляде вверх.

```rust
fn forward(&self) -> Vec3 {
    Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos())
}

fn right(&self) -> Vec3 {
    Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin())
}
```

Это `direction()` с обнулённой y-компонентой для `forward`, и перпендикуляр к нему для `right`.
Нажатие W двигает вдоль `forward`, D — вдоль `right`.

<img src="/diagrams/camera-vectors.svg" alt="Forward и right векторы камеры относительно угла yaw" style="width: 100%;" />

## Обработка ввода

Обновление камеры вызывается каждый кадр из метода `update`:

```rust
fn update(&mut self, dt: f32, input: &Input) {
    if input.mouse_button_pressed(1) {
        let (dx, dy) = input.mouse_delta();
        self.yaw -= dx as f32 * self.sensitivity;
        self.pitch -= dy as f32 * self.sensitivity;
        self.pitch = self.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    }

    let forward = self.forward();
    let right = self.right();
    let mut velocity = Vec3::ZERO;

    if input.key_pressed(KeyCode::KeyW) { velocity += forward; }
    if input.key_pressed(KeyCode::KeyS) { velocity -= forward; }
    if input.key_pressed(KeyCode::KeyD) { velocity += right; }
    if input.key_pressed(KeyCode::KeyA) { velocity -= right; }
    if input.key_pressed(KeyCode::Space) { velocity.y += 1.0; }
    if input.key_pressed(KeyCode::ShiftLeft) { velocity.y -= 1.0; }

    if velocity.length_squared() > 0.0 {
        self.position += velocity.normalize() * self.speed * dt;
    }
}
```

Мышь вращает камеру только при зажатой правой кнопке (`mouse_button_pressed(1)`).
`mouse_delta` возвращает смещение мыши в пикселях с предыдущего кадра. `Input` — часть учебного
каркаса, хранит состояние клавиш и мыши между кадрами.

`yaw -= dx` (не `+=`) — мышь движется вправо (dx > 0), yaw уменьшается, направление поворачивается
вправо. Аналогично `pitch -= dy` — мышь вверх (dy < 0), pitch увеличивается, камера смотрит вверх.

Pitch ограничен значениями от −89° до +89° — при ±90° вектор направления совпадает с вектором «вверх»,
и `look_to_rh` не может построить корректную матрицу.

Перемещение нормализуется — диагональное движение (например, W+D) будет той же скорости, что и
прямолинейное. `dt` обеспечивает независимость скорости от частоты кадров.

<div class="info custom-block" style="padding-top: 8px">
<p class="custom-block-title">Почему правая кнопка мыши?</p>

Мы используем зажатие правой кнопки мыши для обзора, потому что захват курсора (`set_cursor_grab(true)`)
платформозависим и добавляет сложность. В реальном приложении вы бы захватили курсор и скрыли его:
```rust
window.set_cursor_grab(true).unwrap();
window.set_cursor_visible(false);
```
Для учебного примера правая кнопка — простой и надёжный вариант.

</div>

## View-матрица

View-матрица строится из позиции и направления камеры:

```rust
fn view_matrix(&self) -> Mat4 {
    Mat4::look_to_rh(self.position, self.direction(), Vec3::Y)
}
```

`look_to_rh` принимает позицию (`eye`), направление взгляда (`direction`) и вектор «вверх».
В отличие от `look_at_rh` из предыдущих глав, здесь мы передаём направление, а не целевую точку.

## Интеграция с Example

Метод `update` делегирует обработку ввода камере:

```rust
fn update(&mut self, _ctx: &GpuContext, dt: Duration, input: &Input) {
    self.camera.update(dt.as_secs_f32(), input);
}
```

В `render` view-матрица берётся из камеры вместо фиксированного `look_at_rh`:

```rust
let view_mat = Mat4::look_at_rh(Vec3::new(1.0, 1.5, 4.0), Vec3::ZERO, Vec3::Y);  // [!code --]
let view_mat = self.camera.view_matrix();  // [!code ++]
```

Кубы стоят на месте — их model-матрица содержит только сдвиг:

```rust
let model = Mat4::from_translation(cube.position);
let mvp = projection * view_mat * model;
```

Всё остальное — pipeline, текстуры, depth buffer, bind groups — не изменилось с главы про depth buffer.

## Ground plane

Кубы парят в пустоте — добавим плоскость-«пол» под ними. Это два треугольника (4 вершины, 6 индексов):

```rust
fn ground_vertices() -> Vec<Vertex> {
    vec![
        Vertex { position: [-5.0, 0.0, -5.0], uv: [0.0, 0.0] },
        Vertex { position: [5.0, 0.0, -5.0], uv: [10.0, 0.0] },
        Vertex { position: [5.0, 0.0, 5.0], uv: [10.0, 10.0] },
        Vertex { position: [-5.0, 0.0, 5.0], uv: [0.0, 10.0] },
    ]
}

const GROUND_INDICES: [u16; 6] = [0, 2, 1, 0, 3, 2];
```

Плоскость — квадрат от (-5, 0, -5) до (5, 0, 5). UV от 0 до 10 — текстура повторится 10 раз
(сэмплер использует `AddressMode::Repeat`). Вершины, индексы и uniform-буфер создаются отдельно
от кубов — у плоскости свой bind group, но тот же bind group layout и тот же pipeline:

```rust
let ground_uniform_buffer = ctx.device.create_buffer(&BufferDescriptor {
    label: Some("Ground Uniform Buffer"),
    size: ShaderUniforms::min_size().into(),
    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    mapped_at_creation: false,
});

let ground_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
    label: Some("Ground Bind Group"),
    layout: &bind_group_layout,
    entries: &[
        BindGroupEntry {
            binding: 0,
            resource: ground_uniform_buffer.as_entire_binding(),
        },
        BindGroupEntry {
            binding: 1,
            resource: BindingResource::TextureView(&texture_view),
        },
        BindGroupEntry {
            binding: 2,
            resource: BindingResource::Sampler(&sampler),
        },
    ],
});
```

В `render` плоскость рисуется после кубов. Model-матрица — просто сдвиг вниз на 0.5 (чтобы кубы
стояли на плоскости, а не пересекались с ней):

```rust
{
    let ground_model = Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0));
    let mvp = projection * view_mat * ground_model;
    let mut uniform_data = encase::UniformBuffer::new(Vec::new());
    uniform_data.write(&ShaderUniforms { mvp }).unwrap();
    ctx.queue
        .write_buffer(&self.ground_uniform_buffer, 0, &uniform_data.into_inner());
}
```

Перед отрисовкой переключаем vertex buffer, index buffer и bind group на ground-ресурсы:

```rust
rpass.set_vertex_buffer(0, self.ground_vertex_buffer.slice(..));
rpass.set_index_buffer(self.ground_index_buffer.slice(..), IndexFormat::Uint16);
rpass.set_bind_group(0, &self.ground_bind_group, &[]);
rpass.draw_indexed(0..6, 0, 0..1);
```

Плоскость использует тот же шейдер и pipeline — только данные (буферы и uniform) другие.

## Что получилось

::: warning Типичные ошибки
- Pitch ограничен ±89° — при ±90° вектор `direction()` становится параллелен `up` (Vec3::Y), и `look_to_rh` не может построить корректную матрицу (вырожденное векторное произведение)
- `yaw -= dx` (не `+=`) — если перепутать знак, мышь будет двигаться в обратную сторону
:::

Три куба на плоскости. Зажмите правую кнопку мыши и двигайте мышь для обзора, WASD — для
перемещения, Space и Shift — для движения вверх и вниз.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить `speed` и `sensitivity` — почувствовать разницу в управлении
- Поставить камеру дальше (`Vec3::new(0.0, 5.0, 10.0)`) — увидеть кубы сверху
- Добавить ещё несколько кубов на разных позициях и высотах
- Изменить `sensitivity` на отрицательное значение — инвертировать ось Y

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/3d/camera)
