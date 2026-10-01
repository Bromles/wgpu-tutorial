---
title: Проекция и путь на экран
description: "Ортография и перспектива: FOV/aspect/near/far, подобные треугольники и матрица по действию."
prev:
  text: "Камера как система координат"
  link: ../look-at/
next:
  text: "Clip, экран и интерполяция"
  link: ../clip-viewport/
---

# Проекция и путь на экран

::: info Только native
Сцена продолжает [камеру look-at](../look-at/): тот же глаз в `(0, 0, 5)`. Текстура 2×2 — из [главы о загрузке](../../foundations/texture-load/).
:::

Матрица вида [прошлой главы](../look-at/) выражает мир в метрах перед камерой — но экран про метры ничего не знает.
Мост от метров к пикселям наводит **проекция**, и честных режимов у неё два.

**Эксперимент:** пол, стрелки осей и две **одинаковые** текстурированные таблички `0.8 × 0.8` м — одна в 2 м от глаза (плоскость `z = 3`), другая в 4 м (`z = 1`). Клавиша `P` переключает проекцию; разница между табличками — и есть измерение.

## Ортография: параллельные лучи

Ортографическая проекция измеряет сцену пучком **параллельных** лучей: где бы объект ни висел, его метры переводятся в координаты кадра одинаково.
Объём задаётся шестью границами в метрах: `left/right`, `bottom/top`, `near/far`:

```rust
glam::camera::rh::proj::directx::orthographic(
    -half_width,
    half_width,
    -ORTHO_HALF_HEIGHT,
    ORTHO_HALF_HEIGHT,
    NEAR,
    FAR,
)
```

Здесь `ORTHO_HALF_HEIGHT = 3`: в кадр по вертикали попадают метры от −3 до +3; `NEAR = 1`, `FAR = 9` отрезают всё ближе и дальше. Обе таблички в кадре, и на экране они **равны** — расстояние не участвует.

## Перспектива: подобные треугольники

Перспектива пускает лучи **через глаз**. Точка с координатами `(x, y)` на расстоянии `d` от глаза попадает на кадр в той же пропорции — это подобные треугольники:

```text
доля высоты кадра:  y / (d · tan(fov_y/2))
доля ширины кадра:  x / (d · tan(fov_y/2) · aspect)
```

Вертикальный угол `fov_y` (у нас 60° = `π/3` рад) задаёт, сколько метров видно в кадре на расстоянии `d`; `aspect` — отношение сторон кадра.

Высоту кадра даёт треугольник «глаз — центр кадра — верхний край»: угол при глазе — `fov_y/2`, противолежащий катет `d·tan(fov_y/2)` — половина видимой высоты; целиком — `2·d·tan(fov_y/2)`. Доля объекта в кадре — его метры, делённые на эту высоту.

| расстояние d | видимая высота 2·d·tan(30°) | доля таблички 0.8 м |
| --- | --- | --- |
| 2 м | ≈ 2.31 м | ≈ 34.6% высоты кадра |
| 4 м | ≈ 4.62 м | ≈ 17.3% высоты кадра |

Удвоение расстояния — вдвое меньший экранный размер: больше из подобия не следует ничего.

![Кадры ortho-perspective: слева перспектива, справа ортография — одна и та же сцена с двумя табличками.](/results/ortho-perspective-pair.png)

Слева — перспектива: дальняя табличка ровно вдвое меньше ближней — это деление на `w`. Справа — ортография на той же сцене: размеры от расстояния не зависят. Сравните дальний и ближний квад — это и есть измерение главы.

## Матрица по действию

Нужна матрица, которая делает ровно это действие. `glam::camera::rh::proj::directx::perspective(fov_y_radians, aspect, near, far)` закладывает `1/(tan(fov_y/2)·aspect)` и `1/tan(fov_y/2)` в диагональ. **Расстояние она отправляет в `w`**: у вершины на расстоянии `d` clip-координата `w = d`.
Само деление на `w` матрица не делает — его делает растеризатор, и это не техническая деталь, а тема [следующей страницы](../clip-viewport/).

Ключ `P` меняет только второй множитель:

```rust
    /// The projection matrix of the current mode. Perspective divides by the
    /// distance (through w), orthography maps meters to clip directly.
    fn projection_matrix(&self) -> Mat4 {
        match self.projection {
            Projection::Perspective => glam::camera::rh::proj::directx::perspective(FOV_Y, self.aspect, NEAR, FAR),
            Projection::Orthographic => {
                let half_width = ORTHO_HALF_HEIGHT * self.aspect;
                glam::camera::rh::proj::directx::orthographic(
                    -half_width,
                    half_width,
                    -ORTHO_HALF_HEIGHT,
                    ORTHO_HALF_HEIGHT,
                    NEAR,
                    FAR,
                )
            }
        }
    }
```

Оба конструктора — правые (`rh`): камера смотрит вдоль своего `−Z`, как в [look-at](../look-at/). `near = 0` запрещён: точка на near-плоскости получила бы `w = 0`, а делить на ноль нельзя.

## Uniform и шейдер

View и proj едут в шейдер **раздельно** — переключение проекции не должно трогать камеру:

```rust
/// The camera pair of the chapter: the fixed view from look-at plus the
/// projection the P key switches. Two mat4x4 columns, 128 bytes total.
#[derive(ShaderType, Debug, Clone, Copy, PartialEq)]
pub struct Params {
    pub view: Mat4,
    pub proj: Mat4,
}
```

128 байт через encase — знакомый контракт [главы «Смешанные поля»](../../foundations/uniform-params/). Вершинный шейдер применяет оба множителя по порядку «точка → view → proj»:

```wgsl
@vertex
fn vs_textured(input: VertexInput) -> TexturedOutput {
    var output: TexturedOutput;
    output.position = params.proj * (params.view * input.position);
    output.uv = input.uv;
    return output;
}
```

Таблички читают текстуру 2×2 через `textureLoad` — целые тексели без фильтрации, как в [главе «Тексели и upload»](../../foundations/texture-load/): цвета квадрантов остаются точными, и по ним удобно измерять.

## Снимок

Сцена рисуется художником: без depth-теста (он появится только в [главе «Ориентация и видимость»](../depth-culling/)) порядок draw — это и есть видимость, поэтому строго «из глубины»:

```rust
        // Depth testing arrives only in chapter 20: for now the painter's
        // order is the whole story, so the scene goes back to front.
        pass.set_pipeline(&self.flat_pipeline);
        pass.draw_indexed(0..24, 0, 0..1);
        pass.set_pipeline(&self.textured_pipeline);
        pass.draw_indexed(24..30, 0, 0..1);
        pass.draw_indexed(30..36, 0, 0..1);
```

Пол и оси идут первым pipeline (плоские цвета), таблички — вторым. Кадр статичен, и `P` перерисовывает его ровно один раз:

```rust
WindowEvent::KeyboardInput { event: key_event, .. }
    if key_event.state == ElementState::Pressed
        && let PhysicalKey::Code(KeyCode::KeyP) = key_event.physical_key =>
{
    self.projection = match self.projection {
        Projection::Perspective => Projection::Orthographic,
        Projection::Orthographic => Projection::Perspective,
    };
    // The frame is static: repaint exactly when the key
    // changed it.
    window.request_redraw();
}
```

Автоматическая проверка ищет в кадре пиксели четырёх цветов текстуры и меряет ширину каждой таблички по горизонтали: при перспективе ближняя — ровно вдвое шире дальней (≈199.5 против ≈99.8 пикселя), при ортографии — равны.

```sh
cargo run -p ortho-perspective
cargo test -p foundations-verify --test ortho_perspective
```

## Проверьте модель

1. На каком расстоянии от глаза табличка 0.8 м заняла бы ровно половину высоты кадра?
2. Что покажет ортография для двух табличек — и почему пол при этом меняет форму с трапеции на прямоугольник?
3. Почему `near = 0` для перспективы запрещён, а для ортографии опасности нет?
4. Камера та же, `fov_y` увеличили с 60° до 90°. Что станет с табличками на экране?

::: details Решения и контрольные выводы

1. `0.8 / (2·d·tan(30°)) = 0.5` → `2·d·0.5774 = 1.6` → `d ≈ 1.39` м. Ближе near = 1 табличку всё равно не поставить.
2. Одинаковые прямоугольники: параллельные лучи не зависят от расстояния. Пол при перспективе — трапеция (дальняя кромка видна под меньшим углом), при ортографии расстояние не искажает пропорции.
3. У перспективы `w = d`: на near-плоскости вышло бы `w = 0` и деление на ноль. У ортографии `w = 1` всегда — делить не на что.
4. Кадр охватывает больше метров на том же расстоянии (`tan(45°) = 1` против `0.577`): обе таблички уменьшатся, дальняя — вдвое меньше ближней, как и было: отношение от fov не зависит.

:::

Остался последний шаг пути: что именно происходит между clip-координатами с их `w` и пикселями.
[Следующая страница](../clip-viewport/) разбирает его по стадиям — и показывает, где экранно-линейная интерполяция врёт.
