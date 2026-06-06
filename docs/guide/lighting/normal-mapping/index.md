---
editLink: false
---

# Normal Mapping

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/normal-mapping)

**Что уже должно быть понятно:**

- нормали, направленный свет, diffuse lighting
- текстуры и UV-координаты
- вершинные данные и атрибуты

**Что появится в этой главе:**

- normal map — текстура, хранящая нормали вместо цветов
- касательное пространство (tangent space) и TBN-матрица
- вектор tangent в вершинных данных
- преобразование нормалей из tangent space в world space

**Итог:** плоская стена выглядит рельефной — бугры и впадины создаются исключительно нормалями из текстуры

---

Diffuse-текстура добавляет поверхности цвет, но не добавляет рельеф. Нормаль одинакова для всех фрагментов треугольника —
освещение получается плоским. Чтобы плоская геометрия выглядела объёмной, нормаль меняют для каждого текселя.

## Normal map

Normal map — это текстура, в которой RGB-каналы хранят не цвет, а компоненты вектора нормали. Значения из диапазона
$[0,\,1]$ преобразуются обратно в $[-1,\,1]$:

```wgsl
let tangent_normal = normal_map.xyz * 2.0 - 1.0;
```

Каждый тексель содержит свой вектор нормали, и освещение вычисляется для этой «возмущённой» нормали вместо
геометрической. Результат — плоская поверхность получает иллюзию рельефа: бугры, впадины, царапины.

Стандартная normal map выглядит преимущественно фиолетово-синей: это потому, что большинство нормалей направлено
«прямо из поверхности» — компонента Z ≈ 1, что в RGB даёт ≈ (0.5, 0.5, 1.0).

## Почему нельзя хранить нормали сразу в world space

Представим, что нормали хранятся в мировых координатах. При повороте объекта нормали не повернутся вместе с ним —
освещение «отстанет» от геометрии. Текстура привязана к поверхности, поэтому нормали нужно хранить в локальной
системе координат, привязанной к самой поверхности.

Эта система называется **касательным пространством** (tangent space).

## Касательное пространство и TBN-матрица

В каждой точке поверхности определяется локальный базис из трёх векторов:

| Вектор | Обозначение | Направление |
|--------|-------------|-------------|
| **Tangent** | $T$ | вдоль оси $U$ текстуры |
| **Bitangent** | $B$ | перпендикулярен $T$ и $N$, вдоль $V$ |
| **Normal** | $N$ | перпендикуляр поверхности |

Матрица, составленная из этих векторов как столбцов, называется **TBN-матрицей**:

$$
\text{TBN} = \begin{pmatrix} T_x & B_x & N_x \\ T_y & B_y & N_y \\ T_z & B_z & N_z \end{pmatrix}
$$

Она переводит нормаль из tangent space в world space:

$$
\vec{n}_{\text{world}} = \text{TBN} \cdot \vec{n}_{\text{tangent}}
$$

Normal map хранит нормали именно в tangent space: ось Z — это нормаль к поверхности, X — касательная вдоль $U$,
Y — вдоль $V$. Нормаль $(0, 0, 1)$ в tangent space означает «напрямую из поверхности» — плоский участок.

<img src="/diagrams/tbn-basis.svg" alt="TBN базис на поверхности: касательная, битангенса, нормаль" style="width: 100%;" />

## Откуда берётся tangent

Tangent — это направление, вдоль которого изменяется координата $U$ текстуры. Для плоской стены,
разворачённой в плоскости $XY$ с нормалью $(0, 0, 1)$ и UV, увеличивающимся вдоль $+X$:

```
tangent = (1, 0, 0)   // вдоль U
normal  = (0, 0, 1)   // перпендикуляр к поверхности
```

Bitangent вычисляется в шейдере через векторное произведение:

```wgsl
let B = cross(N, T);
```

Для стенки: `cross((0,0,1), (1,0,0)) = (0, 1, 0)` — ось $Y$, что соответствует направлению $V$.

Для сложных мешей (сфера, ландшафт) касательные обычно вычисляются автоматически при экспорте из
моделера (Blender, Substance Painter) и хранятся как дополнительный вершинный атрибут.

## Вершинные данные

Добавим вектор `tangent` к уже знакомой структуре вершины:

```rust
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    tangent: [f32; 3],
}
```

Stride = $12 + 12 + 8 + 12 = 44$ байта. Четыре атрибута вместо трёх:

```rust
const ATTRIBUTES: [VertexAttribute; 4] = [
    // position: offset 0
    VertexAttribute { offset: 0, shader_location: 0, format: VertexFormat::Float32x3 },
    // normal: offset 12
    VertexAttribute { offset: 12, shader_location: 1, format: VertexFormat::Float32x3 },
    // uv: offset 24
    VertexAttribute { offset: 24, shader_location: 2, format: VertexFormat::Float32x2 },
    // tangent: offset 32
    VertexAttribute { offset: 32, shader_location: 3, format: VertexFormat::Float32x3 },
];
```

Стена — четыре вершины с одинаковыми нормалью $(0,0,1)$ и tangent $(1,0,0)$:

```rust
const VERTICES: &[Vertex] = &[
    Vertex { position: [-3.0, -2.0, 0.0], normal: [0.0, 0.0, 1.0],
             uv: [0.0, 3.0], tangent: [1.0, 0.0, 0.0] },
    Vertex { position: [ 3.0, -2.0, 0.0], normal: [0.0, 0.0, 1.0],
             uv: [4.0, 3.0], tangent: [1.0, 0.0, 0.0] },
    Vertex { position: [ 3.0,  2.0, 0.0], normal: [0.0, 0.0, 1.0],
             uv: [4.0, 0.0], tangent: [1.0, 0.0, 0.0] },
    Vertex { position: [-3.0,  2.0, 0.0], normal: [0.0, 0.0, 1.0],
             uv: [0.0, 0.0], tangent: [1.0, 0.0, 0.0] },
];
```

UV повторяются 4×3 раза — normal map будет тайлиться по поверхности.

## Генерация normal map

В этой главе normal map генерируется процедурно: сетка из полусферических бугров.

Для каждого текселя вычисляется расстояние до центра ближайшего бугра. Если тексель внутри бугра —
нормаль отклоняется от $(0,0,1)$ в сторону от центра:

```rust
fn generate_normal_map() -> Vec<u8> {
    let spacing = BUMP_SPACING as f32;
    let half = spacing / 2.0;
    let mut pixels = Vec::with_capacity((TEX_SIZE * TEX_SIZE * 4) as usize);

    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let cell_x = (x as f32 / spacing).floor() * spacing + half;
            let cell_y = (y as f32 / spacing).floor() * spacing + half;

            let dx = x as f32 - cell_x;
            let dy = y as f32 - cell_y;
            let dist_sq = dx * dx + dy * dy;
            let r_sq = BUMP_RADIUS * BUMP_RADIUS;

            let normal = if dist_sq < r_sq {
                let z = (1.0 - dist_sq / r_sq).sqrt();
                Vec3::new(dx / BUMP_RADIUS, -dy / BUMP_RADIUS, z)
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };

            pixels.push(((normal.x * 0.5 + 0.5) * 255.0) as u8);
            pixels.push(((normal.y * 0.5 + 0.5) * 255.0) as u8);
            pixels.push(((normal.z * 0.5 + 0.5) * 255.0) as u8);
            pixels.push(255);
        }
    }
    pixels
}
```

`dy` взят с минусом, потому что в изображении ось $Y$ направлена вниз, а в tangent space ось $Y$ (bitangent)
направлена вверх. Без этой инверсии бугры выглядели бы «вдавленными».

::: tip Формат текстуры
Normal map использует `Rgba8Unorm`, **не** `Rgba8UnormSrgb`. Данные в normal map — это не цвет, а векторы.
sRGB-преобразование исказило бы их значения.
:::

## Шейдер: TBN и сэмплирование

Вершинный шейдер передаёт `normal` и `tangent` без изменений — стена не имеет model-преобразования:

```wgsl
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = camera.view_proj * vec4<f32>(input.position, 1.0);
    output.normal = input.normal;
    output.tangent = input.tangent;
    output.uv = input.uv;
    return output;
}
```

Фрагментный шейдер строит TBN, сэмплирует normal map и вычисляет освещение с возмущённой нормалью:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let N = normalize(input.normal);
    let T = normalize(input.tangent);
    let B = cross(N, T);

    let normal_map = textureSample(normal_tex, normal_sampler, input.uv);
    let tangent_normal = normal_map.xyz * 2.0 - 1.0;
    let world_normal = normalize(
        T * tangent_normal.x + B * tangent_normal.y + N * tangent_normal.z
    );

    let light_dir = normalize(-light.light_dir);
    let diffuse = max(dot(world_normal, light_dir), 0.0);

    let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
    let intensity = light.ambient + diffuse * (1.0 - light.ambient);
    return vec4<f32>(tex_color.rgb * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
}
```

Разберём по шагам:

1. **TBN**: `N` и `T` нормализуются (после интерполяции длины могут чуть измениться), `B` вычисляется через
   `cross(N, T)`.
2. **Сэмплирование**: `textureSample` возвращает `vec4<f32>` в диапазоне $[0,\,1]$.
3. **Декодирование**: умножение на 2 и вычитание 1 переводит в $[-1,\,1]$.
4. **Преобразование**: `T * x + B * y + N * z` — это умножение TBN-матрицы на вектор-столбец.
5. **Освещение**: стандартный diffuse через `dot(world_normal, light_dir)`.

## Настройки сэмплера

Для normal map используется `FilterMode::Linear` — без линейной фильтрации нормали будут «лесенкой»
между текселями, и освещение будет ступенчатым. Адресация — `Repeat`, чтобы normal map тайлилась:

```rust
let sampler = ctx.device.create_sampler(&SamplerDescriptor {
    address_mode_u: AddressMode::Repeat,
    address_mode_v: AddressMode::Repeat,
    mag_filter: FilterMode::Linear,
    min_filter: FilterMode::Linear,
    mipmap_filter: MipmapFilterMode::Nearest,
    ..Default::default()
});
```

## Bind groups

Две группы, аналогично [Материалам и свету](/guide/lighting/materials/):

| Group | Binding | Ресурс |
|-------|---------|--------|
| 0 | 0 | `CameraUniforms` (view_proj) |
| 1 | 0 | `LightUniforms` (light_dir, ambient) |
| 1 | 1 | diffuse texture |
| 1 | 2 | diffuse sampler |
| 1 | 3 | normal map texture |
| 1 | 4 | normal map sampler |

Обе текстуры используют один и тот же объект сэмплера — он без изменений привязан к двум binding'ам.

## Типичные ошибки

::: warning sRGB-формат для normal map
Значения нормалей исказятся гамма-кривой, освещение будет неверным.
Используйте `Rgba8Unorm`.
:::

::: warning Забыли `2.0 * x - 1.0`
Нормали останутся в $[0,\,1]$, и все будут «смотреть» в одну сторону.
Результат — почти полностью тёмная или пересвеченная поверхность.
:::

::: warning Неправильное направление tangent
Если tangent указывает не вдоль $U$, а вдоль $V$, нормали
будут повёрнуты на 90°. Визуально это выглядит как «косое» освещение бугров.
:::

::: warning Bitangent без учёта handedness
`cross(N, T)` даёт правильный bitangent для правосторонних UV.
На зеркально отражённых поверхностях (negative scale) нужен множитель −1, который обычно
передают в компоненте `w` вектора tangent.
:::

::: warning Не нормализуют TBN после интерполяции
`normalize()` обязателен: интерполяция через
растеризатор может изменить длину векторов.
:::

::: warning `FilterMode::Nearest` для normal map
Нормали будут «пикселизированы». Для плавного рельефа
нужен `Linear`.
:::

## Что получилось

- Normal map хранит векторы нормалей в **tangent space** — локальной системе координат поверхности.
- **TBN-матрица** (Tangent × Bitangent × Normal) переводит нормаль из tangent space в world space.
- Вершинный атрибут **tangent** задаёт направление оси $U$ текстуры на поверхности.
- **Bitangent** вычисляется в шейдере как `cross(normal, tangent)`.
- Формат `Rgba8Unorm` (не Srgb) — данные линейны, это векторы, а не цвета.

Результат: плоский квадрат из двух треугольников выглядит как каменная стена с рельефом — без
дополнительной геометрии, исключительно за счёт нормалей из текстуры.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить радиус и шаг bump-паттерна в `generate_normal_map` — увидеть другой рельеф
- Повернуть стену через model-матрицу — убедиться, что нормали корректно трансформируются
- Убрать normal map (вернуть `vec3<f32>(0.0, 0.0, 1.0)`) — сравнить плоскую и рельефную поверхность
- Увеличить интенсивность света — рельеф станет более выраженным

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/normal-mapping)
