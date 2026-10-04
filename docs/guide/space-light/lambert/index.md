---
title: Нормали и первый свет
description: "Нормаль как атрибут, разрывы нормалей без шва позиций, интерполяция с ренормализацией и закон Ламберта."
prev:
  text: "Поверхность и UV-швы"
  link: ../cube-uv/
next:
  text: "Преобразование нормалей"
  link: ../normal-matrix/
---

# Нормали и первый свет

::: info Только native
Снимок [главы «Поверхность и UV-швы»](../cube-uv/): статическая сцена, depth, фиксированная камера `(0, 0, 5)` с перспективой. Вместо куба — плоские грани с явно заданными нормалями; клавиша `N` переключает режим показа.
:::

Фронтальная грань ярче наклонённой: яркость зависит от **ориентации поверхности** — и ориентацию тоже придётся записать в вершины.
А две грани, наклонённые на одинаковый угол в разные стороны, неотличимы — проверим, почему.

**Эксперимент:** две наклонные плоские грани (±30° вокруг X) и одна грань с разрывом нормалей.
Один направленный свет: `L = (0, 0, 1)` — от поверхности к источнику; `albedo = 0.5` — безразмерная доля отражённого света, `ambient = 0.1` — прибавка света не от источника, `intensity = 0.6` — масштаб прямого члена; всё в линейных величинах.
Клавиша `N` переключает диагностический режим `0.5·(N+1)` и ламбертовское затенение.

## Нормаль: перпендикуляр из cross

Плоскую грань задают два касательных направления: `t` — вдоль, `b` — поперёк.
Перпендикуляр обоим — их cross-произведение из [главы «Три измерения и базис»](../basis3d/): `n = t × b`; после нормализации получаем **единичную нормаль** — стрелку, торчащую из поверхности.
Для плоского квадрата все четыре вершины получают одну и ту же нормаль: ориентация общая, но у каждой вершины — своя запись атрибута (урок [главы «Поверхность и UV-швы»](../cube-uv/)).

Запись вершины расширяется: позиция с `w = 1` (точка) и нормаль с `w = 0` (направление — перенос его не сдвигает; договор из [главы «Матрицы и композиция»](../matrix-compose/)).
Четыре компоненты у обоих атрибутов — удобная 16-байтовая кратность, которая ещё пригодится, когда модель перестанет быть тождественной.

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// Position with w = 1: a point.
    position: [f32; 4],
    /// Normal with w = 0: a direction, immune to translation. Padded to four
    /// components so both attributes share the simple 16-byte alignment.
    normal: [f32; 4],
}
```

## Наклонные грани и разрыв без шва

Наклон квадрата вокруг X на угол θ: точка `(x, y, 0)` переходит в `(x, y·cos θ, y·sin θ)`, нормаль `(0, 0, 1)` — в `(0, −sin θ, cos θ)`.
Таблица вершин — явные литералы, как в [главе «Поверхность и UV-швы»](../cube-uv/):

```rust
// Quad A: tilt +30 degrees, center (-0.95, 0.55, 0).
Vertex { position: [-1.55, 1.0696, 0.3, 1.0], normal: QUAD_A_NORMAL },
Vertex { position: [-0.35, 1.0696, 0.3, 1.0], normal: QUAD_A_NORMAL },
Vertex { position: [-1.55, 0.0304, -0.3, 1.0], normal: QUAD_A_NORMAL },
Vertex { position: [-0.35, 0.0304, -0.3, 1.0], normal: QUAD_A_NORMAL },
```

```rust
// Split face, left half: x in [-1, 0], normal tilted -45 degrees.
Vertex { position: [-1.0, -0.3, 0.0, 1.0], normal: SPLIT_LEFT_NORMAL },
Vertex { position: [0.0, -0.3, 0.0, 1.0], normal: SPLIT_LEFT_NORMAL },
Vertex { position: [-1.0, -1.3, 0.0, 1.0], normal: SPLIT_LEFT_NORMAL },
Vertex { position: [0.0, -1.3, 0.0, 1.0], normal: SPLIT_LEFT_NORMAL },
// Split face, right half: x in [0, 1], normal facing the camera.
Vertex { position: [0.0, -0.3, 0.0, 1.0], normal: SPLIT_RIGHT_NORMAL },
Vertex { position: [1.0, -0.3, 0.0, 1.0], normal: SPLIT_RIGHT_NORMAL },
Vertex { position: [0.0, -1.3, 0.0, 1.0], normal: SPLIT_RIGHT_NORMAL },
Vertex { position: [1.0, -1.3, 0.0, 1.0], normal: SPLIT_RIGHT_NORMAL },
```

Нижняя грань — главная хитрость главы: **позиции обоих половин лежат в одной плоскости `z = 0`**, излома геометрии нет вообще.
Нормали — разные:

```rust
const SPLIT_LEFT_NORMAL: [f32; 4] = [0.0, -std::f32::consts::FRAC_1_SQRT_2, std::f32::consts::FRAC_1_SQRT_2, 0.0];
const SPLIT_RIGHT_NORMAL: [f32; 4] = [0.0, 0.0, 1.0, 0.0];
```

Нормаль левой половины наклонена на 45°: обе компоненты равны `1/√2` из стандартной константы.

Разрыв нормалей не следует за UV-швами и не требует шва позиций: это независимый атрибут со своими границами.
Освещение «видит» излом там, где геометрия идеально плоская.

## Интерполяция и ренормализация

Нормаль едет в фрагментный шейдер как varying — интерполируется по треугольнику, как цвета [главы «Интерполяция»](../../foundations/vertex-colors/) и UV [главы «Тексели и upload»](../../foundations/texture-load/).
Интерполяция — взвешенное среднее, а среднее единичных векторов короче единицы: у двух перпендикулярных нормалей длина среднего — ровно `√0.5` (юнит-тест главы).
Короткая нормаль занижает `dot` — поэтому перед использованием нормаль **ренормализуется**:

```wgsl
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
};
```

```wgsl
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = view_proj * input.position;
    output.normal = input.normal.xyz;
    return output;
}
```

## N·L и закон Ламберта

Освещённость наклонной площадки — «сколько света ловит единица площади»: прямыми лучами — максимум, под 90° — ноль.

```text
        L (к источнику)
         ↗
        ╱|  N (нормаль)
       ╱ |   ↑
      ╱  |   |
     ╱ φ |   |
    ╱____|___|______
   поверхность    d = N·L = cos φ
                   φ = 0°: d = 1 — прямо
                   φ = 60°: d = 0.5
                   φ = 90°: d = 0 — ребром
```

Это [dot-произведение](../basis3d/): `d = N·L = cos φ` для единичных векторов.
Направление `L` договоримся считать **от поверхности к источнику**, тогда `d > 0` — поверхность смотрит в сторону света.

Направленный свет и материал — один uniform; компоновка полей — знакомый контракт [смешанных полей](../../foundations/uniform-params/):

```rust
/// Directional light and material response, all in linear units. The layout
/// contract follows chapter 08: vec3 fields align to 16 bytes, while plain
/// scalars align to 4, so encase writes light_dir at 0, albedo at 16, then
/// the scalars pack tightly at 28/32/36, and the struct size rounds up to
/// the 48-byte uniform slot.
#[derive(ShaderType, Debug, Clone, Copy)]
pub struct LightParams {
    /// Unit direction from the surface toward the light source.
    pub light_dir: Vec3,
    /// Base surface color (fraction of light the material reflects).
    pub albedo: Vec3,
    /// Constant term: brightness that does not depend on the normal. An
    /// approximation of indirect light, not a computation of it.
    pub ambient: f32,
    /// Strength of the direct (Lambert) term.
    pub intensity: f32,
    /// 1 = show 0.5*(N+1) instead of shading; the N key switches this.
    pub show_normals: u32,
}
```

Заметьте: скаляры не дожимаются до 16-байтовой границы — после `vec3`, занявшей `16..28`, `f32` с выравниванием 4 ложится на естественное смещение 28. До 48 байтов добивается только размер структуры целиком.
Формула — **закон Ламберта**: `цвет = albedo · (ambient + intensity · d)`, `d = max(N·L, 0)`; clamp отсекает развёрнутые от света поверхности — они не «светятся в минус»:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // Interpolation shortens the varying normal; restore the unit length
    // before measuring the angle to the light.
    let n = normalize(input.normal);
    if (light.show_normals == 1u) {
        // Diagnostic view: map the unit normal from [-1, 1] into [0, 1].
        return vec4<f32>(n * 0.5 + vec3<f32>(0.5), 1.0);
    }
    // L points from the surface toward the light; both N and L live in
    // world space, so the dot product measures the angle between them.
    let l = normalize(light.light_dir);
    let d = max(dot(n, l), 0.0);
    let color = light.albedo * (light.ambient + light.intensity * d);
    // Linear output; the sRGB target encodes it on store.
    return vec4<f32>(color, 1.0);
}
```

Разбирать кадр начните с диагностического режима `N`: нормаль, отображённая в цвет `0.5·(N+1)`, сразу показывает и наклон граней, и излом на плоской split-грани.
Контрольные числа (линейные, до sRGB-кодирования): `d = 1` даёт `0.5·(0.1 + 0.6) = 0.35`; `d = 0` — только ambient: `0.5·0.1 = 0.05`.
Выводим линейный цвет — кодирует sRGB-таргет, как в [главе о кодировании](../../foundations/srgb-mixing/).

**Ambient — не физика, а заглушка**: константа изображает непрямой свет (небо, отражения), который ламбертовская модель не считает.

![Кадры lambert: слева ламбертовское затенение, справа диагностический режим нормалей.](/results/lambert-normals.png)

Слева яркость граней задаёт `N·L`: наклонные грани (±30°) светятся одинаково, излом виден на плоской split-грани. Справа — режим `N`: те же нормали показаны как цвета (`RGB = XYZ`) — у наклонных граней разные цвета при одинаковой яркости слева.

Проверка кадра — GPU-тест: грани плоские, `d` константен внутри каждой, поэтому проба центра сравнивается с CPU-расчётом `quantize(srgb(0.5·(0.1 + 0.6·d)))` для `d = cos 30°`, `cos 45°` и `1`, плюс две пробы у линии излома (по обе стороны).

```sh
cargo run -p lambert
cargo test -p lambert
cargo test -p verify
```

## Проверьте модель

1. Обе наклонные грани (±30°) в режиме Ламберта одинаково яркие при `L = (0, 0, 1)`. Почему? Чем тогда они отличаются?
2. Почему нормаль ренормализуют в фрагментном шейдере, а не используют интерполированное значение как есть?
3. Что изменится в кадре, если `ambient` увеличить с `0.1` до `0.5`, а `intensity` уменьшить с `0.6` до `0.2`?
4. Нормаль `w = 0`: что произойдёт с освещением, когда в [следующих главах](../normal-matrix/) у модели появится перенос, если бы мы хранили нормаль как точку с `w = 1`?

::: details Решения и контрольные выводы

1. `d = cos 30°` у обеих: `dot` с `L = (0,0,1)` измеряет только Z-компоненту нормали, а у `(0, −0.5, 0.866)` и `(0, +0.5, 0.866)` она одинакова. Отличаются направлением нормали — это видно в режиме `N`: те же яркости, разные цвета.
2. Интерполяция укорачивает вектор; `dot` короткой нормали меньше истинного — грань темнеет без причины. Ренормализация восстанавливает единичную длину.
3. Разница яркостей сожмётся: тень и свет станут `0.5·(0.5+0.2·1) = 0.35` и `0.5·(0.5) = 0.25` против `0.35`/`0.05`. Ambient поднимает всё сразу, интенсивность — только освещённое; контраст зависит от их соотношения.
4. Ничего хорошего: как точка, нормаль поедет вместе с моделью при переносе (например, «вверх» на метр) — а направление перпендикулярности переносу не подчиняется. Компонента `w = 0` обнуляет вклад столбца переноса; полный разбор — в главе о [матрице нормалей](../normal-matrix/).

:::

Грань светится — но модель пока не двигается, и нормали не преобразуются.
[Следующая глава](../normal-matrix/) спрашивает: что будет с нормалью при неравномерном масштабе, и почему её нельзя просто умножить на ту же матрицу, что и позицию.
