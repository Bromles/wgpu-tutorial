<img align="right" width="15%" src="docs/public/favicon.svg" alt="logo">

# Изучение WebGPU на Rust

Практический курс из 43 глав в пяти частях: **как появляется изображение** (пиксели, первый кадр, треугольник), **данные и изображение** (буферы, uniforms, текстуры), **пространство и видимость** (матрицы, камера, проекция), **поверхность и свет** (нормали, материалы, прозрачность) и **как устроен кадр** (compute-проходы, тени, MSAA, HDR). Каждая глава — законченный запускаемый пример с тестами; эталонные кадры проверяются offscreen-верификацией и встраиваются в статьи.

### Как запустить

Примеры глав — пакеты общего Cargo workspace:

```bash
cargo run -p <chapter-package>
# например: cargo run -p first-triangle
```

Тесты workspace, включая offscreen-сверку эталонных кадров:

```bash
cargo test --workspace --locked
```

Перегенерация изображений для статей:

```bash
cargo run -p foundations-verify --bin snapshots
```

Сборка сайта с руководством:

```bash
yarn docs:build
```

### Лицензии

Код примеров в директории `code` выложен под двумя лицензиями, из которых вы можете выбрать любую на свое усмотрение:

* [MIT](./LICENSE-MIT)
* [Apache 2.0](./LICENSE-APACHE)

Само руководство в директории `docs` опубликовано под лицензией [Creative Commons Attribution 4.0 International](./LICENSE-CC-BY-4.0)
