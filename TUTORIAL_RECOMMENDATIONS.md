# Рекомендации по дальнейшей структуре руководства

Этот файл фиксирует идеи по развитию руководства

## Главная проблема

Текущий формат естественно ведет к одной из двух крайностей:

- если каждая глава инкрементально наращивает предыдущий пример, руководство постепенно превращается в написание
  мини-движка;
- если каждая статья разбирает отдельную фичу независимо, читателю становится непонятно, как сшивать эти фичи вместе.

Цель стоит сформулировать иначе: руководство должно наращивать не движок, а ментальную модель современного рендера на
wgpu. Код при этом может использовать общий учебный каркас, но каждая глава должна менять только ту часть, о которой
идет речь.

## Основная модель курса

Лучше разделить материалы на три типа страниц.

### Основной маршрут

Линейный путь, который читатель проходит последовательно. Здесь каждая глава опирается на предыдущие понятия и добавляет
одну-две важные идеи.

### Концептуальные страницы

Страницы, объясняющие отдельные темы независимо от конкретного состояния кода:

- bind groups;
- color spaces;
- depth;
- WGSL layout;
- present modes;
- GPU/CPU synchronization;
- resource lifetime.

Такие страницы можно линковать из нескольких глав.

### Лаборатории

Самостоятельные практические статьи про отдельные техники:

- bloom;
- particles;
- compute blur;
- picking;
- shadow maps;
- render bundles;
- GPU culling.

Лаборатория должна запускаться на готовом учебном каркасе и не требовать от читателя вручную восстанавливать весь
предыдущий путь.

## Возможная структура маршрута

### 1. Bootstrap

- окно;
- `Instance`, `Adapter`, `Device`, `Queue`, `Surface`;
- первый clear;
- первый triangle;
- pipeline, shader stages, render pass.

### 2. GPU Data Model

- vertex buffers;
- index buffers;
- uniforms;
- bind groups;
- texture + sampler;
- layout/alignment;
- storage buffers.

### 3. Space And Camera

- координаты WebGPU;
- model/view/projection;
- depth buffer;
- camera controller;
- instancing.

### 4. Materials And Lighting

- normals;
- diffuse/specular;
- multiple lights;
- gamma, sRGB, linear color;
- normal maps;
- shadows.

### 5. Modern Renderer

- render-to-texture;
- postprocessing;
- HDR;
- tone mapping;
- MSAA;
- deferred rendering;
- compute passes;
- GPU culling;
- particles;
- tiled lighting.

### 6. PBR Track

- metallic-roughness;
- IBL;
- environment maps;
- glTF loading;
- final mini-viewer.

## Шаблон каждой главы

В начале каждой статьи стоит явно фиксировать контекст:

```text
Что уже должно быть понятно:
- render pipeline
- bind group
- uniform buffer

Что появится в этой главе:
- depth texture
- depth compare
- camera projection

Итог:
- сцена с кубами и корректной глубиной
```

Это помогает читателю понимать, где он находится в общей карте курса и зачем нужна конкретная фича.

## Учебный каркас вместо движка

После главы про первый треугольник стоит ввести общий учебный каркас и прямо объяснить его назначение:

> Дальше мы не будем каждый раз переписывать winit/surface boilerplate. Он вынесен в учебный каркас. Это не движок:
> здесь нет ECS, сцен, материалов, загрузчика уровней и архитектуры игры. Это только обвязка, чтобы главы были про
> графику.

Возможная структура:

```text
code/
  framework/
    src/
      app.rs
      gpu.rs
      texture.rs
      camera.rs
      assets.rs
  guide/
    getting-started/
      01-creating-window/
      02-init-wgpu/
      03-hello-triangle/
    gpu-data/
      01-vertex-buffer/
      02-uniforms/
      03-textures/
```

Пример интерфейса для учебных примеров:

```rust
trait Example {
    fn new(ctx: &GpuContext) -> Self;
    fn resize(&mut self, ctx: &GpuContext, size: PhysicalSize<u32>);
    fn update(&mut self, dt: Duration);
    fn render(&mut self, ctx: &GpuContext, frame: &TextureView, encoder: &mut CommandEncoder);
}
```

Так каждая глава показывает только свой пример, а не весь `winit` event loop.

## Как не спрятать wgpu за учебным каркасом

Есть отдельный риск: если все примеры работают через framework, читатель может не понять, как реализовать те же вещи в
своем приложении без этой обвязки. Этот риск стоит закрывать не отказом от каркаса, а явным раскрытием абстракции.

Главный принцип: каркас должен экономить внимание, а не становиться источником знания. Читатель должен регулярно видеть,
какие именно строки wgpu-кода были спрятаны и как их вернуть в свое приложение.

### Каркас не должен полностью скрывать wgpu-типы

`Example::render` должен получать реальные `wgpu`-типы или очень тонкую обертку с публичными полями:

```rust
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_format: wgpu::TextureFormat,
}
```

Читателю должно быть видно, что это не магия, а просто вынесенные `device`, `queue`, `surface`, resize и event loop.

### Нужна отдельная глава про устройство каркаса

После введения framework стоит добавить главу вроде "Что скрывает учебный каркас".

В ней нужно показать соответствие между учебными типами и обычным кодом:

```text
framework::App        -> winit ApplicationHandler
GpuContext::new       -> Instance / Adapter / Device / Queue / Surface
Example::resize       -> surface.configure + depth resize
Example::render       -> command encoder + render pass + queue.submit
```

Рядом должна быть ссылка на полный код каркаса.

### В крупных главах нужен блок "без framework"

Для важных тем стоит добавлять короткий переносной recap. Не полный переписанный пример, а список действий:

```text
Чтобы перенести это в свое приложение без framework:
- создайте texture через device.create_texture(...)
- создайте view
- сохраните ее рядом с surface_config
- пересоздавайте при resize
- передайте view в RenderPassDescriptor.depth_stencil_attachment
```

Такие блоки особенно важны для:

- depth buffer;
- camera uniforms;
- textures;
- render-to-texture;
- postprocessing;
- compute passes.

### Каркас должен быть намеренно скучным

В framework не стоит добавлять:

- ECS;
- asset manager;
- scene graph;
- material system;
- renderer graph;
- dependency injection;
- generic abstraction layers.

Допустимый минимум:

- окно;
- surface lifecycle;
- device/queue;
- resize;
- frame acquire/present;
- timing;
- input, если нужен.

### Нужны периодические standalone-примеры

Периодически стоит добавлять полные примеры без учебной обвязки:

- `hello-triangle-standalone`;
- `texture-standalone`;
- `compute-standalone`.

Они могут быть длиннее, зато показывают, как та же техника выглядит в обычном приложении без framework.

## Формат кода для глав

Для каждой темы полезно держать три артефакта:

```text
article.md              // объяснение
code/.../starter        // состояние до главы
code/.../final          // состояние после главы
code/framework          // общий учебный runtime
```

В статье лучше показывать важные фрагменты или diff, а полный код держать рядом. Это дает два режима чтения:

- линейное прохождение по основному маршруту;
- открытие отдельной лаборатории без восстановления всего предыдущего контекста.

## Что поправить в текущих статьях

### Главная страница

`docs/index.md` сейчас слишком длинная и полемичная для входа. Разделы "Почему WebGPU" и "Почему Rust" занимают много
места до первого результата.

Лучше сократить главную страницу до:

- кому курс;
- что построим;
- что нужно знать;
- как устроены примеры;
- карта курса.

Сравнение Vulkan, Metal, DirectX, OpenGL, Dawn и wgpu лучше вынести в appendix: "Почему выбран wgpu".

Важно: утверждения про поддержку браузеров, Metal, Dawn/wgpu и платформы быстро устаревают. Их стоит либо датировать,
либо держать в отдельной странице, которую проще регулярно обновлять.

### Первый треугольник

Страница `docs/guide/getting-started/hello-triangle/index.md` сейчас ссылается на фрагмент
`#renderer-new-surface-config`, но в коде нет соответствующих region-меток. Сборка VitePress это не ломает, но
содержательно статья расходится с кодом.

Нужно либо добавить region-метки в `code/guide/getting-started/hello-triangle/src/main.rs`, либо переписать вставку
фрагмента.

### Навигация

`docs/.vitepress/config.mts` уже содержит будущие главы, но пока это список заглушек. Лучше сделать явную карту курса:

```text
Основной маршрут
GPU data model
3D basics
Lighting
Renderer techniques
Compute
Reference
```

## Что поправить в текущем коде

### Разделить состояние рендера

В `code/guide/getting-started/hello-triangle/src/main.rs` `Renderer` уже начинает превращаться в мешок состояния:

- `device`;
- `queue`;
- `surface`;
- `surface_config`;
- `surface_format`;
- `pipeline`.

Дальше туда неизбежно попадут depth texture, buffers, bind groups, camera, textures и samplers.

Лучше рано разделить:

- `GpuContext`: `device`, `queue`, `surface_format`, `surface_config`;
- `App`: window/event loop;
- `Example`: конкретная глава;
- `FrameContext`: encoder, target view, frame size.

### Убрать `Option<RenderPipeline>`

`pipeline: Option<RenderPipeline>` в hello triangle не нужен. Pipeline создается синхронно, значит можно собрать
`surface_config`, `shader_module`, `pipeline`, а потом вернуть `Self { pipeline }` без `Option` и `expect` на каждом
кадре.

### Упростить async bootstrap

`tokio` ради `block_on` усложняет первые главы. Для руководства по графике лучше:

- либо использовать маленький `pollster::block_on`;
- либо спрятать async-init слой внутри framework и больше не показывать его читателю.

Сейчас `tokio` добавляет лишнюю когнитивную ветку: читатель пришел за GPU, а получает runtime.

### Поменять режим презентации по умолчанию

`PresentMode::AutoNoVsync` в первом треугольнике спорный дефолт. Для курса лучше `AutoVsync`: меньше лишней нагрузки,
меньше шума от очень высокого FPS, меньше странных первых впечатлений.

`AutoNoVsync` можно показать позже в главе про present modes и frame pacing.

### Ограничить `Arc<Window>` каркасом

`Arc<Window>` можно оставить как учебное упрощение, но лучше спрятать это в framework. В главах про графику читатель не
должен снова и снова видеть `Arc<Window>`, `ApplicationHandler`, `ControlFlow`, resize-флаг и surface errors.

## Главный вывод

Не нужно избегать общего каркаса. Его нужно ввести, назвать и ограничить правильно.

Это не "пишем движок", а "убираем winit/wgpu boilerplate за сцену, чтобы изучать рендер".

Так курс сможет быть похож на LearnOpenGL: последовательный и практичный, но без бесконечного расширения одного
архитектурного монолита.
