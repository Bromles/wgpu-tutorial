---
title: "Собираем кадр"
description: "Явный граф: shadow depth → opaque HDR → transparent HDR → resolve → tone map → surface; роли mesh/material/object, прозрачная панель и переключатели H/M."
prev:
  text: "HDR и вывод"
  link: ../hdr-output/
next: false
---

# Собираем кадр

::: info Только native
Интеграция: куб над полом с тенями (стиль [«Карта теней»](../shadow-mapping/)), роли [главы «Геометрия, материал, объект»](../../space-light/scene-objects/), прозрачная панель [главы «Over и представление alpha»](../../space-light/blend-over/), HDR-вывод [главы «HDR и вывод»](../hdr-output/) и MSAA [главы «MSAA и resolve»](../msaa-resolve/). Требуется `Features::IMMEDIATES`. Клавиши: `H` — тени, `M` — MSAA.
:::

Ни одного нового механизма.
Кадр целиком — это уже не «ещё один приём», а **согласование**: кто в каком пространстве рисует, кто чей выход читает, в каком порядке компонуется и где единственное кодирование.

## Явный граф

Один `draw`, до четырёх проходов — shadow depth, opaque, transparent, tone map; при `H`-off прохода тени нет и остаются три. Все зависимости видны глазами:

```text
shadow depth ──┐
               ├→ opaque HDR ──→ transparent HDR ──→ resolve (при MSAA) ──→ tone map ──→ surface
      depth ───┘        (Lambert + видимость)          (панель, blend)                        (Reinhard, sRGB)
```

Каждая стрелка — текстура или вложение из предыдущих глав; ошибка согласования (формат, sample count, размер) здесь ловится валидацией, а не «странностью» кадра.

## Роли: mesh, material, object

Сцена — куб над полом — собрана из явных ролей:

- **mesh** — диапазон индексов в общей паре буферов: `CUBE_MESH = 0..36`, `FLOOR_MESH = 36..42`. Две геометрии, два `draw_indexed`;
- **material** — uniform по 16 байт и свой bind group: куб тёплый, пол сохраняет albedo 0.5 из [главы «Нормали и первый свет»](../../space-light/lambert/). Смена материала — перебиндовка, не перезапись;
- **object** — storage-таблица записей `model + normal_matrix` (по 128 байт), индекс записи едет в командном состоянии.

```rust
            // Mesh role: the draw range. Material role: the group.
            // Object role: the immediate index.
            for (object, mesh, material) in [
                (0u32, &CUBE_MESH, 0usize),
                (1u32, &FLOOR_MESH, 1usize),
            ] {
                pass.set_bind_group(1, &self.material_bind_groups[material], &[]);
                pass.set_immediates(0, bytemuck::bytes_of(&object));
                pass.draw_indexed(mesh.clone(), 0, 0..1);
            }
```

Immediates — механизм стандарта WebGPU/WGSL, но его поддержка зависит от адаптера: в браузерах он часто недоступен. Поэтому `init` отказывается стартовать без этой поддержки. Окно и offscreen-тест запрашивают устройство явно — `gpu_context_with` с `Features::IMMEDIATES` и `max_immediate_size: 4`:

```rust
        // The object index travels in the command state, so the feature
        // and its budget are checked before the first pipeline is built.
        if !gpu
            .device
            .features()
            .contains(wgpu::Features::IMMEDIATES)
        {
            return Err("This example requires Features::IMMEDIATES (native only): the object index travels in the command state".into());
        }
```

## Минимальная карта теней

Тени — в стиле [главы о карте теней](../shadow-mapping/), без украшений: карта 1024×1024, один постоянный bias, PCF выключен.
Проход глубины со стороны света не пишет ни одного цвета — и pipeline честно не имеет фрагментной стадии:

```rust
            // No fragment stage: the pass produces depth only, and a
            // pipeline without fragment shading is valid for exactly that.
            fragment: None,
```

Кастер один — куб: панель тень не отбрасывает, пол сам себя не затеняет.
Сравнение ручное: фрагмент проецирует свою мировую точку в clip-пространство света, читает один тексель карты и сравнивает глубины сам:

```wgsl
fn visibility(world_position: vec3<f32>) -> f32 {
    let light_clip = light_view_proj * vec4<f32>(world_position, 1.0);
    // Orthographic projection: w = 1, the divide would change nothing.
    let ndc = light_clip.xyz;
    // Y scale is negative: the viewport wrote row 0 for NDC y = +1.
    let uv = ndc.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5, 0.5);
    // Outside the light frustum nothing was ever recorded: lit.
    if (uv.x < 0.0 || uv.x >= 1.0 || uv.y < 0.0 || uv.y >= 1.0) {
        return 1.0;
    }
    // Size comes from the texture.
    let map_size = vec2<f32>(textureDimensions(SHADOW));
    let texel = vec2<i32>(uv * map_size);
    let closest = textureLoad(SHADOW, texel, 0);
    // glam maps z to [0, 1] already; the bias on the stored depth
    // moves past depth acne.
    return select(1.0, 0.0, ndc.z > closest + SHADOW_BIAS);
}
```

Три ловушки пути world → light → texel: z уже в `[0, 1]` — тот же диапазон, что хранит карта, повторного ремапа нет (это же про `directx::orthographic` проговаривает [глава о карте теней](../shadow-mapping/)); V-ось карты растёт вниз против NDC +Y — отрицательный масштаб `−0.5` переворачивает ось сразу при вычислении `uv`, поэтому guard симметричен по обеим осям и отсекает всё за пределами `[0, 1)`; вне frustum света нет записанной глубины — там видимость равна 1, иначе по краям выросла бы тёмная рамка.
Bias 0.0005 прибавляется к записанной глубине (`closest + SHADOW_BIAS`) — защита от acne квантованных глубин.

## Прозрачная панель

Панель — горизонтальный прямоугольник над кубом, плоский цвет: свет на неё не влияет, формула освещения не вызывается.
Всё её поведение — состояние pipeline:

```rust
                // Straight-alpha over into the HDR frame: the blend runs
                // in linear light, before tone mapping.
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(STRAIGHT),
                    write_mask: ColorWrites::ALL,
                })],
```

```rust
            // Depth test on, depth writes off: the panel hides behind
            // opaque geometry but never occludes anything itself.
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Less),
```

Порядок графа отвечает на два классических вопроса. **Когда blending?** В HDR-кадре, до tone map — композиция идёт в линейном свете, и полупрозрачный белый над фоном 0.1 даёт честные 0.55 linear. **Когда тонмаппинг?** Один раз, над уже сведённым кадром — панель не темнеет отдельно от подложки.

## MSAA: resolve один раз, в конце

Переключатель `M` вводит полный комплект [главы «MSAA и resolve»](../msaa-resolve/) — мультисэмпловые цвет и глубина, пайплайны count=4, resolve в односэмловый HDR-кадр — с одним уточнением:

```rust
        // Pass 3: the transparent panel over the opaque frame. Depth test
        // on, depth writes off; with MSAA the single resolve of the frame
        // happens here, after every scene pass is done.
```

Resolve стоит у **последнего** сценического прохода: opaque хранит мультисэмпловые данные (`Store`, без resolve), панель блендится прямо в сэмплы, и лишь затем кадр сводится один раз.
Карта теней всегда односэмпловая — у прохода глубины нет цвета, и сглаживать нечего.

## Переключатели

- `H` — тени. Меняется только видимость света: затенённый пиксель пола становится ровно таким же, как освещённый, — Lambert, интенсивность и панель не трогаются;
- `M` — MSAA. Меняются края, не значения: плоские внутренности поверхностей совпадают с count=1 (это закрепляет тест msaa_resolve; в полном кадре плоские пробы точны, края — относительны).

Экспозиция зафиксирована на 1.0 — глава про структуру графа, а не про настройку света.

## Снимок

```sh
cargo run -p full-frame
cargo test -p foundations-verify --test full_frame
```

![Итоговый кадр: тень, прозрачная панель и tone mapping.](/results/full-frame.png)

Итог главы: тень, прозрачная панель и tone mapping собраны в один явный граф проходов.
`H` и `M` — интерактивные переключатели: тень и MSAA меняют кадр, не меняя структуру графа.

Проверка закрепляет четыре точные пробы, «проба → ожидание»:

- верхняя грань куба (видимая, освещённая, `N·L = 0.912`, без затенения) — `[144, 131, 125] ± 1`;
- освещённый пол — полный Lambert-участок;
- затенённый пол — прямой член снят, остаётся ambient-уровень;
- центр прозрачной панели — линейный blend над HDR-фоном, затем Reinhard и однократное sRGB-кодирование.

Одна проба видит только свой участок цепочки: ошибка на этом участке (лишний ремап глубины, двойное кодирование, потерянный проход) срывает именно её, — вместе четыре пробы покрывают всю цепочку освещение → прозрачность → tone mapping → кодирование. Остальное проверяется относительно: в кадре есть яркое и тёмное, переключение `H` меняет хотя бы один пиксель, при `M=4×` меняются только края.

## Проверьте модель

1. Почему панель рисуется после opaque-прохода, но до tone mapper'а, а не наоборот?
2. Что изменится в кадре, если у pipeline панели включить `depth_write_enabled`?
3. Почему shadow map остаётся односэмпловой при count=4 у сцены?
4. Почему переключение `H` не может изменить пиксель панели?

::: details Решения и контрольные выводы

1. Blending должен идти в линейном HDR-свете: `0.5·панель + 0.5·подложка` — корректная линейная смесь; после tone map смешивались бы уже сжатые значения, и яркое подложечное просело бы сильнее тёмных. Tone map над сведённым кадром гарантирует и одно кодирование.
2. Панель начала бы закрывать глубину: последующая (будущая) геометрия за ней отсеклась бы depth-тестом — прозрачная поверхность не имеет права блокировать вывод.
3. У прохода глубины нет цветового вложения — resolve сводит только цвет; глубина от растеризатора одна на сэмпл и читается сравнением в сценическом шейдере.
4. Пиксель панели — результат смешивания плоского цвета с подложкой; тень меняет подложку только там, где панель не проецируется (центр панели смотрит на фон), а видимость в шейдере панели вообще не вычисляется.

:::

Явный граф собран: каждый проход знает свои вложения, каждое преобразование применяется один раз.
Здесь основной маршрут замыкается. Дальше — ответвления с явными входами, каждое независимо от остальных; их список — в разделе [«Что дальше»](/).
