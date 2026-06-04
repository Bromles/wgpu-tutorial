---
editLink: false
---

# MSAA

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/msaa)

**Что уже должно быть понятно:**

- нормали, освещение, instancing
- camera, depth buffer, render pipeline
- текстуры, bind groups

**Что появится в этой главе:**

- мультисэмплинг (MSAA): сглаживание краёв геометрии
- `sample_count: 4` в pipeline и текстурах
- `resolve_target` — автоматическое сведение (resolve) мультисэмпловой текстуры в обычную
- мультисэмпловые color и depth текстуры

**Итог:** сетка 3×3×3 кубов с гладкими краями без «лесенки»

---

Края треугольников выглядят ступенчатыми (aliasing) — пиксель либо принадлежит треугольнику, либо нет.
MSAA (Multisample Anti-Aliasing) сглаживает края, беря несколько сэмплов на пиксель и усредняя результат.

## Принцип

При `sample_count = 4` GPU берёт 4 сэмпла в каждом пикселе. Фрагментный шейдер вызывается один раз —
результат размножается на все сэмплы, попавшие внутрь треугольника. На границе треугольника часть сэмплов
попадёт внутрь, часть — наружу. Итоговый цвет пикселя — среднее всех сэмплов.

MSAA дешевле суперсэмплинга (SSAA), потому что шейдер выполняется только один раз на пиксель, а не на
каждый сэмпл.

## Pipeline: sample_count

В pipeline указываем количество сэмплов:

```rust
multisample: MultisampleState {
    count: 4,  // было 1
    mask: !0,
    alpha_to_coverage_enabled: false,
},
```

`count` — степень двойки: 1 (без MSAA), 2, 4, 8. 4 — стандартный выбор: хорошее сглаживание при
умеренных затратах.

## Мультисэмпловые текстуры

Цветовая и depth-текстуры тоже должны поддерживать мультисэмплинг:

```rust
// Color
let msaa_texture = ctx.device.create_texture(&TextureDescriptor {
    sample_count: 4,           // было 1
    usage: TextureUsages::RENDER_ATTACHMENT,
    format: ctx.surface_format,
    ..
});

// Depth
let depth_texture = ctx.device.create_texture(&TextureDescriptor {
    sample_count: 4,           // было 1
    format: TextureFormat::Depth32Float,
    usage: TextureUsages::RENDER_ATTACHMENT,
    ..
});
```

Обе текстуры используют `RENDER_ATTACHMENT` (без `TEXTURE_BINDING` — мультисэмпловые текстуры нельзя
сэмплировать напрямую). Размер совпадает с окном.

## Resolve

Мультисэмпловая текстура содержит 4 значения на пиксель. Surface ожидает обычную текстуру — по одному
значению на пиксель. Преобразование называется **resolve** и задаётся через `resolve_target`:

```rust
color_attachments: &[Some(RenderPassColorAttachment {
    view: &self.msaa_view,         // мультисэмпловая текстура
    resolve_target: Some(view),    // surface view — куда сводить
    ops: Operations {
        load: LoadOp::Clear(Color::BLACK),
        store: StoreOp::Store,
    },
    depth_slice: None,
})],
```

GPU автоматически сводит (resolves) мультисэмпловую текстуру в `resolve_target` после завершения
render pass. Не нужно делать это вручную.

## Ключевые отличия от обычного рендера

| Параметр | Без MSAA | MSAA ×4 |
|----------|----------|---------|
| Pipeline `sample_count` | 1 | 4 |
| Color texture `sample_count` | 1 | 4 |
| Depth texture `sample_count` | 1 | 4 |
| `resolve_target` | `None` | `Some(view)` |
| Потребление памяти | ×1 | ×4 (color + depth) |

Все три параметра `sample_count` (pipeline, color texture, depth texture) должны совпадать.

## Что получилось

27 кубов с гладкими краями. Сравните с любой предыдущей главой — «лесенка» на гранях исчезла.

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Поставить `sample_count: 1` во всех трёх местах — вернуться к «лесенке»
- Поставить `sample_count: 8` — ещё более гладкие края (если GPU поддерживает)
- Убрать `resolve_target` (вернуть `None`) — получить ошибку: мультисэмпловая текстура
  не может быть отображена напрямую

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/advanced/msaa)
