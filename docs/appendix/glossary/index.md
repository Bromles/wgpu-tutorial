---
prev: false
next: false
editLink: false
---

# Глоссарий

Термины, встречающиеся в руководстве. Английские написания даны там, где они устоялись в русскоязычном геймдеве.

## Графический конвейер

**Render pipeline (графический конвейер)** — главная сущность рендера: описывает всё состояние GPU при отрисовке —
шейдеры, форматы, примитивы, смешивание. Создаётся один раз и используется каждый кадр.
См. [Первый треугольник](/guide/getting-started/hello-triangle/).

**Вершинный шейдер (vertex shader)** — программа на WGSL, вызываемая один раз для каждой вершины; преобразует данные
вершины в позицию в clip space. Единственная обязательная стадия конвейера.
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

**Фрагментный шейдер (fragment shader / pixel shader)** — программа на WGSL, вызываемая один раз для каждого фрагмента (
пикселя); возвращает итоговой цвет.
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

**Сборка примитивов (primitive assembly)** — стадия конвейера, на которой GPU группирует вершины в геометрические
примитивы (треугольники, линии, точки).
См. [Первый треугольник](/guide/getting-started/hello-triangle/).

**Растеризация (rasterization)** — стадия конвейера, на которой GPU определяет, какие пиксели (фрагменты) находятся
внутри примитива, и вычисляет интерполированные значения для каждого.
См. [Первый треугольник](/guide/getting-started/hello-triangle/).

**Фрагмент (fragment)** — потенциальный пиксель: точка экрана, попавшая внутрь растеризуемого примитива. Фрагмент
становится пикселем после прохождения тестов глубины и трафарета.
См. [Depth buffer](/guide/3d/depth-buffer/).

**Интерполяция (interpolation)** — автоматическое плавное вычисление значений `@location` между вершинами для каждого
фрагмента. По умолчанию — перспективно-корректная.
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

**Draw call (вызов отрисовки)** — команда GPU на отрисовку геометрии: `draw` (по вершинам) или `draw_indexed` (по
индексам).
См. [Вершинные буферы](/guide/gpu-data-model/vertex-buffers/).

**Render pass (проход рендера)** — операция рендера внутри `CommandEncoder`; описывает цветовые вложения, операцию
загрузки/сохранения и все команды отрисовки.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**Color attachment (цветовое вложение)** — текстура (`TextureView`), в которую render pass записывает цвет каждого
пикселя.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

## Координаты

**Clip space (пространство отсечения)** — система координат от -1 до 1, в которой вершинный шейдер выдаёт позицию. GPU
автоматически переводит clip space в пиксели экрана. Левый нижний угол = (-1, -1), правый верхний = (1, 1).
См. [Система координат](/guide/math/coordinate-system/).

**NDC (Normalized Device Coordinates)** — нормализованные координаты устройства, получаемые после перспективного деления.
Значения от −1 до 1 по каждой оси.
См. [Система координат](/guide/math/coordinate-system/).

**Перспективное деление (perspective division)** — деление x, y, z на компоненту w вектора `vec4` для перехода из
однородных координат в NDC. См. [Система координат](/guide/math/coordinate-system/).

**Однородные координаты (homogeneous coordinates)** — представление точки четырёхкомпонентным вектором `(x, y, z, w)`.
Для точек `w = 1`, для направлений `w = 0`.
См. [Векторы и матрицы](/guide/math/vectors-matrices/).

**Object space (пространство объекта)** — локальная система координат, в которой определена геометрия объекта.
См. [Система координат](/guide/math/coordinate-system/).

**World space (мировые координаты)** — глобальная система координат сцены; model-матрица переводит object space в world space.
См. [Трансформации MVP](/guide/3d/transformations/).

**View space (пространство камеры)** — система координат, в которой камера находится в начале и смотрит вдоль −Z.
См. [Трансформации MVP](/guide/3d/transformations/).

**Screen space (экранные координаты)** — итоговые координаты в пикселях экрана.
См. [Система координат](/guide/math/coordinate-system/).

**Правосторонняя система координат (right-handed coordinate system)** — система, в которой +X вправо, +Y вверх, +Z к
наблюдателю (−Z от наблюдателя). См. [Система координат](/guide/math/coordinate-system/).

## Шейдеры и WGSL

**WGSL (WebGPU Shading Language)** — язык программирования шейдеров для WebGPU с синтаксисом, похожим на Rust.
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

**Swizzling** — способ обращения к компонентам вектора через `.xyzw` или `.rgba` (например, `v.xyz`, `c.rgb`).
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

**Entry point (точка входа)** — имя функции в шейдерном модуле, с которой начинается выполнение конкретной стадии (
например, `vs_main`).
См. [Шейдеры и WGSL](/guide/gpu-data-model/shaders/).

## Буферы

**Вершинный буфер (vertex buffer)** — массив структур в видеопамяти, описывающих каждую вершину (позиция, цвет, нормаль
и т.д.). См. [Вершинные буферы](/guide/gpu-data-model/vertex-buffers/).

**Индексный буфер (index buffer)** — массив целых чисел, ссылающихся на позиции вершин в вершинном буфере; устраняет
дублирование общих вершин. См. [Индексные буферы](/guide/gpu-data-model/index-buffers/).

**Uniform-буфер (uniform buffer)** — GPU-буфер, доступный шейдерам на протяжении всего draw call; обновляется каждый
кадр. Данные — общие для всего вызова отрисовки, в отличие от вершинного буфера, где данные привязаны к конкретным
вершинам. См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Instance buffer (буфер экземпляров)** — буфер, хранящий данные для каждого экземпляра при инстансинге (model-матрица,
normal matrix и т.д.). GPU считывает один элемент на экземпляр (`VertexStepMode::Instance`).
См. [Instancing](/guide/3d/instancing/).

**Storage buffer** — буфер с произвольным доступом из шейдера (`var<storage>`); поддерживает чтение и запись. Не имеет
ограничения 16 KB как uniform-буфер.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Staging buffer (промежуточный буфер)** — временный буфер в памяти, доступной и CPU, и GPU; используется
`queue.write_buffer()` для передачи данных из оперативной памяти в видеопамять.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

## Геометрия

**Топология примитивов (primitive topology)** — способ группировки вершин в примитивы: `TriangleList` (каждые 3
вершины = отдельный треугольник), `LineList`, `PointList`, `TriangleStrip` и т.д.
См. [Первый треугольник](/guide/getting-started/hello-triangle/).

**Winding order (порядок обхода)** — направление перечисления вершин треугольника (по или против часовой стрелки),
определяющее, какая сторона грани считается передней.
См. [Трансформации MVP](/guide/3d/transformations/).

**Backface culling (отсечение задних граней)** — отбрасывание треугольников, развёрнутых к наблюдателю задней стороной,
до растеризации. Экономит GPU-время на замкнутых 3D-объектах.
См. [Трансформации MVP](/guide/3d/transformations/).

**Base vertex (базовое смещение вершины)** — число, добавляемое к каждому индексу при `draw_indexed`; позволяет
адресовать разные объекты в общем вершинном буфере без перестроения индексов.
См. [Несколько мешей](/guide/advanced/multiple-meshes/).

**Instancing (инстансинг)** — механизм отрисовки нескольких экземпляров одной геометрии за один draw call с разными
данными на экземпляр. См. [Instancing](/guide/3d/instancing/).

## Bind groups и uniform

**Bind group layout (layout группы привязки)** — «контракт», описывающий формат привязок (какие ресурсы, каких типов,
каким стадиям доступны), но не конкретные данные. Позволяет менять данные, не пересоздавая pipeline.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Bind group (группа привязки)** — «экземпляр контракта»: связывает bind group layout с реальными буферами/текстурами.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Binding (привязка)** — индекс внутри bind group, соответствующий `@binding(N)` в шейдере.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Pipeline layout (layout конвейера)** — описание того, какие bind group layouts использует конвейер; связывает ресурсы
с шейдерами.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

**Выравнивание / паддинг (alignment / padding)** — правило WGSL: uniform-структуры выровнены на 16 байт; недостающие
байты заполняются нулями (padding). Крейт `encase` вычисляет выравнивание автоматически.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

## Текстуры

**Texture (текстура)** — двумерный массив текселей (пикселей) в видеопамяти. Может быть источником данных для
шейдера или целью рендера.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**Texture view (представление текстуры)** — «окно» в текстуру, определяющее, как шейдер обращается к ней. Одна текстура
может иметь несколько view с разными форматами или поддиапазонами.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**Sampler (сэмплер)** — объект, описывающий, как GPU читает тексели из текстуры при заданных UV-координатах:
фильтрация (nearest / linear), режим адресации (repeat / clamp-to-edge), уровни mip.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**UV-координаты (texture coordinates)** — двумерные координаты (u, v) от 0.0 до 1.0, привязывающие каждую вершину к
определённой точке текстуры.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**Texel (тексель)** — один пиксель текстуры (texture element).
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**sRGB / гамма-коррекция** — нелинейное цветовое пространство, в котором хранятся изображения. Форматы с суффиксом
`Srgb` (например, `Rgba8UnormSrgb`) автоматически преобразуют sRGB → linear при чтении в шейдере.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

## Поверхность и отображение

**Framebuffer** — текстура поверхности, куда записывается итоговое изображение для вывода на экран.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**Двойная буферизация (double buffering)** — использование двух текстур: одна отображается на экране (front), вторая —
скрытая (back), в неё рисуется новый кадр; затем они меняются местами (swap).
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**Swapchain** — механизм обмена front/back буферов; в старых API — отдельный объект, в wgpu скрыт внутри `Surface`.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**VSync (вертикальная синхронизация)** — синхронизация частоты кадров с частотой обновления монитора; предотвращает
разрывы изображения.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

## Приложение и окно

**Event loop (цикл обработки событий)** — основной цикл приложения, в котором операционная система доставляет события (
ввод, resize, redraw) в обработчики.
См. [Создание окна](/guide/getting-started/creating-window/).

**Render loop (цикл отрисовки)** — самоподдерживающийся цикл перерисовки: `RedrawRequested` → отрисовка →
`request_redraw()` → снова `RedrawRequested`.
См. [Создание окна](/guide/getting-started/creating-window/).

## GPU

**GPU / Видеокарта** — отдельное устройство со своим процессором и памятью (VRAM), работающее параллельно с CPU и
выполняющее графические команды.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**Backend (бэкенд)** — конкретный графический API, через который wgpu общается с видеокартой: Metal (macOS), Vulkan,
DirectX 12.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

**Queue (очередь команд)** — очередь GPU, через которую отправляются записанные `CommandBuffer` на выполнение.
См. [Инициализация wgpu](/guide/getting-started/init-wgpu/).

## Трансформации

**MVP (Model-View-Projection)** — три матрицы, последовательно преобразующие вершину из object space в clip space:
`projection × view × model × vertex`.
См. [Трансформации MVP](/guide/3d/transformations/).

**Model-матрица** — переводит координаты из object space в world space; задаёт позицию, поворот и масштаб объекта.
См. [Трансформации MVP](/guide/3d/transformations/).

**View-матрица** — перестраивает мировые координаты так, чтобы камера оказалась в начале координат и смотрела вдоль −Z.
См. [Трансформации MVP](/guide/3d/transformations/).

**Projection-матрица** — проецирует 3D-пространство на 2D-плоскость с учётом перспективы (или без неё — ортографическая).
См. [Трансформации MVP](/guide/3d/transformations/).

**FOV (Field of View)** — угол обзора камеры; определяет, какая часть сцены видна. Обычно 45°–90°.
См. [Камера](/guide/3d/camera/).

**Near / Far clipping plane (ближняя / дальняя плоскость отсечения)** — объекты ближе near или дальше far не видны;
определяют диапазон глубины projection-матрицы.
См. [Камера](/guide/3d/camera/).

**Column-major order** — способ хранения матрицы в памяти: столбцы идут подряд. Используется в WGSL и OpenGL.
`Mat4::to_cols_array_2d()` из glam возвращает данные в этом формате.
См. [Uniform и bind groups](/guide/gpu-data-model/uniform-bind-groups/).

## Камера

**Yaw** — поворот камеры вокруг оси Y (горизонтальный обзор).
См. [Камера](/guide/3d/camera/).

**Pitch** — наклон камеры вверх/вниз.
См. [Камера](/guide/3d/camera/).

**Euler angles (углы Эйлера)** — три угла (yaw, pitch, roll), описывающие ориентацию объекта в пространстве.
См. [Камера](/guide/3d/camera/).

**Eye position** — позиция камеры в мировых координатах.
См. [Камера](/guide/3d/camera/).

**Up vector (вектор «вверх»)** — направление, считающееся вертикалью; обычно `Vec3::Y` (ось Y).
См. [Камера](/guide/3d/camera/).

**MSAA (Multisample Anti-Aliasing)** — метод сглаживания краёв геометрии: GPU берёт несколько
сэмплов на пиксель, но фрагментный шейдер вызывается один раз. См. [MSAA](/guide/advanced/msaa/).

**Sample count** — количество субпиксельных сэмплов при MSAA (1, 2, 4, 8). Должно совпадать в pipeline,
color texture и depth texture.
См. [MSAA](/guide/advanced/msaa/).

**Resolve target** — текстура, в которую GPU автоматически сводит мультисэмпловые данные после render pass.
См. [MSAA](/guide/advanced/msaa/).

## Освещение

**Нормаль (normal)** — единичный вектор, перпендикулярный поверхности. Определяет, куда «смотрит» грань.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Normal matrix** — матрица для корректного преобразования нормалей: $(M^{-1})^T$, где $M$ — верхняя левая
подматрица 3×3 model-матрицы.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Направленный свет (directional light)** — источник света бесконечно далеко (как солнце); все лучи параллельны.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Diffuse lighting (диффузное освещение)** — яркость грани пропорциональна $\cos$ угла между нормалью и направлением
света: $\max(0, \vec{N} \cdot \vec{L})$.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Ambient (фоновое освещение)** — минимальная яркость, добавляемая ко всем граням, чтобы они не были полностью чёрными.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Specular lighting (зеркальное освещение)** — яркие блики на гладких поверхностях, зависящие от угла между
направлением отражённого света и направлением на камеру.
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Normal map (карта нормалей)** — текстура, хранящая векторы нормалей в касательном пространстве;
RGB-каналы кодируют $(x, y, z)$ из $[-1,\,1]$ как $[0,\,255]$. Позволяет плоской поверхности выглядеть рельефной.
См. [Normal Mapping](/guide/lighting/normal-mapping/).

**Tangent space (касательное пространство)** — локальная система координат поверхности: ось X = tangent (вдоль U),
ось Y = bitangent (вдоль V), ось Z = normal. Normal map хранит нормали именно в этом пространстве.
См. [Normal Mapping](/guide/lighting/normal-mapping/).

**TBN-матрица** — матрица $3\times3$, переводящая нормаль из tangent space в world space;
столбцы — векторы Tangent, Bitangent, Normal.
См. [Normal Mapping](/guide/lighting/normal-mapping/).

**Tangent (касательный вектор)** — единичный вектор на поверхности, направленный вдоль оси $U$ текстурных координат.
Передаётся как вершинный атрибут и используется для построения TBN-матрицы.
См. [Normal Mapping](/guide/lighting/normal-mapping/).

**Bitangent (бинормаль)** — вектор, перпендикулярный tangent и normal; вычисляется как $\vec{N} \times \vec{T}$.
Вместе с tangent и normal образует базис касательного пространства.
См. [Normal Mapping](/guide/lighting/normal-mapping/).
См. [Нормали и базовый свет](/guide/lighting/basics/).

**Аддитивное смешивание (additive blending)** — вклад нескольких источников света складывается;
яркости суммируются. См. [Материалы и свет](/guide/lighting/materials/).

## Shadow mapping

**Shadow map** — depth-текстура, отрендеренная с точки зрения источника света; используется для определения,
находится ли фрагмент в тени. См. [Тени](/guide/lighting/shadows/).

**Shadow pass** — первый render pass в shadow mapping; рисует только глубину сцены с позиции источника света.
См. [Тени](/guide/lighting/shadows/).

**Depth bias** — сдвиг глубины при записи в shadow map для предотвращения shadow acne (самозатенения).
См. [Тени](/guide/lighting/shadows/).

**Shadow acne** — артефакт в виде полос на освещённых поверхностях; возникает без depth bias.
См. [Тени](/guide/lighting/shadows/).

**Peter Panning** — артефакт, при котором тень «отрывается» от объекта из-за слишком большого depth bias.
См. [Тени](/guide/lighting/shadows/).

**PCF (Percentage Closer Filtering)** — метод сглаживания краёв теней через множественные сэмплы shadow map.
См. [Тени](/guide/lighting/shadows/).

**Comparison sampler** — сэмплер с функцией сравнения (`compare: Some(...)`);
используется с `textureSampleCompare` для shadow mapping.
См. [Тени](/guide/lighting/shadows/).

## Depth buffer

**Depth buffer (буфер глубины)** — текстура, хранящая глубину каждого пикселя. GPU сравнивает глубину нового фрагмента
с записанной и отбрасывает фрагменты, скрытые другими. См. [Depth buffer](/guide/3d/depth-buffer/).

**Depth test** — сравнение глубины фрагмента с depth buffer; определяет, виден ли фрагмент.
См. [Depth buffer](/guide/3d/depth-buffer/).

**Depth write** — запись глубины фрагмента в depth buffer после прохождения depth test.
См. [Depth buffer](/guide/3d/depth-buffer/).

**Z-fighting** — мерцание на границе двух поверхностей с почти одинаковой глубиной; возникает из-за
ограниченной точности depth buffer. Устраняется увеличением near plane или разделением объектов.
См. [Depth buffer](/guide/3d/depth-buffer/).

**Frustum (усечённая пирамида)** — область видимости камеры, определяемая near, far и FOV.
Объекты за пределами frustum не рисуются. См. [Система координат](/guide/math/coordinate-system/).

**StoreOp::Store / Discard** — определяет, сохраняются ли данные вложения после render pass.
`Store` — сохраняются (нужно для shadow map, offscreen). `Discard` — отбрасываются для экономии памяти.
См. [Depth buffer](/guide/3d/depth-buffer/).

## Текстуры (продолжение)

**Mipmap** — цепочка уменьшенных копий текстуры (каждая в 2 раза меньше). GPU автоматически выбирает
подходящий уровень детализации (LOD), предотвращая мерцание при минификации.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

**LOD (Level of Detail)** — уровень детализации текстуры; mipmap level 0 = оригинал, level 1 = половина и т.д.
См. [Текстуры и сэмплеры](/guide/gpu-data-model/textures/).

## Рендеринг

**Offscreen rendering / Render-to-texture** — рендер сцены в текстуру, а не на экран. Используется для
постпроцессинга, shadow mapping, reflection и т.д. См. [Render-to-texture](/guide/advanced/render-to-texture/).

**Постпроцессинг (post-processing)** — обработка итогового изображения после рендера сцены: оттенки серого, инверсия,
размытие, bloom и т.д.
См. [Render-to-texture](/guide/advanced/render-to-texture/).

**Полноэкранный квад (fullscreen quad)** — прямоугольник, закрывающий весь экран. Используется для постпроцессинга —
фрагментный шейдер сэмплирует offscreen-текстуру и применяет эффект.
См. [Render-to-texture](/guide/advanced/render-to-texture/).

**Bloom** — эффект светящихся ореолов вокруг ярких объектов; реализуется через выделение ярких
областей, Gaussian blur и аддитивное наложение.
См. [Bloom](/guide/advanced/bloom/).

**Kernel (ядро)** — матрица весов, применяемая при свёртке изображения (например, для размытия или выделения краёв).

**HDR (High Dynamic Range)** — рендеринг в формат с плавающей запятой (`Rgba16Float`), допускающий
значения яркости > 1.0. Позволяет сохранить детали в ярких участках сцены.
См. [HDR и Tone Mapping](/guide/advanced/hdr/).

**LDR (Low Dynamic Range)** — стандартный 8-битный формат (0–1). Мониторы отображают LDR.

**Tone mapping** — преобразование HDR-изображения в LDR для вывода на экран; сжимает диапазон яркостей,
сохраняя детали. Популярные кривые: Reinhard, ACES.
См. [HDR и Tone Mapping](/guide/advanced/hdr/).

**Rgba16Float** — формат текстуры с 16-битным float на канал; хранит значения до ±65504.
Используется для HDR-рендеринга.
См. [HDR и Tone Mapping](/guide/advanced/hdr/).

## Compute

**Compute shader** — программа на WGSL, выполняемая на GPU вне графического конвейера; может читать и писать
в storage-буферы и storage-текстуры. Используется для симуляций, обработки данных, частиц.
См. [Compute Passes](/guide/advanced/compute/).

**Compute pass** — аналогично render pass, но для compute-шейдеров; записывается в `CommandEncoder`.
См. [Compute Passes](/guide/advanced/compute/).

**Workgroup** — группа потоков compute-шейдера, выполняющихся совместно и имеющих доступ к общей памяти
(`var<workgroup>`).
См. [Compute Passes](/guide/advanced/compute/).

**Dispatch** — команда запуска compute-шейдера с заданным количеством workgroup: `compute_pass.dispatch_workgroups(x, y, z)`.
См. [Compute Passes](/guide/advanced/compute/).

**Storage texture** — текстура, доступная compute-шейдеру для записи (`texture_storage_2d<format, write>`)
или чтения-записи (`read_write`). Формат должен совпадать с форматом текстуры.
См. [Compute Passes](/guide/advanced/compute/).

## PBR (Physically Based Rendering)

**PBR** — модель рендеринга, основанная на физических свойствах материалов; обеспечивает реалистичное
освещение при разных условиях.

**Metallic** — параметр PBR: 0 = диэлектрик (пластик, дерево), 1 = металл. Определяет, как поверхность
отражает свет.

**Roughness (шероховатость)** — параметр PBR: 0 = идеально гладкая (зеркало), 1 = полностью матовая.

**IBL (Image-Based Lighting)** — метод освещения, использующий окружение (environment map) как источник света;
обеспечивает реалистичные отражения.

**Environment map (карта окружения)** — текстура, представляющая всё окружение вокруг объекта;
обычно в формате cubemap или equirectangular.
