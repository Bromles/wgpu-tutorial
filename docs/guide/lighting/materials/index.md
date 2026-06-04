---
editLink: false
---

# Материалы и свет

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/materials)

**Что уже должно быть понятно:**

- нормали, направленный свет, ambient
- instancing, camera, depth buffer
- текстуры, UV-координаты, сэмплеры

**Что появится в этой главе:**

- несколько источников света: массив в uniform-буфере
- diffuse texture вместо плоского цвета
- три источника разного цвета, влияющие на сцену одновременно

**Итог:** сетка 3×3×3 кубов, освещённая тремя источниками разного цвета, с текстурой вместо плоского серого

---

В прошлой главе мы добавили один направленный свет. Но в реальных сценах несколько источников — солнце,
небо, отражения. В этой главе мы добавим три источника с разными цветами и заменим плоский цвет на diffuse texture.

## Несколько источников света

Источники хранятся в массиве фиксированной длины внутри uniform-буфера:

```rust
#[derive(ShaderType, Clone, Copy)]
struct Light {
    direction: Vec3,
    color: Vec3,
}

#[derive(ShaderType)]
struct LightUniforms {
    lights: [Light; 3],
    ambient: f32,
}
```

`Vec3` из glam реализует `encase::ShaderType`, поэтому `Light` тоже можно пометить `#[derive(ShaderType)]`.
Массив `[Light; 3]` — фиксированный размер, WGSL не поддерживает динамические массивы в uniform.

В шейдере:

```wgsl
struct Light {
    direction: vec3<f32>,
    color: vec3<f32>,
};

struct LightUniforms {
    lights: array<Light, 3>,
    ambient: f32,
};
```

Обратите внимание: в WGSL это `array<Light, 3>`, а в Rust — `[Light; 3]`.

## Суммирование вклада источников

Фрагментный шейдер проходит по всем источникам в цикле:

```wgsl
var total = vec3<f32>(0.0);
for (var i = 0u; i < 3u; i++) {
    let light_dir = normalize(-light.lights[i].direction);
    let diffuse = max(dot(normal, light_dir), 0.0);
    total += light.lights[i].color * diffuse * tex_color.rgb;
}
let ambient = light.ambient * tex_color.rgb;
return vec4<f32>(ambient + total, 1.0);
```

Каждый источник даёт свой вклад: тёплый белый сверху-слева, голубой справа, красный снизу-справа.
Ambient — минимальная яркость, не зависящая от источника.

## Diffuse texture

Вместо плоского `vec3<f32>(0.85, 0.85, 0.85)` используем текстуру:

```wgsl
let tex_color = textureSample(diffuse_tex, diffuse_sampler, input.uv);
```

Цвет из текстуры умножается на вклад каждого источника. Это называется **diffuse map** — текстура,
определяющая базовый цвет поверхности.

Для этого вершины теперь хранят UV-координаты:

```rust
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
}
```

Текстура и сэмплер находятся в bind group 1 рядом с light uniform:

```rust
let light_bgl = ctx.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
    entries: &[
        // binding 0: Light uniform (FRAGMENT)
        // binding 1: Diffuse texture (FRAGMENT)
        // binding 2: Diffuse sampler (FRAGMENT)
    ],
});
```

Все три ресурса используются только во фрагментном шейдере, поэтому `visibility: ShaderStages::FRAGMENT`.

## Три источника с разными цветами

```rust
lights: [
    Light { direction: Vec3::new(-0.5, -1.0, -0.3), color: Vec3::new(1.0, 0.95, 0.85) },
    Light { direction: Vec3::new(0.7, -0.3, 0.5), color: Vec3::new(0.3, 0.5, 1.0) },
    Light { direction: Vec3::new(-0.2, -0.5, 0.8), color: Vec3::new(0.9, 0.3, 0.3) },
],
```

Первый — тёплый белый (солнечный), второй — голубой, третий — красный. Разные направления создают
цветовые переходы на гранях куба.

## Что получилось

27 кубов (3×3×3) с шахматной текстурой. Три источника света создают разноцветные блики и тени на гранях.
Камера перемещается как обычно.

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Изменить цвета источников — посмотреть, как смешиваются
- Увеличить массив до `[Light; 5]` (и в WGSL `array<Light, 5>`) — добавить больше источников
- Убрать один источник (color = `Vec3::ZERO`) — увидеть разницу
- Заменить текстуру на однотонную — сравнить с результатом прошлой главы

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/materials)
