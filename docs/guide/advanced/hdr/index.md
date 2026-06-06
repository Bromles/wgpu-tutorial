---
editLink: false
---

# HDR и Tone Mapping

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/hdr)

**Что уже должно быть понятно:**

- render-to-texture и постпроцессинг
- направленный свет, diffuse lighting
- текстуры и вершинные буферы

**Что появится в этой главе:**

- HDR (High Dynamic Range) — рендеринг в формате с плавающей запятой
- `Rgba16Float` — формат текстуры, хранящий значения $> 1.0$
- tone mapping — сжатие HDR → LDR для отображения
- кривая Reinhard и ACES
- два render pass: сцена → HDR текстура, тонмаппинг → экран

**Итог:** ярко освещённая сцена без выжженных белых областей — детали сохраняются и в светлых участках

---

## Проблема LDR

Обычный формат `Rgba8Unorm` хранит значения от 0 до 1. Если яркость фрагмента превышает 1.0,
он обрезается (clamps) до белого — детали пропадают. Яркий источник света и белая стена выглядят
одинаково. Реальный мир содержит диапазон яркостей от $10^{-3}$ (лунный свет) до $10^{6}$ (прямое солнце),
и обычный 8-битный формат не способен его передать.

## HDR-рендеринг

HDR (High Dynamic Range) — рендеринг в текстуру с форматом, поддерживающим значения больше 1.0.
В wgpu для этого используется `Rgba16Float` — 16-битный float на канал, диапазон $[-65504,\, 65504]$
с достаточной точностью для графики.

```rust
let hdr_texture = ctx.device.create_texture(&TextureDescriptor {
    format: TextureFormat::Rgba16Float,
    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    ...
});
```

Сцена рендерится в эту текстуру как обычно — но цвета могут выходить за пределы $[0,\,1]$.

## Почему HDR текстуру не покажешь на экране

Монитор отображает значения от 0 до 1 (LDR). HDR-изображение нужно преобразовать в LDR —
этот процесс называется **tone mapping**. Без него значения $> 1.0$ всё равно обрежутся при выводе.

## Tone mapping

Tone mapping — функция, сжимающая HDR-диапазон в $[0,\,1]$, сохраняя детали в ярких и тёмных участках.

### Reinhard

Простейшая кривая, используемая с 2002 года:

$$
L_{\text{out}} = \frac{L_{\text{in}}}{L_{\text{in}} + 1}
$$

```wgsl
fn reinhard(color: vec3<f32>) -> vec3<f32> {
    return color / (color + vec3<f32>(1.0));
}
```

Значение 0 → 0, 0.5 → 0.33, 1.0 → 0.5, 5.0 → 0.83, $\infty$ → 1.0. Плавная кривая без
жёсткой обрезки. Недостаток: яркие области выглядят «приглушёнными».

### ACES (Academy Color Encoding System)

Кривая, разработанная киноиндустрией. Лучше сохраняет контраст и насыщенность:

```wgsl
fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e),
                 vec3<f32>(0.0), vec3<f32>(1.0));
}
```

ACES даёт более кинематографичную картинку: яркие участки плавно уходят в белый,
а средние тона сохраняют насыщенность.

<img src="/diagrams/tone-mapping-curves.svg" alt="Сравнение кривых тоновой коррекции: Reinhard и ACES" style="width: 100%;" />

## Два render pass

Структура аналогична [Render-to-texture](/guide/advanced/render-to-texture/), но с другим
форматом offscreen-текстуры:

```
Pass 1: scene pipeline → Rgba16Float offscreen texture
Pass 2: post pipeline  → surface (LDR, tone mapping applied)
```

### Pass 1: сцена в HDR

Pipeline создаётся с цветовой целью `Rgba16Float` вместо surface format:

```rust
fragment: Some(FragmentState {
    targets: &[Some(ColorTargetState {
        format: TextureFormat::Rgba16Float, // HDR
        ...
    })],
    ...
}),
```

Шейдер умножает цвет на высокую интенсивность (`intensity = 3.0`), что даёт значения до ≈ 3.0
на ярко освещённых гранях — HDR-текстура это сохраняет:

```wgsl
let intensity = light.ambient + diffuse * light.intensity;
return vec4<f32>(tex_color.rgb * vec3<f32>(1.0, 0.95, 0.85) * intensity, 1.0);
```

### Pass 2: tone mapping на экран

Полноэкранный квад сэмплирует HDR-текстуру и применяет ACES:

```wgsl
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let hdr = textureSample(hdr_tex, hdr_sampler, input.uv);
    let mapped = aces(hdr.rgb);
    return vec4<f32>(mapped, 1.0);
}
```

Поверхность (`surface_format`) — обычно `Bgra8UnormSrgb`, которая автоматически конвертирует
линейные значения в sRGB для монитора. После tone mapping цвета линейны, и эта конвертация
корректна.

## Настройки сэмплера для HDR

HDR-текстура использует `FilterMode::Nearest`. `Rgba16Float` с линейной фильтрацией
поддерживается не на всех платформах — для максимальной совместимости:

```rust
let hdr_sampler = ctx.device.create_sampler(&SamplerDescriptor {
    mag_filter: FilterMode::Nearest,
    min_filter: FilterMode::Nearest,
    ..Default::default()
});
```

Адресация `ClampToEdge` по умолчанию — fullscreen quad не выходит за пределы $[0,\,1]$.

## Bind groups

Сцена использует две группы (как в предыдущих главах), постпроцессинг — одну:

| Pass | Group | Binding | Ресурс |
|------|-------|---------|--------|
| Scene | 0 | 0 | CameraUniforms |
| Scene | 1 | 0 | LightUniforms (light_dir, ambient, **intensity**) |
| Scene | 1 | 1 | diffuse texture |
| Scene | 1 | 2 | diffuse sampler |
| Post  | 0 | 0 | HDR texture (Rgba16Float) |
| Post  | 0 | 1 | HDR sampler |

## Resize

При изменении размера окна HDR-текстура пересоздаётся, а bind group пересоздаётся
с новой текстурой. Сэмплер и pipeline не меняются:

```rust
fn resize(&mut self, ctx: &GpuContext, _new_size: PhysicalSize<u32>) {
    let (hdr_tex, hdr_view) = Self::create_hdr_texture(ctx);
    self.hdr_texture = hdr_tex;
    self.hdr_texture_view = hdr_view;

    let hdr_bgl = self.post_pipeline.get_bind_group_layout(0);
    self.hdr_bind_group = ctx.device.create_bind_group(&BindGroupDescriptor {
        layout: &hdr_bgl,
        entries: &[
            BindGroupEntry { binding: 0,
                resource: BindingResource::TextureView(&self.hdr_texture_view) },
            BindGroupEntry { binding: 1,
                resource: BindingResource::Sampler(&self.hdr_sampler) },
        ],
        ...
    });
    // depth texture тоже пересоздаётся
}
```

## Типичные ошибки

::: warning Рендер в surface format вместо Rgba16Float
HDR-значения обрежутся до 1.0 ещё до tone mapping, и весь смысл HDR теряется.
:::

::: warning Tone mapping в сцене, а не в постпроцессинге
Если применить Reinhard/ACES прямо в сценовом шейдере, несколько перекрывающихся источников света дадут неверный результат: каждый clamp'ится независимо.
:::

::: warning Забыли пересоздать HDR bind group при resize
Старая bind group ссылается на текстуру не того размера → артефакты или падение.
:::

::: warning Linear filter для Rgba16Float без проверки
На некоторых GPU `Rgba16Float` не поддерживает линейную фильтрацию в textureSample. Используйте `Nearest` или проверяйте features.
:::

::: warning Не учитывают sRGB surface
Если surface format = `Bgra8UnormSrgb`, финальный вывод получает автоматическую gamma-коррекцию. Не нужно добавлять `pow(color, 1/2.2)` вручную.
:::

## Что получилось

Ярко освещённая сцена, где детали сохраняются и в светлых, и в тёмных участках. Без HDR яркие грани
выжигались бы в белое пятно — теперь tone mapping (ACES) плавно сжимает диапазон. Ключевая схема:
сцена рендерится в HDR-текстуру (`Rgba16Float`), затем полноэкранный квад применяет tone mapping
и выводит результат на LDR-поверхность. В следующих главах мы будем использовать HDR-рендеринг
как основу для bloom и других эффектов, работающих с яркостью за пределами обычного диапазона.

<!-- TODO: скриншот -->

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить `intensity` на 1.0 — без HDR разницы почти не видно
- Переключить ACES на Reinhard — более «приглушённый» результат
- Поставить `intensity` на 10.0 — яркие области выжигаются, но детали в них сохраняются благодаря tone mapping

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/hdr)
