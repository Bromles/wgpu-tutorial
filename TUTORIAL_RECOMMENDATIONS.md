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
    fn resize(&mut self, _ctx: &GpuContext, _new_size: PhysicalSize<u32>) {}
    fn update(&mut self, _ctx: &GpuContext, _dt: Duration, _input: &Input) {}
    fn render(&mut self, ctx: &GpuContext, view: &TextureView, encoder: &mut CommandEncoder);
}
```

Правила:
- `GpuContext` — публичные поля `wgpu`-типов, без обёрток
- `required_features: adapter.features() - Features::all_experimental_mask() - Features::MAPPABLE_PRIMARY_BUFFERS` — запрашиваем все доступные не-экспериментальные фичи адаптера (native-only, браузер не нужен), кроме MAPPABLE_PRIMARY_BUFFERS (performance footgun на дискретных GPU)
- Не добавлять: ECS, asset manager, scene graph, material system, renderer graph
- Допустимый минимум: окно, surface lifecycle, device/queue, resize, frame acquire/present, timing, input
- `ControlFlow::Wait` — event loop засыпает до нового события, мы сами запрашиваем перерисовку

### Framework-модули

- `camera` — `Camera` struct (position, yaw, pitch, update, view_matrix)
- `texture` — `generate_checkerboard()`, `create_depth_texture()`
- `geometry` — `CUBE_POSITIONS`, `CUBE_NORMALS`, `CUBE_UVS`, `CUBE_INDICES`

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

Каждая глава содержит:
- **Типичные ошибки** (блок `::: warning`) — единообразно во всех главах
- **Попробуйте сами** (блок `::: tip`) — упражнения для самостоятельной работы
- **Скриншот результата** (или `<!-- TODO: скриншот -->` плейсхолдер)
- **Секция результата** — заголовок `## Что получилось` (не `## Итог`)

## Конвенции кода

### WGSL и vertex attributes

Типы в WGSL и Rust должны совпадать:
- `vec3<f32>` в WGSL → `VertexFormat::Float32x3` в Rust
- `vec4<f32>` в WGSL → `VertexFormat::Float32x4` в Rust

Не использовать `vec4<f32>` в шейдере, если Rust передаёт `Float32x3` — wgpu молча дополняет до vec4, но это сбивает с толку читателей.

### Имена переменных и полей

Полные описательные имена, без сокращений:
- `vertex_buffer`, `index_buffer`, `render_pass`, `compute_pass`
- `camera_uniform_buffer`, `light_uniform_buffer`
- `scene_texture`, `depth_texture`, `bright_texture`

Не использовать: `vb`, `ib`, `r`, `c`, `s`, `t`, `v`, `bg`, `ub`.

### Лейблы wgpu-ресурсов

Единообразные описательные лейблы с пробелами:
- `"Camera Uniform Buffer"`, `"Camera Bind Group Layout"`, `"Camera Bind Group"`
- `"Cube Vertex Buffer"`, `"Floor Index Buffer"`, `"Shadow Depth Texture"`
- `"Render Pipeline"`, `"Post Process Render Pipeline"`, `"Scene Render Pass"`

### encase

Использовать `.expect("Failed to write uniform buffer")` или `.expect("Failed to write storage buffer")` вместо `.unwrap()`.

## Что уже сделано

- **Начало работы:** Создание окна, Инициализация wgpu, Первый треугольник
- **Модель данных GPU:** Шейдеры и WGSL, Вершинные и индексные буферы, Uniform и bind groups, Текстуры и сэмплеры
- **Математика:** Векторы и матрицы, Система координат (LaTeX)
- **3D и камера:** Трансформации MVP, Depth buffer, Камера, Instancing
- **Освещение:** Нормали и базовый свет, Материалы и множественные источники, Тени (shadow mapping), Normal Mapping
- **Продвинутый рендер:** Render-to-texture и постпроцессинг, Несколько мешей, MSAA, HDR и tone mapping, Compute passes, Bloom, Particles
- **Каркас:** `GpuContext`, `trait Example`, `Input`, `ControlFlow::Wait`, pollster, `Camera`, `generate_checkerboard`, `create_depth_texture`, cube geometry
- **Глоссарий:** ~180 терминов по 17 категориям с обратными ссылками
- **Диаграммы:** 34 SVG
- **Типичные ошибки:** в каждой главе
- **Перекрёстные ссылки:** кликабельные ссылки между главами
- **Структура:** CTA кнопка, sidebar с «Введение», wgpu 30.0 version indicator

---

## Глобальные улучшения

### Скриншоты результатов

Во всех главах есть плейсхолдеры `<!-- TODO: скриншот -->`. Нужно запустить каждый пример и сделать PNG.

### Структурные мелочи

- Sidebar: glossary в файловой системе `docs/guide/glossary/`, но в sidebar под «Приложение» — несоответствие
- Нет nav bar (только sidebar)

---

## Потенциальные проблемы кода

- **transformations:** backface culling включён без depth buffer — документировано в статье
- **shadows:** double depth bias (pipeline `DepthBiasState` + shader-side `light_coords.z - 0.005`) — стоит добавить пояснение в статью о Peter Panning

---

## Аудит содержания статей — оставшиеся проблемы

### Концептуальные пробелы

- **Выбор texture format** — нет систематического руководства (когда `Rgba8Unorm`, когда `Rgba16Float`, когда sRGB). Частично покрыто в textures (sRGB vs linear warning block) и init-wgpu (sRGB details)
- **Error handling** — wgpu validation errors, device loss — не обсуждаются
- **Memory barriers / синхронизация** — wgpu handles implicitly, но не обсуждается

### Подробности — что стоит расписать

| Глава | Что расписать | Почему |
|-------|---------------|--------|
| Uniform и bind groups | Процесс `write_buffer` → staging buffer → GPU — подробнее раскрыть staging | Частая тема вопросов, ключ к пониманию CPU/GPU разделения |

---

## Качество демо-сцен

| Приоритет | Глава | Проблема | Предложение | Усилие |
|-----------|-------|----------|-------------|--------|
| medium | MSAA | Те же кубы — разница AA почти незаметна | Split-screen: левая половина MSAA=1, правая MSAA=4 | medium |
| small | Shadows | Минимальная сцена не раскрывает технику | Добавить высокий объект (столб) для драматичной тени | small |
| small | 9 глав | Одна и та же сцена «сетка кубов» для 60% руководства | Варьировать объекты, расстановку, окружение | medium |

---

## Следующие главы (новый контент)

### Приоритет 1: Концептуальные страницы

```
Gamma, sRGB, linear color
  └ linked from: Textures, Lighting, HDR
  └ без этого читатели будут ошибаться с форматами текстур

WGSL layout и alignment
  └ linked from: Uniform bind groups, Storage buffers
  └ vec3 выравнивание, struct size, array stride — главная источник багов

Present modes и VSync
  └ linked from: Init wgpu

GPU/CPU synchronization
  └ linked from: compute passes, staging buffers
```

### Приоритет 2: PBR Track

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
