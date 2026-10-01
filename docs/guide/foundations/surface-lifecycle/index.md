---
title: Жизненный цикл поверхности
description: Размер окна, приостановка, владение кадром и отложенное восстановление surface в wgpu 30.
prev:
  text: "Первый кадр"
  link: ../first-frame/
next:
  text: "От трёх точек к треугольнику"
  link: ../first-triangle/
---

<script setup>
import SurfaceLifecycleDiagram from './SurfaceLifecycleDiagram.vue'
</script>


# Жизненный цикл поверхности

::: info Только native
Продолжаем главу [«Первый кадр»](../first-frame/) на тех же wgpu 30 и событийном цикле winit.
Новый результат не должен выглядеть сложнее: это всё то же серое окно.
Теперь оно допускает изменение размера и умеет откладывать некоторые повторные попытки получения кадра.
:::

В «Первом кадре» мы проследили один путь: получить изображение окна, очистить его, отправить команды и передать кадр на показ.
Но окно не обязано оставаться пригодным для рисования между двумя событиями.
Пользователь может изменить размер, свернуть его, а платформа может приостановить приложение или потерять поверхность вывода.
**Когда следующий кадр допустим и что именно нужно восстановить перед ним?**

**Сохраняем:** линейный цвет `(0.5, 0.5, 0.5, 1.0)`, sRGB-формат, clear/store и порядок submit/present.
Шейдеров и геометрии по-прежнему нет; никакой новый цвет не маскирует ошибки жизненного цикла.
**Меняем:** политику событий, размер окна и реакцию на результаты получения surface texture.
Начальный запрошенный размер остаётся физическим `800 × 600`, но запрет изменения размера из «Первого кадра» убран.

В «Первом кадре» политика была намеренно сокращена: `Timeout`, `Outdated` и `Lost` завершали пример с диагностикой.
`Suboptimal` позволял показать полученный кадр, не организуя последующую переконфигурацию.
Здесь сам серый кадр сохранён, а решения о его допустимости вынесены перед рисованием.

## Запускаем снимок

Из корня workspace:

```sh
cargo run -p surface-lifecycle
```

Исходник целиком: [`code/guide/foundations/surface-lifecycle/src/main.rs` на GitHub](https://github.com/Bromles/wgpu-tutorial/blob/master/code/guide/foundations/surface-lifecycle/src/main.rs).
Ниже — короткие фрагменты этого файла и отличия от снимка «Первого кадра».
Фрагменты с пометками `[!code ++]` и `[!code --]` показывают добавленные и удалённые строки относительно той версии; удалённые строки живут только в статье, в рабочем исходнике их уже нет.
Импорты и связующий код опущены; фрагменты не предназначены для копирования целиком в отдельную программу.

Окно называется `wgpu | Surface lifecycle`, закрывается крестиком или клавишей Escape.
Менять его размер можно обычным перетаскиванием границы.

Ожидаемый результат: равномерный прежний серый по всей клиентской области, без треугольника и анимации.
Линейное значение `0.5` при sRGB-кодировании соответствует примерно `188` в восьмибитном RGB.
После любых переходов жизненного цикла серый должен возвращаться без сдвига яркости.
Оценка глазами не доказывает точность байтов: на показ влияют система вывода и дисплей.

![Эталонный кадр проверки: при любом легальном переходе жизненного цикла клиентская область снова становится однородным серым 188. Эталон снят offscreen-рендером 768×576 того же clear.](/results/surface-lifecycle.png)

## Состояние не сводится к размеру

После suspend окно ещё есть, но рисовать нельзя; после `Lost` контекст есть, а поверхности нет.
Это независимые факты, и каждый из них может запретить кадр — выпишем их по отдельности:

| Поле `App` | Что оно означает | Почему отдельное |
| --- | --- | --- |
| `active` | Приложение находится между resume и suspend | Ненулевой размер не отменяет приостановку |
| `occluded` | Последнее оконное событие сообщило о скрытии | Скрытое окно может сохранять старый размер |
| `size` | Последний известный физический размер клиентской области | Размер в `config` может ещё быть прежним |
| `context` | Есть ли набор surface/device/queue/config | После потери или suspend его может не быть |
| `needs_configure` | Нужно заново применить конфигурацию | Даже одинаковые размеры не гарантируют её актуальность |
| `retry_at` | Есть ли срок отложенной попытки | Ошибка не должна запускать немедленный бесконечный цикл |

Структура `App` выросла из трёх полей «Первого кадра» (`window`, `context`, `failure`):

```rust
#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    context: Option<Context>,
    active: bool, // [!code ++]
    occluded: bool, // [!code ++]
    size: PhysicalSize<u32>, // [!code ++]
    retry_at: Option<Instant>, // [!code ++]
    failure: Option<String>,
}
```

`window` по-прежнему хранится отдельно от `context`: потеря GPU-контекста не требует создавать второе окно.
`failure` служит для передачи фатальной ошибки из обработчика событий к завершению `main`.
Состояния могут сочетаться: например, приложение активно, но скрыто; контекст существует, но его конфигурация устарела.
Поэтому один флаг `ready` не объяснил бы, что именно нужно сделать дальше.

`Context` внутри изменился минимально — добавлен признак устаревания конфигурации:

```rust
struct Context {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    needs_configure: bool, // [!code ++]
}
```

Контекст здесь маленький: surface, device, queue, конфигурация и её признак устаревания.
Он не содержит пользовательских текстур, буферов сцены или pipeline, которые пришлось бы восстанавливать отдельно.
На этом держится выбранная ниже политика `Lost`.

Инициализация `Context::new` совпадает с «Первым кадром» вплоть до диагностических меток: те же instance, surface, adapter, device, capabilities, выбор sRGB-формата, alpha mode и FIFO.
Единственное содержательное отличие — начальное значение флага в возвращаемой структуре:

```rust
Ok(Self {
    surface,
    device,
    queue,
    config,
    needs_configure: true, // [!code ++]
})
```

Поверхность ещё не настроена ни разу, поэтому первая допустимая попытка кадра обязана вызвать configure даже при совпадении размеров с заполнителями.

## Карта решений

<SurfaceLifecycleDiagram />

Схема показывает состояния и условия переходов, а не длительности исполнения.
Серые блоки — ожидание без таймера, жёлтые — запланированная повторная попытка.
Синий путь ведёт к получению кадра, зелёный — к успешной передаче на показ, красный — к завершению.
Текстовые подписи дублируют смысл цвета.
Ветки `Outdated` и `Lost` намеренно не объединены в одно «пересоздать всё».

## Resume и suspend

`resumed` разрешает работу приложения, но не рисует прямо в обработчике.
Если приложение уже активно, повторное событие ничего не делает.
Иначе окно создаётся только при `window.is_none()`; существующее окно сохраняется вместе со своим `WindowId`, а ошибка создания завершает приложение.
После этого сбрасываются старые сведения о скрытии и повторной попытке, читается текущий размер и запрашивается redraw:

```rust
self.active = true;
self.occluded = false;
self.retry_at = None;
let window = self.window.as_ref().unwrap();
self.size = window.inner_size();
window.request_redraw();
```

Контекст создаётся лениво в первом **допустимом** redraw.
Если после resume окно имеет нулевой размер, нет причины уже сейчас создавать и конфигурировать вывод для фиктивного изображения.
`request_redraw()` только просит доставить событие перерисовки; это не непосредственный вызов render.

При suspend окно уничтожать не нужно:

```rust
fn suspended(&mut self, event_loop: &ActiveEventLoop) {
    self.active = false;
    self.context = None;
    self.retry_at = None;
    event_loop.set_control_flow(ControlFlow::Wait);
}
```

Мы устанавливаем `active = false`, отпускаем весь `Context`, отменяем retry и выбираем `ControlFlow::Wait`.
Окно остаётся в `App`, но больше не даёт разрешения рисовать.
Повторный suspend безопасен: присвоить `None` уже пустому полю и повторно запретить рисование допустимо.
Это и есть **идемпотентность**: повторное уведомление не создаёт второй комплект ресурсов и не ломает состояние.

Минимизация окна и приостановка приложения — разные события.
На настольной ОС сворачивание может дать resize или occlusion, но не обязано вызвать `suspended`.
Проверка минимизации поэтому не доказывает, что ветка suspend была исполнена.

## Размер: ноль означает пропуск

Физический размер измеряется в пикселях клиентской области, логический связан с масштабированием интерфейса.
Здесь храним `PhysicalSize<u32>` и перед redraw снова читаем `window.inner_size()`.
Так конфигурация ориентируется на фактический размер окна: события могут приходить с опозданием.

Если хотя бы одно измерение равно нулю, кадр пропускается до создания контекста и получения texture.
Не заменяем ноль единицей через `max(1)` и не подставляем прежние `800 × 600`.
Такое «исправление» изобрело бы область вывода, которой окно сейчас не предоставляет.
Нулевой размер — состояние ожидания, а не ошибка пользователя.

`Context::resize`, как и в «Первом кадре», дополнительно защищается от нулевых размеров и проверяет `max_texture_dimension_2d` запрошенного устройства; слишком большой размер завершает попытку ошибкой с диагностикой.
Изменилось условие применения конфигурации:

```rust
if self.needs_configure // [!code ++]
    || (self.config.width, self.config.height) != (size.width, size.height)
{
    self.config.width = size.width;
    self.config.height = size.height;
    self.surface.configure(&self.device, &self.config);
    self.needs_configure = false; // [!code ++]
    tracing::info!(
        width = size.width,
        height = size.height,
        "Configured surface"
    );
}
```

Нули в начальном `SurfaceConfiguration` — заполнители до первой настройки.
До `surface.configure` код доходит только с допустимыми ненулевыми значениями.
Конфигурация применяется, если размер отличается **или** выставлен `needs_configure`: второе условие нужно, например, после `Outdated` при неизменных ширине и высоте.
После configure флаг снимается и появляется лог `Configured surface` с физическими размерами.
Несколько событий resize могут схлопнуться в один redraw: конфигурируем один раз по последнему размеру.

## Какие события принимаем

Перед обработкой `WindowEvent` проверяем наличие своего окна и совпадение `WindowId`.
Даже в однооконном примере событие без такой проверки не должно менять произвольное состояние приложения.
Это проверка маршрутизации; о пригодности surface она не говорит.

В «Первом кадре» обе ветки сводились к немедленному запросу redraw.
Теперь `Resized` сохраняет размер, отменяет старый retry и помечает существующую конфигурацию устаревшей, а `Occluded` запоминает состояние скрытия:

```rust
WindowEvent::Resized(_) | WindowEvent::Occluded(false) => { // [!code --]
    window.request_redraw(); // [!code --]
} // [!code --]

WindowEvent::Resized(size) => { // [!code ++]
    self.size = size; // [!code ++]
    self.retry_at = None; // [!code ++]
    if let Some(context) = &mut self.context { // [!code ++]
        context.needs_configure = true; // [!code ++]
    } // [!code ++]
    if self.active && !self.occluded && size.width != 0 && size.height != 0 { // [!code ++]
        window.request_redraw(); // [!code ++]
    } // [!code ++]
} // [!code ++]

WindowEvent::Occluded(occluded) => { // [!code ++]
    self.occluded = occluded; // [!code ++]
    self.retry_at = None; // [!code ++]
    if self.active && !occluded { // [!code ++]
        window.request_redraw(); // [!code ++]
    } // [!code ++]
} // [!code ++]
```

Redraw запрашивается только для активного, нескрытого окна с ненулевыми измерениями.
Событие `Occluded(true)` запрещает рисование через флаг; `Occluded(false)` разрешает снова попросить redraw.
Даже если этот запрос сделан при нулевом размере, проверка внутри redraw не допустит configure или acquire.

Ветка `RedrawRequested` тоже перестала быть тонкой обёрткой над `Context`:

```rust
WindowEvent::RedrawRequested => {
    let Some(context) = &mut self.context else { // [!code --]
        return; // [!code --]
    }; // [!code --]
    let size = window.inner_size(); // [!code --]
    if size.width == 0 || size.height == 0 { // [!code --]
        return; // [!code --]
    } // [!code --]
    // No surface texture exists while resize configures the surface. // [!code --]
    if let Err(error) = context.resize(size).and_then(|()| context.render(window)) { // [!code --]
        tracing::error!(%error, "Rendering stopped"); // [!code --]
        self.failure = Some(error.to_string()); // [!code --]
        event_loop.exit(); // [!code --]
    } // [!code --]
    if !self.active { // [!code ++]
        return; // [!code ++]
    } // [!code ++]
    self.size = window.inner_size(); // [!code ++]
    if let Err(error) = self.redraw() { // [!code ++]
        tracing::error!(%error, "Rendering stopped"); // [!code ++]
        self.failure = Some(error.to_string()); // [!code ++]
        self.retry_at = None; // [!code ++]
        event_loop.exit(); // [!code ++]
    } // [!code ++]
}
```

Неактивное приложение возвращается сразу.
Для активного окна обновляется физический размер, после чего вся попытка кадра вынесена в `App::redraw`.
Фатальная ошибка сохраняется в `failure`, отменяет retry и вызывает `event_loop.exit()`.
Закрытие окна и Escape остаются обычным выходом; эти ветки не изменились и приведены в полном исходнике.

## Одна попытка получить кадр

У redraw строгий порядок: сначала проверить, разрешён ли кадр (активность, скрытость, ненулевой размер), затем срок retry, подготовить контекст, применить конфигурацию, получить изображение.
До первых двух проверок нельзя ни конфигурировать surface, ни получать texture «на всякий случай»:

```rust
fn redraw(&mut self) -> Result<(), Box<dyn Error>> {
    if !self.active || self.occluded || self.size.width == 0 || self.size.height == 0 {
        self.retry_at = None;
        return Ok(());
    }
    if self
        .retry_at
        .is_some_and(|deadline| Instant::now() < deadline)
    {
        return Ok(());
    }
    self.retry_at = None;
    let window = self.window.as_ref().unwrap();
    if self.context.is_none() {
        self.context = Some(Context::new(window.clone())?);
    }
    let context = self.context.as_mut().unwrap();
    context.resize(self.size)?;
```

Недопустимое состояние отменяет retry: скрытому или приостановленному окну не нужен периодический опрос.
Если срок ещё не наступил, ранний redraw возвращается, сохраняя этот срок.
Контекст создаётся только здесь — в первом допустимом redraw, в том числе после suspend или `Lost`.
Ошибка создания фатальна и выходит через `?` наружу.

Далее — получение изображения. В wgpu 30 `get_current_texture()` возвращает перечисление `CurrentSurfaceTexture`; старый `Result` с `SurfaceError` остался в прежних версиях API.
В снимке обработаны все семь вариантов:

| Результат acquire | Есть frame? | Действие |
| --- | --- | --- |
| `Success` | Да | Clear, submit, present; без собственного нового redraw |
| `Suboptimal` | Да | Present, затем `needs_configure = true`; без таймера и нового redraw |
| `Timeout` | Нет | Сохранить контекст, назначить retry через 100 мс |
| `Outdated` | Нет | Пометить конфигурацию устаревшей, назначить retry через 100 мс |
| `Lost` | Нет | Удалить весь контекст, назначить retry через 100 мс |
| `Occluded` | Нет | Пропустить попытку без таймера и без изменения оконного флага |
| `Validation` | Нет | Вернуть фатальную ошибку с указанием проверить GPU-диагностику |

```rust
match context.surface.get_current_texture() {
    wgpu::CurrentSurfaceTexture::Success(frame) => context.render(window, frame),
    wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
        context.render(window, frame);
        // Present consumes the frame before any later configure.
        context.needs_configure = true;
    }
    wgpu::CurrentSurfaceTexture::Outdated => {
        context.needs_configure = true;
        self.retry_at = Some(Instant::now() + Duration::from_millis(100));
    }
    wgpu::CurrentSurfaceTexture::Lost => {
        // wgpu 30 requires surface recreation. Rebuild the whole small context,
        // including adapter compatibility and capabilities, on the next redraw.
        self.context = None;
        self.retry_at = Some(Instant::now() + Duration::from_millis(100));
    }
    wgpu::CurrentSurfaceTexture::Timeout => {
        self.retry_at = Some(Instant::now() + Duration::from_millis(100));
    }
    wgpu::CurrentSurfaceTexture::Occluded => {
        // Skip without a timer. Do not latch the window-event flag here:
        // platforms without Occluded events can recover on an OS redraw.
    }
    wgpu::CurrentSurfaceTexture::Validation => {
        return Err("Surface acquisition failed validation; see GPU diagnostics".into());
    }
}
```

Вариант `Success` передаёт frame в `render` целиком: дальнейшие шаги те же, что в «Первом кадре».

### Почему Suboptimal сначала показываем

Этот вариант уже содержит пригодный `SurfaceTexture`, хотя условия вывода больше не оптимальны.
Отбрасывать готовый кадр только ради configure не нужно: сначала `render` потребляет его через present.
Лишь после возврата выставляется `needs_configure`.
**Ни `request_redraw`, ни retry здесь не назначаются.**
Переконфигурация произойдёт при следующем допустимом redraw от ОС или оконных событий, если он появится.
Кадр рисуется только по событию: флаг означает необходимость работы, но сам её не планирует.

### Outdated и Lost требуют разного

`Outdated` означает, что текущую конфигурацию вывода нужно обновить.
Оставляем тот же контекст и на следующей разрешённой попытке вызываем configure даже при прежнем размере.
Старый frame в этой ветке не получен, поэтому удерживать его до восстановления невозможно.

`Lost` в API wgpu 30 требует пересоздания самой surface: повторный configure уже не поможет.
Наш пример идёт дальше: `self.context = None` отпускает surface, device, queue и config.
На следующем допустимом redraw `Context::new` снова создаст instance и surface, выберет совместимый adapter и запросит device/queue.
Затем заново прочитает capabilities и выберет поддерживаемые формат, alpha mode и FIFO.
Так мы не переносим настройки и предположения о совместимости на новую surface вслепую.

Это простая политика маленького clear-примера: в большом приложении выбор при потере surface может быть иным.
Если создание нового контекста не удалось, ошибка завершает приложение; бесконечной серии попыток bootstrap нет.

### Два источника occlusion

`WindowEvent::Occluded` обновляет хранимый оконный флаг и тем самым блокирует последующие попытки.
А `CurrentSurfaceTexture::Occluded` сообщает только о результате конкретного acquire.
Мы пропускаем его без retry, но **не** присваиваем `self.occluded = true`.
Иначе на платформе без парного оконного `Occluded(false)` приложение могло бы навсегда заблокировать само себя.
Следующий redraw от ОС сможет попробовать снова, если остальные условия допуска выполнены.

## Retry без активного ожидания

Для `Timeout`, `Outdated` и `Lost` записывается `Instant::now() + Duration::from_millis(100)` — эвристический срок следующей попытки.
Ошибка может повториться, ОС может доставить событие позже, а окно может стать скрытым или нулевым.
Resize, occlusion и suspend отменяют прежний срок, потому что условия попытки изменились.

Ожидание срока встроено в цикл событий:

```rust
fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
    event_loop.set_control_flow(ControlFlow::Wait);
    if !self.active || self.occluded || self.size.width == 0 || self.size.height == 0 {
        self.retry_at = None;
        return;
    }
    if let Some(deadline) = self.retry_at {
        if Instant::now() >= deadline {
            self.retry_at = None;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
    }
}
```

`about_to_wait` вызывается циклом, когда очередь событий на текущей итерации исчерпана.
Сначала выбирается `ControlFlow::Wait`: без работы цикл ждёт событий.
Если окно пригодно и срок находится в будущем, выбирается `ControlFlow::WaitUntil(deadline)`.
Цикл спит до события или срока вместо непрерывного опроса.
Когда срок достигнут, он снимается и запрашивается один redraw; само рисование здесь не выполняется.
Если ОС доставит redraw раньше, проверка `retry_at` в `redraw` не даст преждевременно повторить acquire.

В обработчике не нужны `sleep(100 ms)`, цикл ожидания или `.await` ради этой задержки.
Блокировка потока событий мешала бы принимать resize, закрытие окна и уведомления платформы.
Bootstrap в `Context::new` при этом исполняется на потоке событий через `pollster::block_on` — при первом запуске и восстановлении после `Lost`.
Поток блокирует только bootstrap; задержка повторной попытки выполняется циклом событий.

## Владение frame задаёт границу configure

`SurfaceTexture` принадлежит одной попытке кадра, а не хранится в `App` между событиями.
Поэтому получение вынесено из `Context::render`: функция принимает уже полученный frame по значению и перестала возвращать `Result`, потому что разбор исходов acquire выполняется снаружи:

```rust
fn render(&self, window: &Window) -> Result<(), Box<dyn Error>> { // [!code --]
fn render(&self, window: &Window, frame: wgpu::SurfaceTexture) { // [!code ++]
```

Это позволяет сначала разобрать результат acquire и лишь затем выполнять прежний clear.
Тело `render` не изменилось: view полученного `frame.texture`, encoder, render pass с прежними clear/store, затем

```rust
self.queue.submit([encoder.finish()]);
window.pre_present_notify();
self.queue.present(frame);
```

При выходе из внутреннего блока `_pass` уничтожается: этот неявный `drop` завершает запись прохода до `encoder.finish()`.
В используемом API именно `queue.present(frame)` потребляет frame; после вызова нельзя снова использовать это значение.
Возврат из present не доказывает, что GPU уже закончил работу или монитор показал пиксели.

До повторной configure не должен оставаться живой surface frame предыдущей попытки.
Если бы кадр отменяли, его пришлось бы отпустить (`drop(frame)`), а не хранить ради будущей перерисовки.
В данном коде успешные кадры не отменяются: они потребляются present, а локальные view/encoder не переживают вызов render.
Поэтому `Suboptimal` выше сначала показывал кадр и лишь потом помечал configure.

## Где восстановление заканчивается

`Validation` — ошибка, а не временная нехватка изображения: повторять ту же некорректную операцию по таймеру бессмысленно.
Ошибка из redraw даёт `Rendering stopped`, сохраняется в `failure`, завершает цикл и возвращается из `main`.
Ошибки создания окна, настройки контекста и превышение лимита размера также не переводятся в retry.

`main` не отличается от «Первого кадра»: инициализация журнала, `EventLoop::new`, `run_app` и возврат сохранённой `failure`.
Отдельно в `Context::new` установлен общий обработчик неперехваченных GPU-ошибок `device.on_uncaptured_error`.
Он пишет `Unrecoverable GPU error` и немедленно вызывает `std::process::exit(1)`, минуя обычный возврат через `failure`.
Неперехваченная validation-ошибка GPU тоже фатальна; она не обязана доходить как вариант результата acquire.
Не путайте `CurrentSurfaceTexture::Lost` с потерей устройства (`DeviceLost`).
Потеря device остаётся фатальной: ветка surface `Lost` восстанавливает только поверхность.

## Проверяем переходы

В headless-среде нельзя проверить реальное окно, все платформенные уведомления и все результаты acquire.
Даже успешный обычный запуск не доказывает работу редких `Lost`, `Outdated`, `Timeout` или `Suboptimal`.

1. Запустите пример. Ожидайте `Selected GPU`, затем `Configured surface` и `Rendered clear` с фактическими размерами. Начальный запрос равен `800 × 600`, но окончательные размеры определяет ОС.
2. Измените клиентскую область до физических `641 × 359`. Ожидайте configure и render с `width=641`, `height=359`, прежний серый во всей области. Размер внешней рамки окна не равен клиентскому.
3. Сверните и восстановите окно, затем верните `800 × 600`. При реально полученном нулевом измерении configure и acquire должны пропускаться; после восстановления ожидается новый серый кадр с ненулевыми размерами.
4. Оставьте окно неподвижным. Код не запрашивает кадр после каждого успешного present; постоянного потока `Rendered clear` от собственного цикла быть не должно. Дополнительные redraw от ОС допустимы.
5. Проследите владение в `render`: где уничтожается pass, где выполняется `pre_present_notify`, где потребляется frame? Объясните, почему configure можно сделать до acquire, но нельзя вставить между acquire и present живого кадра.

::: details Контрольные наблюдения

1. Журнал: `Selected GPU` → `Configured surface` → `Rendered clear`; размеры определяет ОС, а не запрос.
2. После resize до `641 × 359`: тот же серый цвет, новая пара `width=641, height=359` в журнале configure и render.
3. При сворачивании кадры пропускаются (нет `Rendered clear`); после восстановления — новый кадр, если размер стал ненулевым.
4. Окно неподвижно → нет периодических `Rendered clear` от собственного цикла (только от событий ОС).
5. Pass уничтожается при выходе из блока `{}` в `render`; `pre_present_notify` — после submit и до present; frame потребляется `queue.present(frame)`. Configure до acquire допустим — кадра ещё нет; между acquire и present живой frame уже существует, и configure «под ним» сломала бы контракт владения.

:::

Для перехода «ноль → восстановление» важно проверить именно условие, а не угадать его по виду свёрнутого окна.
Если ОС сохраняет ненулевой размер и сообщает occlusion, проверена другая ветка пропуска.
В коде нет отдельных логов на каждый пропуск и каждый результат acquire; отсутствие строк не доказывает, какой вариант произошёл.
При восстановлении до прежнего размера новая configure зависит от полученных событий и флага, а не только от сравнения чисел.

Пиксели после resize не должны менять цвет: меняется число пикселей, не формула clear.
Лог `Rendered clear` подтверждает прохождение CPU-пути до present, но не измеряет показ или завершение GPU.

Теперь первый clear сохраняет свой смысл даже при изменении условий вывода: сначала проверяем право на кадр, затем готовим surface и только потом владеем её изображением.
В главе [«От трёх точек к треугольнику»](../first-triangle/) на этом сером фоне появится первая геометрия.
