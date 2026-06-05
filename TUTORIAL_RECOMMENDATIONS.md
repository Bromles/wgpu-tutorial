# Рекомендации по развитию руководства

## Принципы подачи

1. **Один концепт — один визуальный результат.** Каждая глава добавляет одну новую идею, и читатель сразу видит
   результат на экране.

2. **Теория — ровно в тот момент, когда она нужна.** Не заранее, не потом.

3. **Минимум абстракций до первого результата.** Чем быстрее читатель увидит результат, тем лучше.

4. **WGSL — полноценный язык.** Rust-разработчику WGSL кажется знакомым, но ловушек достаточно
   (layout, alignment, @binding, отличия в типах).

5. **Каждая глава отвечает на вопрос «что я увижу в конце?».**

6. **Сначала объясняем — потом используем.** Глава не должна нарушать собственный принцип.

## Учебный каркас

Framework в `code/framework/`:

```rust
pub trait Example: 'static {
    fn init(ctx: &GpuContext) -> Self;
    fn resize(&mut self, ctx: &GpuContext, new_size: PhysicalSize<u32>);
    fn update(&mut self, ctx: &GpuContext, dt: Duration, input: &Input);
    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder);
}
```

Правила:
- `GpuContext` — публичные поля `wgpu`-типов, без обёрток
- Не добавлять: ECS, asset manager, scene graph, material system, renderer graph
- Допустимый минимум: окно, surface lifecycle, device/queue, resize, frame acquire/present, timing, input

## Шаблон каждой главы

```text
Что уже должно быть понятно:
- ...

Что появится в этой главе:
- ...

Итог:
- ...
```

В статье — inline-код. Полный код — по ссылке на репозиторий.

## Что уже сделано

- **Начало работы:** Создание окна, Инициализация wgpu, Первый треугольник
- **Модель данных GPU:** Шейдеры и WGSL, Вершинные буферы, Индексные буферы, Uniform и bind groups, Текстуры и сэмплеры
- **Математика:** Векторы и матрицы, Система координат (LaTeX)
- **3D и камера:** Трансформации MVP, Depth buffer, Камера, Instancing
- **Освещение:** Нормали и базовый свет, Материалы и множественные источники, Тени (shadow mapping)
- **Продвинутый рендер:** Render-to-texture и постпроцессинг, Несколько мешей, MSAA
- **Каркас:** `GpuContext`, `trait Example`, `Input`, `ControlFlow::Poll`, pollster
- **Глоссарий:** ~180 терминов по 17 категориям с обратными ссылками
- **Диаграммы:** 15+ SVG (normals, bind groups, shadows, wireframe cube, vertex/instance step, frustum, vectors, matrix, sphere, MSAA, additive blending, depth comparison, render-to-texture two-pass)
- **Типичные ошибки:** в каждой главе (hello-triangle, shaders, textures, transformations, depth-buffer, camera, instancing, lighting/basics, materials, shadows, render-to-texture, model-loading, vertex-buffers, index-buffers, uniform-bind-groups)
- **Перекрёстные ссылки:** кликабельные ссылки между главами
- **Структура:** CTA кнопка, sidebar с «Введение», wgpu 29.0 version indicator

---

## Глобальные улучшения

### Скриншоты результатов

Ни одного скриншота. Каждый «Итог» — только текст. Хотя бы по одному PNG на главу.

### Glossary — дополнить обратные ссылки

Сейчас обратные ссылки есть у ~10 терминов. Добавить ко всем остальным.

### Структурные мелочи

- Sidebar: glossary в файловой системе `docs/guide/glossary/`, но в sidebar под «Приложение» — несоответствие
- Нет nav bar (только sidebar)

---

## Следующие главы (новый контент)

### Приоритет 1: Дописать основную линейку

```
Normal Mapping
  └ visual result: плоский квадрат выглядит рельефным
  └ tangent space, normal map texture, TBN matrix
  └ аналог: LearnOpenGL Normal Mapping

HDR, tone mapping
  └ visual result: яркие источники не выжигают сцену
  └ RGBA16Float, tone mapping curve
```

### Приоритет 2: Лаборатории

```
Compute passes
  └ visual result: compute blur, particles
  └ compute pipeline, dispatch, storage buffers

Bloom
  └ visual result: светящиеся ореолы вокруг ярких объектов

Particles
  └ visual result: система частиц на compute shader
```

### Приоритет 3: PBR Track

```
PBR: Metallic-Roughness
  └ visual result: реалистичные материалы

IBL, Environment Maps
  └ visual result: отражения окружения

glTF Loading
  └ visual result: загруженная модель с PBR-материалами

Mini-viewer
  └ финальный проект
```

### Концептуальные страницы

```
Gamma, sRGB, linear color
  └ linked from: Textures, Lighting, HDR

WGSL layout и alignment
  └ linked from: Uniform bind groups, Storage buffers

Present modes и VSync
  └ linked from: Init wgpu

GPU/CPU synchronization
  └ linked from: compute passes, staging buffers
```
