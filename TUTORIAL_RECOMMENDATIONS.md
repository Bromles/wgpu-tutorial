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
- **Глоссарий:** ~150 терминов по 13 категориям
- **Багофиксы (ревью 2026-06):** `orthographic_rh_gl` → `orthographic_rh`, render-to-texture resize bind group,
  Chinese characters в lighting/basics, незакрытый backtick, `Features::default()` → `Features::empty()`,
  floor instance buffer в shadows, опечатка «аттрибуты», удалены `docs/examples/` и старый `glossary.md`

---

## Критические проблемы (требуют немедленного внимания)

### P0: Структурные ошибки в порядке глав

**Shadows ссылается на render-to-texture, который в sidebar идёт после.** `lighting/shadows` в пререквизитах указывает
render-to-texture, но в sidebar он в Advanced — после Lighting. Читатель по порядку ещё не знаком с offscreen rendering.

**Решение:** Переставить render-to-texture перед shadows (в раздел Освещение или в начало Продвинутого перед тенями).

### P0: Главы становятся короче по мере усложнения

| Глава | Строки |
|---|---|
| Textures | 408 |
| Transformations | 263 |
| Lighting: Basics | 191 |
| Shadows | 177 |
| Materials | 147 |
| Model loading | 151 |
| **MSAA** | **125** |

MSAA (125 строк) и Materials (147 строк) слишком коротки для своего концептуального веса. Нужно 200-250+ каждая.

### P1: Глава «Шейдеры и WGSL» объединяет две разные темы

Одновременно WGSL-справочник и vertex buffer tutorial. WGSL reference section стоит ПОСЛЕ кода — нарушение
принципа «сначала объясняем, потом используем».

**Решение:** Разделить на «WGSL как язык» (типы, функции, control flow) и «Вершинные данные» (data pipeline,
VertexBufferLayout, bytemuck).

### P1: `textures/index.md` молча подменяет bind group 0

В предыдущей главе `@group(0)` = uniform buffer. В textures — `@group(0)` = texture + sampler. Uniform buffer
исчез без объяснения.

**Решение:** Добавить переходный параграф, объясняющий подмену.

### `look_at_rh` и `look_to_rh` — необъяснённый разрыв

`transformations` объясняет `look_at_rh`, но код использует `look_to_rh`. Camera chapter переключается полностью.
Читатель не понимает, почему и когда использовать каждый вариант.

**Решение:** Явно объяснить разницу в transformations и camera.

---

## Диаграммы — главный пробел относительно LearnOpenGL

В главах Lighting и Advanced нет ни одной диаграммы. Критически нужны:

### Приоритет 1 (для существующих глав)

| Диаграмма | Где | Описание |
|---|---|---|
| N·L угол | lighting/basics | поверхность + нормаль + свет + угол θ |
| 3 нормали в одной вершине | lighting/basics | куб: угол, где сходятся 3 грани с разными нормалями |
| 2 bind groups архитектура | lighting/basics | camera group 0 + light group 1 |
| Shadow mapping принцип | lighting/shadows | frustum света → depth texture → сравнение глубин |
| Координатный трансформ | lighting/shadows | clip space в UV, инверсия Y, depth bias |
| Depth bias визуал | lighting/shadows | shadow acne, peter panning |
| Камера | 3d/camera | yaw, pitch, direction, forward, right |
| Wireframe куб | 3d/transformations | куб с пронумерованными вершинами |
| Vertex/instance step | 3d/instancing | паттерн чтения vertex и instance buffer |
| Frustum | math/coordinate-system | near/far/FOV |
| Векторное сложение | math/vectors-matrices | tip-to-tail геометрия |
| Матрица×вектор | math/vectors-matrices | числовой пример с 4×4 |
| 4 семпла в пикселе | advanced/msaa | coverage diagram |
| Sphere stacks/slices | advanced/model-loading | широта/долгота на сфере |

### Приоритет 2 (улучшение существующих)

- render-to-texture: двухпроходная архитектура (scene → offscreen, post → screen)
- depth-buffer: «wrong vs right» визуал без/с depth test
- index-buffers: vertex cache hit rate при переиспользовании индексов

---

## Недостающее содержимое по главам

### init-wgpu
- Что такое «backend» (Metal/Vulkan/DX12) — фундаментальное понятие
- VSync — хотя упомянуть
- `new_without_display_handle()` — объяснить или заменить на `Default`

### hello-triangle
- Pipeline создаётся один раз и immutable — это фундаментальное отличие от OpenGL
- `include_wgsl!` проверяет синтаксис WGSL на этапе компиляции

### shaders (WGSL)
- Предупреждение о `vec3<f32>` alignment (16 байт в WGSL структурах)
- Почему `#[repr(C)]` (GPU ожидает плоский предсказуемый layout)
- WGSL address spaces: `uniform`, `storage`, `private`, `workgroup`

### textures
- UV (0,0) = верхний левый, clip space Y направлен вверх — контраст нужно проговорить явно
- Мипмаппинг (хотя 3-4 предложения)
- gamma/sRGB (с forward-reference к lighting)
- Переходный параграф: bind group 0 сменился

### uniform-bind-groups
- Заменить magic numbers (2.094, 4.189) на именованные константы
- WGSL address spaces обзор

### math/vectors-matrices
- Убрать dot product / cross product в lighting/basics (где реально используются)
- Убрать MVP preview (дублирует transformations)
- Добавить identity matrix

### math/coordinate-system
- Числовой пример: вершина через все пространства
- NDC Z range [0,1] в WebGPU, а не [-1,1] как в OpenGL — нужно проговорить явно

### 3d/transformations
- Wireframe куб с номерами вершин (SVG)
- Объяснить winding order computation (GPU cross product)
- Ассоциативность умножения — пояснить

### 3d/depth-buffer
- «Wrong vs right» визуал без depth buffer
- Z-fighting — хотя упомянуть
- `TEXTURE_BINDING` флаг без нужды — убрать или объяснить
- `StoreOp::Store` / `Discard` — когда использовать каждый
- Почему `Depth32Float`, а не `Depth24Plus`

### 3d/camera
- Вывод формулы direction из yaw/pitch (из сферических координат)
- Объяснить `Input` struct происхождение
- Почему `pitch -= dy` (инверсия Y)
- Почему минусы в `-yaw.sin()`, `-yaw.cos()` (camera looks along -Z)

### 3d/instancing
- Диаграмма: как GPU читает vertex buffer (step Vertex) и instance buffer (step Instance)
- `Pod`/`Zeroable` для вершинных данных, `ShaderType`/`encase` для uniform — когда какой подход
- Почему `[[f32; 4]; 4]` вместо `Mat4` (Pod constraint)

### lighting/basics
- Код vertex shader с `light_pos` для normal matrix computation
- Спекулярный свет — хотя упомянуть «будет позже»
- Bind group architecture диаграмма

### lighting/materials
- Название обещает «материалы», но в главе только diffuse texture + несколько источников света
- Расширить до 200+ строк
- WGSL struct alignment для `[Light; 3]` массива
- Добавить диаграмму аддитивного смешивания

### lighting/shadows
- Показать данные пола (vertex/index)
- Код vertex shader с `light_pos` вычислением
- PCF упоминание
- `fragment: None` объяснение (depth-only rendering)
- Peter Panning (артефакт от excess bias)

### advanced/render-to-texture
- Объяснить `FilterMode::Nearest` выбор
- Cross-reference к MSAA (offscreen + multisampling)

### advanced/model-loading
- Переименовать директорию `model-loading` → `multiple-meshes` (glTF/OBJ не загружается)
- Исправить sphere generation code (в статье переменные без определения)
- Обсудить дублирование `view_proj` в каждом меше, альтернатива — общий camera bind group

### advanced/msaa
- Расширить до 200+ строк
- Before/after визуал (aliased vs anti-aliased edge)
- Другие AA: FXAA, TAA
- Cross-reference к render-to-texture
- `alpha_to_coverage_enabled` объяснить или убрать

---

## Глобальные улучшения

### «Типичные ошибки» — одна из сильных сторон LearnOpenGL

Добавить секцию в каждую главу. Примеры:

| Глава | Ошибка |
|---|---|
| hello-triangle | Pipeline immutable, нельзя менять после создания |
| shaders | `vec3<f32>` в WGSL struct = 16 bytes alignment |
| textures | UV (0,0) = верхний левый, не нижний |
| uniform-bind-groups | 16KB limit на uniform buffer |
| transformations | near=0 → perspective projection panic |
| camera | `look_to_rh` panics если direction = Vec3::ZERO |
| shadows | `orthographic_rh_gl` вместо `orthographic_rh` → неправильный Z range |
| MSAA | sample_count не поддерживается → panic |

### Перекрёстные ссылки — сделать кликабельными

Сейчас все ссылки на другие главы — текстовые, без URL. Примеры где нужно исправить:

- transformations: «как в главе про uniform-буферы» → link
- transformations: «глава Векторы и матрицы» / «Система координат» → links
- vertex-buffers: «следующая глава» → link
- index-buffers: «в главе про освещение» → link
- materials: «В прошлой главе» → link
- shaders: «в главе про instancing» → link

### Скриншоты результатов

Ни одного скриншота. Каждый «Итог» — только текст. Хотя бы по одному PNG на главу.

### Glossary — добавить недостающие термины

- MSAA, sample count, resolve target
- Shadow map, shadow pass, depth bias, PCF, Peter Panning
- Specular lighting
- Mipmap, LOD
- Compute shader, compute pass, workgroup, dispatch
- PBR, roughness, metallic, IBL
- Post-processing, framebuffer, bloom, kernel

### Glossary — добавить обратные ссылки

Каждый термин должен ссылаться на главу, где он введён.

### Структурные мелочи

- Landing page: убрать обещания ненаписанных глав (или отметить «в разработке»)
- Landing page: добавить CTA кнопку «Начать обучение»
- Sidebar: первый items group без `text` поля — добавить `"Введение"`
- Sidebar: glossary в файловой системе `docs/guide/glossary/`, но в sidebar под «Приложение» — несоответствие
- Добавить wgpu version indicator (сейчас `wgpu = "29.0"`)
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
