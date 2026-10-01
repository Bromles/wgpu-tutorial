---
title: Over и представление alpha
description: "Прозрачность как операция композиции over: straight и premultiplied RGB, явные BlendState, линейное смешивание и одно sRGB-кодирование на выходе."
prev:
  text: "Список и сумма источников"
  link: ../light-list/
next:
  text: "Порядок, depth и cutout"
  link: ../blend-order/
---

# Over и представление alpha

::: info Только native
Снимок [«Список и сумма источников»](../light-list/): камера и стенд прежние, света больше нет — вместо него диагностические прямоугольники; клавиша `M`.
:::

Значение alpha в выводе фрагментного шейдера само по себе ничего не делает: пока у pipeline выключено смешивание, `a = 0.5` рисуется так же непрозрачно, как `a = 1`.
Прозрачность — не цвет, а **операция композиции** над уже записанным цветом: пиксель кадра и пиксель фрагмента объединяются уравнением, а не заменой.

**Эксперимент:** фиксированная ortho-камера, света нет.
Слева пресет A: синий opaque фон-квадрат и красный прямоугольник с `alpha = 0.5` поверх него.
Справа пресет B: чёрный opaque фон и два полупрозрачных прямоугольника — красный и зелёный, оба `0.5`, — перекрывающиеся в зоне контроля.
Фон — тоже квад, а не clear: opaque pipeline рисует первым.
Клавиша `M` переключает представление источника: straight ↔ premultiplied.
Кадр при этом **не должен измениться ни на один пиксель** — меняется только то, кто умножает RGB на alpha: pipeline или шейдер.

## Покрытие и оператор over

Модель простая: фрагмент закрывает долю `as` пикселя, и сквозь оставшуюся долю `1 − as` виден старый цвет.
Композиция **over** (источник поверх приёмника):

```text
C = Cs·as + Cd·(1 − as)        a = as + ad·(1 − as)
```

Вторая строка — тот же оператор для альфа-канала: кадр тоже может быть полупрозрачным, и его накопленная непрозрачность растворяется по тем же весам.
Контрольный пиксель A — красный `(1, 0, 0, 0.5)` поверх синего фона `(0, 0, 1, 1)`:

```text
straight:   RGB = (1,0,0)·0.5 + blue·(1−0.5) = (0.5, 0, 0.5)
alpha:      a   = 0.5 + 1·0.5 = 1        кадр остаётся непрозрачным
```

## Straight и premultiplied RGB

Одно и то же «красный наполовину прозрачный» можно упаковать двумя способами.

**Straight** («чистый» цвет): RGB — цвет непрозрачной поверхности, непрозрачность лежит только в alpha.
Тогда pipeline обязан взвесить RGB сам — множитель `SrcAlpha` в факторе источника.

**Premultiplied** (предумноженный): шейдер отдаёт уже взвешенный цвет, `rgb' = rgb·a`.
Множитель источника становится единицей — `One` — и уравнение вырождается до суммы «сколько добавил источник + что осталось от фона»:

```text
premultiplied:  RGB = (0.5,0,0) + blue·0.5 = (0.5, 0, 0.5)
```

Тот же результат — представление меняет распределение работы, а не ответ.
Зачем тогда второе? При интерполяции и фильтрации соседних значений premultiplied ведёт себя согласованно: частично прозрачные участки несут долю цвета, пропорциональную альфе. Straight-цвет переносит полную яркость через прозрачные участки — на полупрозрачных краях это даёт ореолы. И premultiplied естественно выражает «светящиеся» частицы, у которых `rgb'` больше, чем `rgb·a`.
Оба представления живут в одном шейдере как две точки входа:

```wgsl
// Straight alpha: RGB leaves the shader untouched; the blend factors
// SrcAlpha / OneMinusSrcAlpha weigh it at the output merger.
@fragment
fn fs_straight(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}

// Premultiplied alpha: the shader itself weighs RGB by alpha, so the
// pipeline can use the simpler One / OneMinusSrcAlpha factors.
@fragment
fn fs_premultiplied(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color.rgb * input.color.a, input.color.a);
}
```

## BlendState: уравнения, вписанные в pipeline

Оператор over задаётся парой `BlendComponent` — по одному на цвет и альфа, каждый с факторами и операцией.
Никаких пресетов: каждая строка ниже — член уравнения.

```rust
/// The over operator for straight sources: the pipeline weighs RGB by the
/// source alpha itself. Both components are written out field by field -
/// no REPLACE preset hides an equation.
const STRAIGHT: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
};

/// The same operator for premultiplied sources: RGB arrives from the
/// shader already weighted by alpha, so the source factor is simply One.
/// The alpha component is shared with the straight state.
const PREMULTIPLIED: BlendState = BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::One,
        dst_factor: BlendFactor::OneMinusSrcAlpha,
        operation: BlendOperation::Add,
    },
    alpha: STRAIGHT.alpha,
};
```

Пара «представление × факторы» обязана быть согласована: premultiplied-шейдер со straight-факторами умножил бы RGB на alpha **дважды** — `(0.5,0,0)·0.5 + blue·0.5 = (0.25, 0, 0.5)` вместо `(0.5, 0, 0.5)`.
Поэтому переключение `M` меняет pipeline целиком — точку входа и факторы вместе.
Геометрия при этом одна: три pipeline (opaque, straight, premultiplied) читают одни и те же буферы.

## Линейное смешивание, одно кодирование

Цвета квандов — линейные, как в [главе «Цвет и его кодирование»](../../foundations/srgb-mixing/).
Для sRGB-цели WebGPU обязан смешивать в линейном пространстве: записанный код фона декодируется, складывается, и кодируется **один раз** — при записи результата.

```text
A: (0.5, 0, 0.5) линейных → коды (188, 0, 188)
B: красный затем зелёный   → (0.25, 0.5, 0) → коды (137, 188, 0)
```

В пресете B уже спрятан сюжет следующей страницы: `over` не коммутативен.
Зелёный поверх красного даёт `(0.25, 0.5, 0)`; красный поверх зелёного — `(0.5, 0.25, 0)`.
Порядок — часть данных прозрачной сцены.

## Снимок

Пять квадов в одном вершинном буфере: два фона (A и B), затем три источника; ortho-матрица загружается в uniform один раз — камера статична.

```rust
fn vertices() -> [Vertex; 20] {
    let quads = [
        quad(-HALF_X, -HALF_Y, 0.0, HALF_Y, BLUE),
        quad(0.0, -HALF_Y, HALF_X, HALF_Y, BLACK),
        quad(-1.8, -0.9, -0.6, 0.9, RED),
        quad(0.3, -0.9, 1.5, 0.9, RED),
        quad(0.9, -0.9, 2.1, 0.9, GREEN),
    ];
    let mut vertices = [Vertex { position: [0.0; 4], color: [0.0; 4] }; 20];
    for (quad, chunk) in quads.iter().zip(vertices.as_chunks_mut::<4>().0.iter_mut()) {
        chunk.copy_from_slice(quad);
    }
    vertices
}
```

Порядок вывода — явный: сначала фон opaque-pipeline (смешивания нет вообще), затем источники; внутри одного `draw_indexed` примитивы обрабатываются в порядке индексов, поэтому красный B ляжет до зелёного.

```rust
        // Opaque first: the backgrounds of both presets, alpha = 1, no
        // blending at all.
        pass.draw_indexed(BACKGROUNDS, 0, 0..1);
        // Sources after opaque, in a fixed order (A red; B red, then B
        // green). Only the representation changes between the pipelines.
        let sources = if self.premultiplied {
            &self.premultiplied_pipeline
        } else {
            &self.straight_pipeline
        };
        pass.set_pipeline(sources);
        pass.draw_indexed(SOURCES, 0, 0..1);
```

Проверка прогоняет оба представления и сверяет контрольные пиксели с CPU-цепочкой `over_straight`/`over_premultiplied` из `params.rs`: A — `(188, 0, 188)` и альфа 255, B — `(137, 188, 0)`, плюс чистые зоны синего/красного/зелёного как свидетели входов.

![Кадр blend-over: пресеты A и B наложения полупрозрачных прямоугольников.](/results/blend-over.png)

Пресеты главы: слева A — красный `alpha = 0.5` поверх синего фона; справа B — красный и зелёный на чёрном с зоной перекрытия (контрольные числа — в тексте выше). Клавиша `M` переключает straight/premultiplied — кадр не меняется ни на пиксель.

```sh
cargo run -p blend-over
cargo test -p blend-over
cargo test -p foundations-verify
```

## Проверьте модель

1. Пресет A, premultiplied: шейдер вернул `(0.5, 0, 0, 0.5)`. Какие факторы дадут верный пиксель и какое число окажется в синем канале до кодирования?
2. Что запишется в альфа-канал кадра после наложения красного `a = 0.5` на opaque фон? А после зелёного поверх результата?
3. Пресет B: зелёный нарисовали первым, красный вторым — какой линейный цвет и какие коды у контрольного пикселя?
4. Почему для alpha-канала фактор источника — `One`, а не `SrcAlpha`, даже в straight-pipeline?

::: details Решения и контрольные выводы

1. `One / OneMinusSrcAlpha`: `(0.5,0,0) + blue·0.5`; синий канал до кодирования — `0 + 1·0.5 = 0.5`, после — код 188. Умножать на `SrcAlpha` ещё раз значило бы взвесить RGB дважды.
2. `a = 0.5 + 1·(1−0.5) = 1`; вторым слоем — `0.5 + 1·0.5 = 1` снова: непрозрачный фон «поглощает» непрозрачность источников, кадр остаётся opaque.
3. Сначала зелёный: `(0, 0.5, 0)` поверх чёрного фона. Затем красный: `(0.5, 0, 0) + (0, 0.5, 0)·0.5 = (0.5, 0.25, 0)`, коды `(188, 137, 0)` — против `(137, 188, 0)` при обратном порядке. Коммутативности нет.
4. Alpha источника уже «своя»: весить её собственной непрозрачностью значило бы квадратичное затухание — `as²`. Стандарт over для альфы — те же веса, что и для цвета, но без повторного взвешивания источника: `One / OneMinusSrcAlpha`.

:::

Один источник над фоном — арифметика.
[Следующая страница](../blend-order/) добавляет второй: порядок, depth-тест без записи и cutout.
