---
title: "Карта теней"
description: "Depth-only проход с позиции направленного света, ортографическая shadow map, world→light clip→NDC→UV с переворотом Y, texture_depth_2d и ручное сравнение глубин."
prev:
  text: "Fragment или compute: обработка изображения"
  link: ../image-pipeline/
next:
  text: "Артефакты теней и PCF"
  link: ../shadow-pcf/
---

# Карта теней

::: info Только native
Снимок собирает три известных куска: [направленный Lambert](../../space-light/lambert/), [куб из 24 вершин](../../space-light/cube-uv/) и [промежуточную текстуру](../render-to-texture/). Одна сцена, два прохода в одном `draw`, одна карта глубины света.
:::

Освещённая нормаль — ещё не видимость источника.
Между поверхностью и светом может стоять другой объект, и `dot(N, L)` об этом ничего не знает: он меряет угол, а не препятствия.

**Эксперимент:** куб со стороной 1 м парит в сантиметре над полом 4×4 м; направленный свет светит сверху-сбоку, камера и свет фиксированы.
Перед основным проходом сцена рисуется **с позиции света** — но не цветом, а глубиной — в текстуру 1024×1024.
Основной проход для каждого фрагмента спрашивает эту карту: «есть ли между мной и светом что-то ближе?»

```text
pass 1: cube ──depth-only──→ shadow map 1024×1024 (Depth32Float)
pass 2: floor + cube ──Lambert × visibility──→ surface
```

## Сцена: caster и receiver

Роли разделяем явно: куб — **caster** (отбрасывает тень), пол — **receiver** (принимает).
Это не свойство геометрии, а решение прохода: в depth-проход рисуется только куб, потому что под полом нет никого, кого он мог бы затенить.

Меш — знакомая таблица куба [главы «Поверхность и UV-швы»](../../space-light/cube-uv/), где UV заменены на плоские нормали граней: каждый угол встречается три раза с тремя разными нормалями, излом живёт в атрибуте.
Нормаль пола — `+Y` у всех четырёх углов.
Позиция+нормаль — те же 32 байта (`w = 1` / `w = 0`), что в [Lambert](../../space-light/lambert/).

Свет фиксирован: `L = normalize((0.5, 1.0, 0.5)) ≈ (0.408, 0.816, 0.408)` — сверху, с равным наклоном в `+X` и `+Z`.
Куб центрирован в `(0, 0.51, 0)`: его нижняя грань на высоте 1 см, и у тени остаётся видимая зона контакта.
Камера — `(2.5, 2.0, 4.0)` смотрит на `(0, 0.3, 0)`, `glam::camera::rh::proj::directx::perspective(60°, 4:3, 0.1, 50)`; аспект зафиксирован, чтобы проекция не зависела от окна.

Контрольные числа формулы (albedo 0.5, ambient 0.1, intensity 0.6):

- пол, `d = dot(+Y, L) = 0.816`: `0.5·(0.1 + 0.6·0.816) ≈ 0.295` → sRGB-код **148**;
- пол в тени (`visibility = 0`): `0.5·0.1 = 0.05` → код **63**.

## Свет — это ортографическая камера

Направленный свет — это параллельные лучи.
Параллельные лучи — это ортографическая проекция: строим вторую камеру, «глаз света», в 10 единицах вдоль `L` от центра сцены:

```rust
pub fn light_eye() -> Vec3 {
    LIGHT_TARGET + light_dir() * LIGHT_DISTANCE
}
```

```rust
/// The light view*projection: an orthographic box fits the parallel rays
/// of a directional light. Multiplied as P·V, applied to a point from
/// the right: `light_view_proj * model * position`.
pub fn light_view_proj() -> Mat4 {
    glam::camera::rh::proj::directx::orthographic(
        -LIGHT_HALF_WIDTH,
        LIGHT_HALF_WIDTH,
        -LIGHT_HALF_WIDTH,
        LIGHT_HALF_WIDTH,
        NEAR_L,
        FAR_L,
    ) * glam::camera::rh::view::look_at_mat4(light_eye(), LIGHT_TARGET, Vec3::Y)
}
```

Коробка `±3 м × ±3 м`, глубина `[5, 15]`: сцена (все углы пола и куба при всех положениях клавиши `M`) занимает по оси света 8.8–12.0 единиц от глаза — с запасом внутри, это проверяет юнит-тест `light_frustum_covers_floor_and_all_cube_poses`.
Глаз света ≈ `(4.08, 8.66, 4.08)`.

Матрица та же, что для камеры, — и применяется так же, справа налево: `light_view_proj * model * position`.

## Depth-only проход

Карту света создаём один раз: формат — глубина, usage — и вложение, и текстура, как у [промежуточного кадра](../render-to-texture/):

```rust
        // The shadow map: written by pass 1 as an attachment, read by
        // pass 2 as a texture. Fixed size, independent of the window.
        let shadow_texture = gpu.device.create_texture(&TextureDescriptor {
            label: Some("Shadow map"),
            size: Extent3d {
                width: SHADOW_SIZE,
                height: SHADOW_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Depth32Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
```

1024×1024 текселя на 6×6 м — это ≈5.9 мм на тексель.

Пайплайн прохода — только вершины: `fragment: None` валиден и означает «цвет не пишем вовсе», пишет только растеризатор в depth-attachment:

```rust
                // No fragment stage: the pass writes depth only, and
                // `fragment: None` is the valid way to say so.
                fragment: None,
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    ..PrimitiveState::default()
                },
                // The pass has a depth attachment, so the pipeline must
                // declare a depth_stencil state with the same format.
                depth_stencil: Some(DepthStencilState {
                    format: TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
```

Вершинный шейдер — две матрицы и всё:

```wgsl
// Depth-only pass: the light camera cares about coverage, not color, so
// the pipeline has no fragment stage at all.
@vertex
fn vs_depth(input: VertexInput) -> @builtin(position) vec4<f32> {
    return light_view_proj * model * input.position;
}
```

Сам проход — первый в `draw`, без color-attachments, с очисткой глубины в 1.0 («дальше всего») и обязательным `Store`: следующие проходы читают ровно эти значения:

```rust
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Shadow depth pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.shadow_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..RenderPassDescriptor::default()
            });
            pass.set_pipeline(&self.depth_pipeline);
            pass.set_bind_group(0, &self.cube_shadow_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // Only the cube casts: the floor is a pure receiver, there
            // is nothing below it to shadow.
            pass.draw_indexed(CUBE_INDICES, 0, 0..1);
```

Обратите внимание на bind group прохода: в нём нет самой карты — она привязана как **вложение** прохода, и читать её тут же запрещено контрактом «не читать и писать одно изображение в одном проходе».

## Из мира в тексель карты: ось Y

Основной проход для каждого фрагмента повторяет путь точки через матрицу света: world → light clip → деление на w → NDC → тексель.

X прост: `u = 0.5·x_ndc + 0.5`.
**Y надо перевернуть**, и это не договорённость, а следствие записи:

1. Проход pass 1 писал карту через viewport — как любой render target: строка 0 наверху, `row = (1 − y_ndc)/2 · H`. Фрагмент с `y_ndc = +1` (верх изображения света) попал в **строку 0**.
2. `textureLoad` адресует те же строки сверху: `y = 0` — первая строка.
3. Читающая сторона обязана обратить то же соответствие: `+1 → v = 0`, то есть `v = 0.5 − 0.5·y_ndc`. Вместе: `uv = ndc · (0.5, −0.5) + (0.5, 0.5)`.

Переворот не устранить поворотом света: запись и чтение используют **одну и ту же** матрицу — любой поворот взаимно сокращается. Переворот живёт в конвенции «строка 0 сверху», от камеры не зависящей.

Контрольные точки для самопроверки: угол пола `(−2, 0, −2)` в изображении света имеет NDC `(0, +0.67)` — верхняя половина, `v ≈ 0.163`, строки 0–167; угол `(2, 0, 2)` — NDC `(0, −0.87)`, `v ≈ 0.93`, нижние строки.
Перепутаете знак — прочитаете глубину зеркально отражённого места сцены.

Z переворачивать не нужно: `glam::camera::rh::proj::directx::orthographic` уже отображает глубину в `[0, 1]` — ровно тот диапазон, что хранит карта.

Вне frustum (`w ≤ 0` или `uv` вне `[0, 1)`) ничего записано не было — видимость равна 1.
Видимая часть пола укладывается в NDC `|x|, |y| ≤ 0.94`, так что граница коробки не даёт тёмной рамки.

## Сравнение глубин

Глубинная текстура привязывается не как цветная: `TextureSampleType::Depth` на стороне wgpu соответствует `texture_depth_2d` на стороне WGSL (у `texture_2d<f32>` другой тип привязки — `Float`; это видно прямо по реестру wgpu-types).

```rust
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        // A depth texture pairs with texture_depth_2d on
                        // the WGSL side; textureLoad returns its single
                        // f32 channel directly.
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
```

Вся проверка видимости — умножение матрицы на мировую позицию (её несёт varying из вершинного шейдера — та же позиция, которую двигает клавиша `M`), знакомый перевод в UV и одно сравнение:

```wgsl
fn shadow_visibility(world_pos: vec3<f32>) -> f32 {
    if (scene.shadows_on == 0u) {
        return 1.0;
    }
    let clip = light_view_proj * vec4<f32>(world_pos, 1.0);
    // An orthographic light always gives w = 1; the guard stays for the
    // day the light becomes perspective, where w <= 0 means "behind it".
    if (clip.w <= 0.0) {
        return 1.0;
    }
    let ndc = clip.xy / clip.w;
    // NDC -> map UV. The viewport wrote row 0 for NDC y = +1, so the
    // sample transform must send +1 back to v = 0: the Y scale is
    // negative. The depth map is not a color image; flipping V here is
    // the same relation any render target applies on write.
    let uv = ndc * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    // Outside the light frustum nothing was ever recorded: lit.
    if (uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0) {
        return 1.0;
    }
    let size = vec2<f32>(textureDimensions(SHADOW));
    let texel = vec2<i32>(uv * size);
    let stored = textureLoad(SHADOW, texel, 0);
    let reference = clip.z / clip.w;
    // glam's orthographic already maps z to [0, 1], the same range
    // the map stores: no z remap, a plain comparison.
    return select(1.0, 0.0, reference > stored + BIAS);
}
```

`reference` — своя глубина фрагмента по оси света, `stored` — глубина в текселе.
Фрагмент в тени, если он **дальше** записанного: `reference > stored + BIAS`, где `BIAS = 0.0` — первый, честный вариант, место для поправки обозначено.
(В wgpu есть и аппаратный путь — sampler с `compare`, который делает сравнение сам; мы остаёмся при `textureLoad`, чтобы сравнение было видно глазами.)

Формула освещения знакома по [Lambert](../../space-light/lambert/), меняется один множитель:

```wgsl
    // The shadow scales only the direct term; ambient survives.
    let visibility = shadow_visibility(input.world_pos);
    let color = scene.albedo * (scene.ambient + scene.intensity * d * visibility);
```

Ambient не зависит от видимости — тень не чёрная, а «своя» для каждой поверхности.

Основной проход — второй в том же `draw`, с собственным depth-attachment. Куб ближе пола, поэтому с `Less` порядок рисования перестаёт иметь значение. Floor и cube различаются только bind group с model-матрицей: у пола она всегда единичная, куб пересчитывается на GPU из той же модели.

## Клавиши и проверки

- **H** — тени вкл/выкл: исчезает только падающая тень, Lambert остаётся;
- **M** — куб по X: `0 → +1 → −1 → 0`.

Число для `M`: тень — проекция вдоль `L`, центр тени куба = `C − L·(C.y/L.y)`.
При `L.x/L.y = 0.5` и высоте 0.51 это `(−0.255, 0, −0.255)`: полметра высоты — четверть метра тени по `-X` и `-Z`.
Сдвиг куба на `+1` переносит тень на `(0.745, 0, −0.255)` — неподвижный пол остаётся receiver.

Именно сдвинутая поза ловит переворот Y: новый центр тени лежит в нижней половине изображения света, и забытый минус в формуле v отправит lookup в верхнюю половину карты — там глубина 1.0, «никого ближе», и пиксель ошибочно осветится.
На симметричной позе `x = 0` ошибка почти невидима — проверяйте ориентацию аналитически.

## Снимок

```sh
cargo run -p shadow-mapping
cargo test -p foundations-verify
```

![Кадр карты теней: куб над полом и его тень.](/results/shadow-mapping.png)

Тень куба на полу, у основания — зона контакта.
Интерактивно `H` снимает и возвращает тень, `M` сдвигает куб: тень переезжает вместе с ним, неподвижный пол остаётся receiver'ом.

![Маска видимости: пиксели без прямого света помечены красным.](/results/shadow-mapping-visibility.png)

Тот же кадр диагностической маской: чистым красным помечены пиксели, где shadow-сравнение сняло прямой свет.
Полоса на полу — сама тень; красные полосы на гранях куба — acne при `bias = 0` (состояние главы по умолчанию): сравнение проигрывает собственной глубине соседних текселей карты, разбор и лечение — в следующей главе.

Проверка зондирует пол и сверяет всю тень: точка вне тени равна чистому Lambert (код 148), а каждый пиксель, который переключает `H`, обязан держать ровно ambient-значение `0.05` (код 63) — и до, и после сдвига куба на `+1` диф с аналитическим следом тени совпадает (это же фиксирует и ориентацию V карты: перевёрнутый lookup затемнил бы не тот район пола).

## Проверьте модель

1. Почему у камеры света ортографическая проекция, а не перспективная?
2. Почему пол не рисуется в depth-проходе — и что появилось бы на нём при bias = 0, если бы рисовался?
3. Что лежит в текселе карты, куда куб не попал, и что видит фрагмент пола при сравнении с ним?
4. Куб сдвинули на +1 по X. Куда и почему ровно туда сместится центр тени?

::: details Решения и контрольные выводы

1. Направленный свет — параллельные лучи; ортокамера сохраняет их параллельными, и тень не зависит от расстояния до источника. Перспектива превратила бы свет в точечный.
2. Пол — чистый receiver: затенять некого. Будь он caster, его фрагменты сравнивали бы `reference` с собственной записанной глубиной соседних текселей — по всему полу проступил бы acne при `BIAS = 0`.
3. Очистное значение 1.0 («дальше всего»); `reference` пола по углам ≈ 0.38–0.70 < 1.0 — видимость 1, вне тени.
4. На `(0.745, 0, −0.255)`: тень — проекция вдоль `L`, сдвиг caster на единицу сдвигает проекцию на ту же единицу, а смещение от высоты (`−0.255`) не меняется.

:::

Одна карта, одно сравнение — и край тени получился ступенчатым, а на самом кубе полосы.
[Следующая глава](../shadow-pcf/) разбирает, откуда артефакты и что с ними делает PCF.
