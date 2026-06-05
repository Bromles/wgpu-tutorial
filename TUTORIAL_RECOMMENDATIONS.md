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
- `required_features: adapter.features()` — запрашиваем все доступные фичи адаптера (native-only, браузер не нужен)
- Не добавлять: ECS, asset manager, scene graph, material system, renderer graph
- Допустимый минимум: окно, surface lifecycle, device/queue, resize, frame acquire/present, timing, input
- `ControlFlow::Wait` — event loop засыпает до нового события, мы сами запрашиваем перерисовку

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
- **Типичные ошибки** (блок `::: warning`)
- **Попробуем** (блок `tip`) — упражнения для самостоятельной работы
- **Скриншот результата** (или `<!-- TODO: скриншот -->` плейсхолдер)

## Что уже сделано

- **Начало работы:** Создание окна, Инициализация wgpu, Первый треугольник
- **Модель данных GPU:** Шейдеры и WGSL, Вершинные буферы, Индексные буферы, Uniform и bind groups, Текстуры и сэмплеры
- **Математика:** Векторы и матрицы, Система координат (LaTeX)
- **3D и камера:** Трансформации MVP, Depth buffer, Камера, Instancing
- **Освещение:** Нормали и базовый свет, Материалы и множественные источники, Тени (shadow mapping), Normal Mapping
- **Продвинутый рендер:** Render-to-texture и постпроцессинг, Несколько мешей, MSAA, HDR и tone mapping, Compute passes, Bloom, Particles
- **Каркас:** `GpuContext`, `trait Example`, `Input`, `ControlFlow::Wait`, pollster
- **Глоссарий:** ~180 терминов по 17 категориям с обратными ссылками
- **Диаграммы:** 28 SVG
- **Типичные ошибки:** в каждой главе
- **Перекрёстные ссылки:** кликабельные ссылки между главами
- **Структура:** CTA кнопка, sidebar с «Введение», wgpu 29.0 version indicator

---

## Глобальные улучшения

### Скриншоты результатов

Во все главы добавлены плейсхолдеры `<!-- TODO: скриншот -->`. Нужно запустить каждый пример и сделать PNG.

### Glossary — дополнить обратные ссылки

Сейчас обратные ссылки есть у ~10 терминов. Добавить ко всем остальным.

### Структурные мелочи

- Sidebar: glossary в файловой системе `docs/guide/glossary/`, но в sidebar под «Приложение» — несоответствие
- Нет nav bar (только sidebar)

---

## Исправленные баги кода

- **particles:** `ParticleData` — отсутствовал `_pad0` после `pos: [f32; 3]`, что ломало выравнивание WGSL `vec3<f32>` (align 16). Структура была 40 байт вместо ожидаемых WGSL 48
- **multiple-meshes:** `normal_matrix: [[f32; 3]; 3]` через encase давал 36 байт, а WGSL `mat3x3<f32>` ожидает 48 (column stride 16). Заменено на `Mat3` из glam
- **init-wgpu:** `required_features: adapter.features() & Features::default()` → `adapter.features()`, `get_default_config()` → ручное создание `SurfaceConfiguration` с выбором sRGB формата
- **framework:** `Features::empty()` → `adapter.features()`, `ControlFlow::Poll` → `ControlFlow::Wait`

### Потенциальные проблемы (не ломают работу, но стоит иметь в виду)

- **5 lighting/advanced примеров:** шейдеры объявляют normal-колонки как `vec4<f32>`, а Rust передаёт `Float32x3`. Работает, потому что GPU заполняет `.w` = 1.0, но семантически несовместимо
- **transformations:** backface culling включён без depth buffer — куб выглядит «вывернутым» при определённых углах. Это сделано сознательно (depth buffer в следующей главе), но не документировано

---

## Где стоит добавить диаграммы или расписать подробнее

### Диаграммы

| Глава | Что не хватает | Почему важно |
|-------|----------------|--------------|
| Uniform и bind groups | Диаграмма pipeline layout ↔ bind group layouts ↔ bind groups — сейчас три сущности описаны текстом, но визуальная схема связи сильно помогла бы | Читатели регулярно путают pipeline layout и bind group layout |
| Camera | Схема forward/right векторов относительно yaw — сейчас только текст и формула | Нагляднее, чем текстовое описание |
| Shadows | Диаграмма shadow UV transform: clip space → NDC → UV — сейчас только текст | Координатный трансформ — главная источник ошибок в shadow mapping |
| HDR | График Reinhard и ACES кривых — сейчас только формулы | Визуальное сравнение кривых проще для понимания |
| Normal mapping | Диаграмма TBN basis на поверхности — сейчас только таблица | Трёхмерная визуализация касательного пространства сложна для воображения |
| Bloom | Схема ping-pong между текстурами | Текстовое описание чередования текстур путает |

### Подробности

| Глава | Что расписать | Почему |
|-------|---------------|--------|
| init-wgpu | `surface_format` и sRGB — почему ищем sRGB, что будет если не найдём | Читатели спотыкаются о sRGB/linear при переходе к текстурам |
| Uniform и bind groups | Процесс `write_buffer` → staging buffer → GPU — подробнее раскрыть staging | Частая тема вопросов, ключ к пониманию CPU/GPU разделения |
| Lighting/basics | Почему 24 вершины — сейчас зовём из depth-buffer главы, но стоит показать визуально | Ключевой концепт для понимания нормалей |
| Textures | sRGB vs linear — при сэмплировании, при normal maps. Вынести в отдельный блок | Самая частая ошибка: Rgba8UnormSrgb для normal map |
| Compute passes | Разница `textureLoad` vs `textureSample` — когда какой использовать | compute-шейдеры не могут `textureSample` — частая ловушка |

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
