---
title: Управляемая камера
description: "Клавиатура и мышь: накопление состояния, yaw/pitch, dt, захват курсора и потеря фокуса."
prev:
  text: "Ориентация и видимость"
  link: ../depth-culling/
next:
  text: "Поверхность и UV-швы"
  link: ../cube-uv/
---

# Управляемая камера

::: info Только native
Сцена — снимок [главы «Ориентация и видимость»](../depth-culling/): два пересекающихся треугольника и depth; сверху добавлен пол из [главы «Проекция и путь на экран»](../ortho-perspective/). Новое — не геометрия, а управление.
:::

Кадры больше не одинаковые: позу камеры двигают клавиатура и мышь.
Вопрос главы — как связать **native input** с позой так, чтобы скорость не зависела от частоты кадров, а мышь вела себя предсказуемо.

**Управление:** `WASD` — полёт, `Space`/`Shift` — вверх/вниз, мышь (после захвата кликом) — поворот, `R` — сброс позы, `Alt` — отпустить курсор.
Скорость `2 м/с`, поворот — фиксированная чувствительность в радианах на единицу mouse delta.

## События → состояние → один update

События приходят не в такт кадрам: три движения мыши между кадрами должны дать один поворот на сумму, а удерживаемая клавиша — движение, пропорциональное **времени кадра**, а не числу событий.
Оболочка пересылает raw-события окна и устройства; она ничего не знает о камерах.
Сэмпл превращает поток событий в два накопителя:

- `pressed` — множество удерживаемых клавиш (`Pressed` добавляет, `Released` убирает);
- `pending_delta` внутри камеры — сумма mouse delta со времён прошлого кадра.

Один раз за кадр, в `draw`, накопители потребляет единственный `update`:

```rust
// One update per frame consumes the accumulated input state.
self.camera.update(dt, &self.pressed);
```

`dt` берётся из пары `Instant`, как в [главе «Точки и движение»](../triangle-motion/).

Мышь при этом на `dt` **не** умножается: delta — уже готовое «насколько подвинули с прошлого раза», единица у неё — перемещение, а не скорость.

## Взгляд из углов: yaw и pitch

Поза — `position` и два угла вместо трёх векторов [главы «Камера как система координат»](../look-at/).
Выведем `forward` из сферических координат: старт — взгляд вдоль `−Z` (при `yaw = 0, pitch = 0`); `pitch` наклоняет вокруг X (положительный — вверх); `yaw` поворачивает в горизонтальной плоскости (положительный — вправо, к `+X`):

```text
forward = (sin(yaw)·cos(pitch),  sin(pitch),  −cos(yaw)·cos(pitch))
```

Проверки: `yaw = 0, pitch = 0` → `(0, 0, −1)`; `yaw = π/2` → `(1, 0, 0)` — повернулись вправо; `pitch = π/2` → `(0, 1, 0)` — вверх.

```rust
pub fn forward(&self) -> Vec3 {
    Vec3::new(
        self.yaw.sin() * self.pitch.cos(),
        self.pitch.sin(),
        -self.yaw.cos() * self.pitch.cos(),
    )
}
```

`right` строится тем же cross-произведением из [главы «Три измерения и базис»](../basis3d/) — `forward × world_up`; при любом pitch он горизонтален:

```rust
pub fn right(&self) -> Vec3 {
    self.forward().cross(Vec3::Y).normalize()
}
```

`pitch` зажат в `±(π/2 − 0.01)`: у вертикали `forward` параллелен `world_up`, cross вырождается в ноль, и базис взгляда построить нельзя — clamp держит нас на шаг от вырождения, а не на нём.
Матрица вида собирается знакомым конструктором [главы «Камера как система координат»](../look-at/) — мы уже проверяли, что он совпадает с базисом, собранным вручную:

```rust
pub fn view_matrix(&self) -> Mat4 {
    glam::camera::rh::view::look_at_mat4(self.position, self.position + self.forward(), Vec3::Y)
}
```

## Движение: 2 м/с и нормализация диагонали

`update` применяет мышь (без `dt`), затем интегрирует движение (с `dt`):

```rust
pub fn update(&mut self, dt: f32, keys: &HashSet<KeyCode>) {
    self.yaw += self.pending_delta.0 * SENSITIVITY;
    self.pitch = (self.pitch - self.pending_delta.1 * SENSITIVITY)
        .clamp(-PITCH_LIMIT, PITCH_LIMIT);
    self.pending_delta = (0.0, 0.0);

    let forward = self.forward();
    let right = self.right();
    let mut direction = Vec3::ZERO;
    if keys.contains(&KeyCode::KeyW) {
        direction += forward;
    }
    if keys.contains(&KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.contains(&KeyCode::KeyD) {
        direction += right;
    }
    if keys.contains(&KeyCode::KeyA) {
        direction -= right;
    }
    if keys.contains(&KeyCode::Space) {
        direction += Vec3::Y;
    }
    if keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::ShiftRight) {
        direction -= Vec3::Y;
    }
    if direction.length_squared() > 0.0 {
        self.position += direction.normalize() * SPEED * dt;
    }
}
```

Ключевая строка — `direction.normalize()`: сумма `W + D` имеет длину `√2`, без нормализации диагональ была бы в `√2` раза быстрее прямых направлений.
Контроль — юнит-тест: секунда полёта вперёд при 30 update по `1/30 с` и при 120 по `1/120 с` даёт одинаковые 2 м.

## Мышь: позиции против относительного движения

Поворот камеры определяется тем, **насколько подвинули** мышь, и у winit для этого два разных потока событий.

- `WindowEvent::CursorMoved` — **позиция** курсора в окне. Пока курсор свободен, разность соседних позиций действительно равна перемещению. Но позиция проходит через путь указителя ОС: ускорение, прилипание к краям экрана, а после захвата (`CursorGrabMode::Locked`) — клампинг и телепорты скрытого курсора, зависящие от платформы. Доверять этой разности как дельте нельзя.
- `DeviceEvent::MouseMotion` — **относительное движение** самого устройства: «датчик мыши сдвинулся на (dx, dy)». Этот поток не зависит ни от позиции курсора, ни от краёв экрана и продолжает течь, пока курсор захвачен и скрыт. Его и копим в камере:

```rust
fn device_event(&mut self, event: &DeviceEvent) {
    // Relative motion of the mouse device, not a cursor position: it
    // keeps flowing while the cursor is locked and hidden, and it does
    // not pass through the OS pointer-speed curve.
    if self.cursor_locked
        && let DeviceEvent::MouseMotion { delta } = event
    {
        self.camera.add_mouse_delta(delta.0 as f32, delta.1 as f32);
    }
}
```

Пока курсор свободен, дельты не копятся: иначе камера крутилась бы при простом наведении.
Единица дельты — счетчик устройства (не пиксель и не метр); чувствительность `SENSITIVITY` связывает её с радианами.

Режим захвата выбирается независимо от источника данных: `Locked` прячет курсор и отвязывает его от краёв экрана (наш выбор), `Confined` оставил бы его видимым внутри окна — тогда позиции снова осмысленны, но относительный поток всё равно проще.
Захват выполняется по первому клику:

```rust
fn grab_cursor(&mut self, window: &Window) {
    match window.set_cursor_grab(CursorGrabMode::Locked) {
        Ok(()) => {
            window.set_cursor_visible(false);
            self.cursor_locked = true;
        }
        Err(error) => {
            tracing::warn!(%error, "cursor lock unavailable; steering stays on the keyboard");
        }
    }
}
```

Захват может не поддерживаться платформой — неуспех обрабатываем явно (warn), сцена остаётся управляемой с клавиатуры.
`Alt` и потеря фокуса отпускают курсор; `Focused(false)` дополнительно чистит клавиши и накопленные дельты — удержанная до переключения окна клавиша не должна «догонять» время:

```rust
WindowEvent::Focused(false) => {
    self.release_cursor(window);
    self.pressed.clear();
}
```

Одноразовые действия не любят автоповтор: ОС шлёт `Pressed` много раз подряд, поэтому `R` и `Alt` срабатывают только когда `insert` действительно добавил клавишу.

Юнит-тесты ниже проверяют накопление и применение дельт — арифметику камеры. А вот приходят ли дельты из ОС в ответ на реальное движение руки, проверяется интерактивно: захватите курсор и повращайте камеру до потери и возврата фокуса.

## Снимок

`draw` связывает всё: `dt` из `Instant`, один `update`, пересоздание локальной depth-текстуры по `pending_size` (размер приходит контрактом `resize` ещё до первого кадра — как в [главе «Ориентация и видимость»](../depth-culling/)), и один uniform на 64 байта:

```rust
// The depth attachment must match the frame. The size comes from
// the framework's resize contract (guaranteed before the first draw),
// so a pending size (re)creates the attachment here, in draw.
if let Some(size) = self.pending_size.take()
    && self.depth_size != (size.width, size.height)
{
    self.recreate_depth(gpu, size.width, size.height);
}

let view_proj = self.projection * self.camera.view_matrix();
```

Проекция — `glam::camera::rh::proj::directx::perspective(60°, aspect, 0.1, 50)`; `aspect` пересчитывается в `recreate_depth` вместе с размером.
Шейдер — один uniform и одна матрица:

```wgsl
@group(0) @binding(0) var<uniform> view_proj: mat4x4<f32>;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = view_proj * input.position;
    output.color = input.color.rgb;
    return output;
}
```

Пересекающиеся треугольники при облёте меняют цвет у линии пересечения — depth из [главы «Ориентация и видимость»](../depth-culling/) сортирует их сам, порядок draw не меняется.

![Стартовый кадр camera-fly: пол и два пересекающихся треугольника со стартовой позы камеры.](/results/camera-fly.png)

Стартовая поза: глаз в `(0, 1.5, 4)` с наклоном вниз, `yaw = 0`. Дальше кадр в ваших руках — `WASD`, `Space`/`Shift` и мышь после захвата кликом; `R` возвращает эту позу.

Проверки главы — юнит-тесты `camera.rs`, без GPU:

```sh
cargo run -p camera-fly
cargo test -p camera-fly
```

## Проверьте модель

1. Почему мышь не умножают на `dt`, а клавиши — умножают?
2. Что случится с базисом взгляда при `pitch = π/2` без clamp, и какое условие из [главы «Три измерения и базис»](../basis3d/) нарушится?
3. Два mouse delta `3` и `4` пришли между кадрами. На сколько радиан повернётся камера, и почему это не зависит от `dt`?
4. Камера в `(0, 0, 5)`, `yaw = 0`, `pitch = 0`. Куда придётся позиция после секунды `W` при скорости 2 м/с — и что изменится, если одновременно держать `D`?

::: details Решения и контрольные выводы

1. Mouse delta — уже «перемещение за интервал», её единица привязана к событию; `dt` превратил бы её в скорость и дважды учёл время. Клавиша состояния не имеет — только «нажата», поэтому путь = скорость × `dt`.
2. `forward = (0, 1, 0)` параллелен `world_up`: `forward × up` — нулевой вектор, `right` не нормализуется; построение базиса из параллельных входов мы отвергали ещё в [главе «Три измерения и базис»](../basis3d/). Clamp `±(π/2 − 0.01)` держит cross ненулевым (`cos(π/2 − 0.01) ≈ 0.01`).
3. На `(3 + 4) · SENSITIVITY = 7 · SENSITIVITY` радиан: дельты суммируются до одного update, `update` применяет сумму один раз — это и проверяет тест `mouse_deltas_accumulate_before_update`.
4. `(0, 0, 3)`: одна секунда пути вдоль `−Z` со скоростью 2 м/с. С `W + D` — `(√2, 0, 5 − √2)`: те же 2 м по диагонали `(1, 0, −1)/√2`; каждый компонент получает `2/√2 = √2`.

:::

Камера слушается — но поверхность куба по-прежнему описана наивно.
[Следующая глава](../cube-uv/) показывает, почему восемь углов куба — это двадцать четыре GPU-вершины.
