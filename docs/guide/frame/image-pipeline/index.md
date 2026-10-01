---
title: "Fragment или compute: обработка изображения"
description: "Одна попиксельная операция двумя путями: fullscreen fragment-pipeline и compute с storage texture, textureStore, двухмерный dispatch с bounds check."
prev:
  text: "Промежуточный кадр: render-to-texture"
  link: ../render-to-texture/
next:
  text: "Карта теней"
  link: ../shadow-mapping/
---

# Fragment или compute: обработка изображения

::: info Только native
Снимок [«Промежуточный кадр»](../render-to-texture/): вход, разрешение, linear RGB и финальный показ прежние. Новое — развилка обработки и клавиша `P`.
:::

Промежуточный кадр — это просто данные, а данные можно обработать.
Самая простая операция над изображением — **независимая**, попиксельная: каждый тексель превращается сам по себе, соседи не нужны.
Такую операцию умеют делать оба механизма GPU — и это редкий случай, когда fragment- и compute-пути можно сравнить в честной лобовой схеме.

**Эксперимент:** вход — прямоугольник [главы «Тексели и upload»](../../foundations/texture-load/) в RGBA8Unorm; оба пути умножают **линейный** RGB каждого текселя на `0.5`, alpha сохраняют.
Fragment-путь — fullscreen-треугольник; compute-путь — storage texture и `textureStore`.
Клавиша `P` переключает, что показывает финальный fullscreen-проход; оба результата считаются каждый кадр.

```text
                ┌─ fragment halve ─→ fragment result ─┐
quad → input ───┤                                    ├─ copy → surface
                └─ compute halve ──→ compute result ──┘
```

## Fragment-путь

Тот же fullscreen-треугольник, что копировал кадр в главе о промежуточной текстуре, — только теперь между чтением и записью стоит операция:

```wgsl
@fragment
fn fs_halve(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(position.xy);
    let color = textureLoad(INPUT, texel, 0);
    // Linear light is halved; alpha passes through untouched.
    return vec4<f32>(color.rgb * 0.5, color.a);
}
```

Фрагментный invocation сам знает свой пиксель — `position`, адресация та же, что в главе о промежуточной текстуре: центр пикселя → тот же тексель, `textureLoad` уровня 0.
Пишет проход в **отдельный** результат — не в свой же вход.

## Storage texture

Compute-путь не растеризует: ему нужен другой выход — текстура, в которую шейдер пишет сам.
Это **storage texture**: привязка с доступом `write`, значение — те же четыре байта RGBA8Unorm:

```rust
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::Rgba8Unorm,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
```

Поддержку формата не угадываем, а спрашиваем: `Rgba8Unorm` с `STORAGE_BINDING` — базовая возможность WebGPU, без флагов адаптера, и код это фиксирует явно:

```rust
        // RGBA8Unorm storage textures are base WebGPU; the check makes the
        // contract explicit instead of failing later in validation.
        let storage_usages = TextureFormat::Rgba8Unorm
            .guaranteed_format_features(wgpu::Features::empty())
            .allowed_usages;
```

Usage'ы тоже другие: результат compute никогда не бывает render attachment — ему нужен `TEXTURE_BINDING | STORAGE_BINDING`:

```rust
        // The compute output is never rendered into: the storage binding
        // replaces the render attachment usage.
        self.compute_result = Some(create_intermediate(
            gpu,
            &self.frame_layout,
            "Compute result",
            width,
            height,
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
        ));
```

Контракт прежний: одно изображение не читается и не пишется в одном проходе.
Вход и выход — разные текстуры, оба пути только читают `input`, пишут каждый в свой результат.

## Compute-путь

Целый шейдер умещается в несколько строк:

```wgsl
@group(0) @binding(0) var INPUT: texture_2d<f32>;
@group(0) @binding(1) var RESULT: texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(8, 8)
fn halve(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(INPUT);
    if (id.x < size.x && id.y < size.y) {
        let texel = vec2<i32>(id.xy);
        let color = textureLoad(INPUT, texel, 0);
        textureStore(RESULT, texel, vec4<f32>(color.rgb * 0.5, color.a));
    }
}
```

Сопоставьте роли: fragment `position` — это compute `global_invocation_id`; invocation `(x, y)` обрабатывает тексель `(x, y)`.
Чтение то же (`textureLoad`, уровень 0), запись — `textureStore` вместо записи возвращаемого значения.

## Двухмерный dispatch

Группа теперь двумерная: `@workgroup_size(8, 8)` — 64 invocation'а на плитку 8×8 текселей.
И dispatch двумерный, с тем же округлением вверх, что и в главе [«Базовый compute»](../compute-basics/):

```rust
            // Rounded up to whole 8x8 groups; the shader bounds check
            // discards the surplus invocations.
            pass.dispatch_workgroups(width.div_ceil(8), height.div_ceil(8), 1);
```

Числа: при **641×359** — `⌈641/8⌉ × ⌈359/8⌉ = 81 × 45` групп: 648 × 360 = 233 280 запусков, из них 230 119 полезных, 3 161 отсекается bounds check'ом по обеим осям.
Стандартный размер офскрин-стенда 768×576 кратен 8: 96 × 72 группы, и проверка молчит — но убрать её нельзя: следующий нестандартный размер вернёт лишние invocation'ы, и без guard'а они вышли бы за границу `RESULT`.

Один нюанс на будущее: compute не получает неявных производных fragment-стадии, поэтому `textureSample` там недоступен; нужна фильтрация — есть `textureSampleLevel` с **явным** уровнем LOD.
Когда `textureSampleLevel` с nearest-сэмплером в центрах текселей даёт тот же целотексельный результат, что `textureLoad`, — тема ответвления об измерениях; в этой главе он не нужен.

## Снимок

```sh
cargo run -p image-pipeline
cargo test -p foundations-verify
```

Клавиша `P` переключает показ: кадры неотличимы.

![Два пути обработки: слева fragment, справа compute.](/results/image-pipeline-paths.png)

Одно и то же деление надвое, выполненное двумя путями: слева fullscreen fragment-проход, справа compute с `textureStore`.
Кадры неотличимы — в этом главный вывод главы: для попиксельной операции механизм не виден на экране.

Проверка — две пробы, «проба → что ловит»:

- разность экранов двух путей: нулевая, **абсолютная**, байт в байт — ловит любое расхождение самих путей;
- попиксельная сверка с моделью `encode(0.5 · input)`: квадранты из главы о текселях дают код 188, фон — 137, alpha везде 1 — ловит ошибку в величине операции; диагностический допуск 1/255 на канал покрывает UNORM-округление.
Это сравнение семантики, а не скорости: измерения — после главы про тайминги, и победитель здесь не назначается.

## Проверьте модель

1. Почему `RESULT` объявлен как `texture_storage_2d<rgba8unorm, write>`, а не `texture_2d<f32>`?
2. Сколько invocation'ов запустится при 641×359 и сколько из них сделает полезную работу?
3. Оба пути умножают RGB на 0.5. Почему код белого квадранта на экране — 188, а не 128?
4. Что мешает переписать compute-шейдер так, чтобы он читал и писал одну и ту же текстуру?

::: details Решения и контрольные выводы

1. `texture_2d<f32>` — sampled-текстура: только чтение целых или фильтрованных текселей. Storage texture — имя записи: формат и доступ (`write`) заданы в типе, и `textureStore` пишет ровно в него.
2. 81 × 45 групп по 64 invocation'а = 233 280 запусков; полезных 641 × 359 = 230 119, остальные 3 161 отсекает проверка `id.x < size.x && id.y < size.y`.
3. Умножение идёт в линейном свете: 1.0 → 0.5, и лишь при выводе поверхность кодирует: encode(0.5) ≈ 0.735 → код 188. 128 получилось бы при умножении готовых sRGB-кодов — это другая (и неправильная для света) операция.
4. Три независимых препятствия. Первое — привязка: наша `RESULT` объявлена как `write`-storage, и читать из неё в WGSL нельзя — ошибка валидации ещё до запуска. Второе — контракт языка: в базовом WebGPU/WGSL storage-текстуры доступны только на запись; чтение-и-запись — отдельное расширение языка и feature устройства, не всюду и не для всех форматов. Третье — алгоритм, и это главное: наш точечный проход трогает только свой тексель, но приём с соседями (например, размытие) читает тексели, которые параллельно пишут другие invocation'ы, — глобальный in-place-фильтр так не строится в принципе; как такие проходы разбивают и синхронизируют — тема ответвления о кооперации в compute. Поэтому вход и выход — разные текстуры: это работает везде и не требует ничего доказывать.

:::

Независимая попиксельная операция оказалась по силам обоим механизмам.
[Следующая глава](../shadow-mapping/) впервые потребует **второго взгляда на сцену**: depth-only проход с позиции света и карта теней.
