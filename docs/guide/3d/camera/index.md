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

## Обработка ввода

`Input` — структура из учебного каркаса (`framework::Input`), которая хранит состояние ввода между кадрами:
нажатые клавиши (`pressed_keys`), смещение мыши (`mouse_delta`), нажатые кнопки мыши. Каркас обновляет
её каждый кадр перед вызовом `Example::update`.

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

## Что получилось

::: warning Типичные ошибки
- `direction().normalize()` обязателен — `look_to_rh` panic'ает при нулевом направлении
- Pitch ограничен ±89° — при ±90° камера переворачивается (gimbal lock)
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
