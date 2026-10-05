---
title: От трёх точек к треугольнику
description: WGSL, entry points и builtins, сборка треугольника, покрытие выборок и первый render pipeline.
prev:
  text: "Жизненный цикл поверхности"
  link: ../surface-lifecycle/
next:
  text: "Граница оконного фреймворка"
  link: ../framework-triangle/
---

<script setup>
import RasterDiagram from './RasterDiagram.vue'
</script>


# От трёх точек к треугольнику

::: info Только native
Продолжаем главу [«Жизненный цикл поверхности»](../surface-lifecycle/): wgpu 30, winit, pollster.
Оконная обвязка, серый фон и вся политика восстановления поверхности переносятся без изменений.
:::

Очистка заполняла всё изображение одним цветом.
Как нарисовать на этом фоне фигуру — и какие пиксели она займёт?
В этой главе появляются **шейдер** — программа для GPU — и **pipeline** — набор настроек её исполнения.

**Эксперимент:** прежнее окно `800 × 600`, тот же серый фон, но поверх него — сплошной белый треугольник с вершинами `(-0.75, -0.75)`, `(0.75, -0.75)` и `(0, 0.75)`.
Позиции заданы прямо в шейдере, без буферов; изучаемая переменная — одна позиция одной вершины.

**Сохраняем:** sRGB-формат поверхности, clear/store, порядок acquire → submit → present и всю обработку событий из предыдущей главы.
**Меняем:** содержимое кадра — после clear записывается один треугольник; цвет фона и треугольника фиксированы.

## Запускаем снимок

Из корня workspace:

```sh
cargo run -p first-triangle
```

Ожидаемый результат — серый фон из главы [«Первый кадр»](../first-frame/) и белый треугольник вершиной вверх:

![Эталонный кадр снимка first-triangle: белый треугольник вершиной вверх на сером фоне.](/results/first-triangle.png)

Этот PNG — не снимок экрана: его сгенерировал и сверил с эталоном крейт `verify` (см. раздел [«Как проверяется кадр»](#как-проверяется-кадр)).
Окно можно растягивать и сворачивать: lifecycle из [«Жизненного цикла поверхности»](../surface-lifecycle/) работает здесь целиком.

Исходники снимка — четыре файла:

| Файл | Ответственность |
| --- | --- |
| `src/shader.wgsl` | WGSL-программа: позиции вершин и цвет фрагментов |
| `src/sample.rs` | Логика кадра: создание pipeline и запись draw |
| `src/lib.rs` | Открыть `Sample` наружу крейта для верификации |
| `src/main.rs` | Окно, surface lifecycle, acquire/present — перенос из [«Жизненного цикла поверхности»](../surface-lifecycle/) |

Ниже — фрагменты этих файлов; связующий код опущен. Изменения оконной части показаны диффами относительно снимка «Жизненный цикл поверхности»: строки `[!code --]` жили там и удалены, `[!code ++]` добавлены сейчас.

## Куда смотрит шейдер: clip-пространство

Чтобы GPU решил, какие пиксели займёт фигура, позициям нужен общий язык — одна система координат.
Вершина несёт её в четырёх числах — компонентах `vec4`.
Первые три задают позицию, четвёртая называется `w` и пока фиксируется равной `1`.
Шейдер возвращает позицию в **clip-пространстве**, и GPU принимает вершину только если она удовлетворяет условиям:

```text
-w ≤ x ≤ w    -w ≤ y ≤ w    0 ≤ z ≤ w
```

При `w = 1` это знакомый куб от `-1` до `1` по x и y и от `0` до `1` по z; наш `z = 0.5` — середина.
Перспективы здесь нет: `w` — константа, а не расстояние до камеры.
Общий случай деления на `w` и нелинейной глубины появится в главе о проекции; пока достаточно частного случая.

Выводимое изображение — тоже прямоугольник с координатами, но экранный `y` растёт вниз.
Поэтому вершина с `y = 0.75` окажется сверху, а `y = -0.75` — снизу.

<RasterDiagram />

## WGSL: первая программа

**WGSL** — язык шейдеров WebGPU. Это отдельный язык со своей типизацией, не Rust; начнём с программы целиком.
Файл `src/shader.wgsl`:

```wgsl
// Clip-space positions of the three vertices. With w = 1 the clip conditions
// -w <= x,y <= w and 0 <= z <= w reduce to the familiar [-1, 1] cube, and
// z = 0.5 keeps the triangle inside the depth range. There is no perspective
// here: the fourth component is a constant, not a distance.
const POSITIONS: array<vec4<f32>, 3> = array(
    vec4<f32>(-0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.75, -0.75, 0.5, 1.0),
    vec4<f32>(0.0, 0.75, 0.5, 1.0),
);

// One linear color for the whole triangle. The sRGB surface encodes it on write.
const COLOR: vec4<f32> = vec4<f32>(1.0, 1.0, 1.0, 1.0);

// The vertex stage runs once per vertex. vertex_index is provided by the draw
// call and selects which of the three positions to return. The returned value
// is a clip-space position consumed by primitive assembly and rasterization.
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    return POSITIONS[vertex_index];
}

// The fragment stage runs for each covered sample of the assembled triangle
// and returns the color this fragment proposes for the attachment.
@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return COLOR;
}
```

Разберём её построчно, сверху вниз.

### Значения на уровне модуля

`const` объявляет именованную константу **module scope** — она видна всем функциям файла, в отличие от локальных переменных.
`array<vec4<f32>, 3>` — тип «массив из трёх элементов `vec4<f32>`»; `f32` — 32-битное число с плавающей точкой.
Конструктор `vec4<f32>(-0.75, -0.75, 0.5, 1.0)` собирает вектор из четырёх компонент в указанном порядке `x, y, z, w`; у числовых литералов тип выводится из конструктора.
К массиву обращаются по индексу: `POSITIONS[vertex_index]`.

Цвет тоже `vec4<f32>`: три линейных RGB-компоненты и alpha, все от 0 до 1.
Здесь это белый `(1, 1, 1)` и непрозрачность `1`; sRGB-кодирование выполнит цель вывода, как в [«Первом кадре»](../first-frame/).

### Функции и точка входа

`fn` объявляет функцию; тип возврата стоит после `->`, а `return` возвращает значение и завершает вызов.
Имена `vs_main` и `fs_main` — наш выбор; роль каждой функции задаёт её атрибут.

`@vertex` и `@fragment` превращают функции в **entry points** — точки входа, которые вызывает GPU.
Их код исполняется параллельно множеством вычислителей, но программист пишет одиночный вызов: для одной вершины, для одного фрагмента.

`@builtin(vertex_index)` — **built-in**: значение, которое GPU подставляет сам.
Номер вершины `0`, `1` или `2` приходит параметром `u32` — беззнаковым целым.
Атрибут `@builtin(position)` у типа возврата `vs_main` означает: возвращённый `vec4` трактуется как clip-позиция.
У `fs_main` параметров нет — ей пока нечего получать снаружи, а `@location(0)` говорит: результат записывается в первый цветовой выход pipeline.

Вся «геометрия» главы — три возврата констант по индексу.
Никакого чтения буферов: pipeline layout пуст, bindings отсутствуют.

## Сборка треугольника и покрытие

Рисование — это команда внутри render pass.
В `Sample::draw` после clear выполняется:

```rust
pass.set_pipeline(&self.pipeline);
pass.draw(0..3, 0..1);
```

`draw(0..3, 0..1)` просит три вершины с индексами `0, 1, 2` и ноль экземпляров сверх первого.
Для каждого индекса GPU вызывает `vs_main`, подставляя `vertex_index`.
Три clip-позиции образуют примитив: в `TriangleList` каждые три вершины — один треугольник, без соединения с соседними.

Дальше фиксируется покрытие: GPU определяет, какие **выборки** изображения лежат внутри треугольника.
**Выборка** — точка, по которой решается покрытие; в наших кадрах она одна на пиксель, поэтому пока «выборка» значит «пиксель».
Для каждой покрытой выборки запускается `fs_main`, и его результат становится **фрагментом** — кандидатом на вклад в пиксель.
Кандидат — потому что до записи могут вмешаться будущие механизмы: тест глубины, отбрасывание, смешивание.
В этой главе все они выключены, и фрагмент напрямую пишет свой цвет в attachment.

## Pipeline

Шейдер нельзя «позвать» напрямую: его исполняет **render pipeline** — неизменяемый объект с полной конфигурацией рисования.
Создаётся он один раз, при инициализации снимка, в `Sample::new`.

### Модуль шейдера

```rust
let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));
```

Макрос `include_wgsl!` встраивает содержимое файла в бинарник при компиляции: отдельный WGSL-файл лежит рядом с `sample.rs`, но приложению не нужно читать его с диска во время выполнения.

### Дескриптор

```rust
let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
    label: Some("First triangle pipeline"),
    // No bind group layouts: the shader reads no bound resources.
    layout: None,
    vertex: VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: PipelineCompilationOptions::default(),
    },
    fragment: Some(FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(ColorTargetState {
            format,
            // No blending: the fragment value replaces the attachment.
            blend: None,
            write_mask: ColorWrites::ALL,
        })],
        compilation_options: PipelineCompilationOptions::default(),
    }),
    primitive: PrimitiveState {
        topology: PrimitiveTopology::TriangleList,
        front_face: FrontFace::Ccw,
        // No culling: both winding orders stay visible.
        cull_mode: None,
        ..PrimitiveState::default()
    },
    // No depth/stencil attachment and no depth testing.
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    cache: None,
    multiview_mask: None,
});
```

Что здесь важно:

- `vertex` и `fragment` указывают один и тот же модуль, но разные entry points.
- `buffers: &[]` — вершинных буферов нет: позиции живут в константах шейдера.
- `targets` описывает цветовую цель: `format` обязан совпадать с форматом attachment, иначе валидация wgpu отклонит проход. `blend: None` явно выключает смешивание, `write_mask: ALL` разрешает запись всех каналов.
- `topology: TriangleList` задаёт способ сборки примитивов, `front_face` — какую ориентацию вершин считать лицевой.
- Поля, которые мы не изучили, отключены явно: `cull_mode: None` — без отсечения невидимых граней, `depth_stencil: None` — без теста глубины, `multisample` — одна выборка на пиксель.
- `layout: None` выводит пустой pipeline layout: у шейдера нет привязанных ресурсов, и в следующих главах этот раздел начнёт расти.

pipeline неизменяем: нельзя «поменять цвет» у готового объекта — можно создать другой.
Изменяемая часть кадра — данные и команды, а не конфигурация исполнения.

## Снимок: что именно изменилось

Оконная часть — это снимок «Жизненный цикл поверхности» с точечными правками.
Перенесено без изменений: `App` и его поля, `App::redraw` со всеми семью исходами acquire, обработчики `resumed`/`suspended`/`window_event`/`about_to_wait`, `Context::new` вплоть до выбора формата, `Context::resize` и `main`.
Изменились заголовок окна (`wgpu | First triangle`) и диагностические метки `First triangle ...`; сообщение журнала теперь `Rendered frame` вместо `Rendered clear` — кадр перестал быть пустым clear.

`Context` получает поле с логикой кадра:

```rust
struct Context {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    sample: Sample, // [!code ++]
    needs_configure: bool,
}
```

В конце `Context::new`, после сборки `config`, создаётся sample — pipeline привязан к выбранному sRGB-формату:

```rust
let sample = Sample::new(&device, format); // [!code ++]
Ok(Self {
    surface,
    device,
    queue,
    config,
    sample, // [!code ++]
    needs_configure: true,
})
```

Запись кадра переезжает из `Context::render` в `Sample::draw`; владение кадром не меняется:

```rust
{
    let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { // [!code --]
        label: Some("Surface lifecycle clear pass"), // [!code --]
        color_attachments: &[Some(wgpu::RenderPassColorAttachment { // [!code --]
            view: &view, // [!code --]
            resolve_target: None, // [!code --]
            ops: wgpu::Operations { // [!code --]
                load: wgpu::LoadOp::Clear(wgpu::Color { // [!code --]
                    r: 0.5, // [!code --]
                    g: 0.5, // [!code --]
                    b: 0.5, // [!code --]
                    a: 1.0, // [!code --]
                }), // [!code --]
                store: wgpu::StoreOp::Store, // [!code --]
            }, // [!code --]
            depth_slice: None, // [!code --]
        })], // [!code --]
        ..Default::default() // [!code --]
    }); // [!code --]
} // [!code --]
self.sample.draw(&mut encoder, &view); // [!code ++]
self.queue.submit([encoder.finish()]);
window.pre_present_notify();
self.queue.present(frame);
```

`Sample::draw` — весь кадр целиком: знакомые clear/store и одна команда draw:

```rust
pub fn draw(&self, encoder: &mut CommandEncoder, view: &TextureView) {
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("First triangle pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color {
                    r: 0.5,
                    g: 0.5,
                    b: 0.5,
                    a: 1.0,
                }),
                store: StoreOp::Store,
            },
            depth_slice: None,
        })],
        ..RenderPassDescriptor::default()
    });
    pass.set_pipeline(&self.pipeline);
    pass.draw(0..3, 0..1);
}
```

`Sample::draw` принимает encoder и view, а не создаёт их сама: кто владеет кадром, тот и отправляет команды.
Именно эта граница позволяет верификации рисовать тот же кадр без окна.

## Как проверяется кадр

Кроме интерактивных проверок ниже, у главы есть автоматическая: крейт `verify` рендерит кадр offscreen — в обычную текстуру `Rgba8UnormSrgb` `768 × 576` (ширина кратна 64 из-за выравнивания строк при копировании), читает байты назад и сравнивает с эталонным PNG, который вы видели выше.

```sh
cargo test -p first-triangle
```

Тест дополнительно проверяет точечные значения: центр кадра — белый `(255, 255, 255)`, угол — код sRGB для линейного `0.5`, то есть около `188`.
Это авторский инструмент, не учебный код: readback будет объяснён только в ветке B1, поэтому он живёт в отдельном крейте `verify`, а не в `src/` снимка; тест главы лежит рядом в `tests/` и в статье не показывается.

## Проверьте модель

1. Поменяйте местами первые две записи в `POSITIONS`. Почему закрашенная область не изменилась при выключенном culling?
2. Уведите все `x` за `1` (например, `1.5`). Что останется в кадре и почему условия clip нарушены для всех вершин?
3. Замените `z = 0.5` на `1.5` только у одной вершины. Предскажите, исчезнет ли треугольник целиком.
4. `draw(0..2, 0..1)` вместо `0..3`: сколько вершин получит GPU и что нарисует `TriangleList` из двух вершин?

::: details Решения и контрольные выводы

1. Порядок вершин задаёт ориентацию (winding), но `cull_mode: None` отключил отсечение по ней; сам набор точек не изменился, и покрытие то же.
2. Все вершины нарушают `-w ≤ x ≤ w`; примитив отсекается целиком, останется серый фон. Точные пиксели у границы не фиксируются между GPU.
3. Исчезнет не целиком: вершина с `z = 1.5` нарушает `0 ≤ z ≤ w`, и треугольник обрезается плоскостью `z = w` — останется меньшая фигура. Проверьте интерактивно.
4. Две вершины не образуют треугольник: `TriangleList` собирает примитивы тройками, кадр останется без треугольника, только серый фон от clear.

:::

Белый треугольник на сером фоне — минимальный кадр с геометрией.
Следующий шаг маршрута — извлечение оконной обвязки в общий фреймворк, чтобы дальше изучать данные и шейдеры, не повторяя lifecycle в каждой главе.
