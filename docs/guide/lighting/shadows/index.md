---
editLink: false
---

# Тени

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/shadows)

**Что уже должно быть понятно:**

- нормали, направленный свет, diffuse texture
- instancing, camera, depth buffer
- render-to-texture (offscreen rendering)

**Что появится в этой главе:**

- shadow map: depth-текстура с точки зрения источника света
- orthographic projection для направленного света
- `textureSampleCompare` — сравнение глубины в шейдере
- depth bias — предотвращение shadow acne
- два render pass: shadow depth, scene (кубы + пол)

**Итог:** три куба на плоскости, отбрасывающие тени

---

Без теней объекты кажутся парящими — нет связи с поверхностью. Shadow mapping — стандартная техника:
рендерим глубину сцены с точки зрения света, затем при отрисовке сцены сравниваем глубину фрагмента
с записанной.

## Принцип

1. **Shadow pass** — рисуем сцену в depth-текстуру, используя view/projection матрицу источника света.
   Цветового вложения нет — нужен только depth.
2. **Scene pass** — при отрисовке каждого фрагмента проецируем его позицию в пространство света.
   Если глубина фрагмента больше записанной в shadow map — фрагмент в тени.

## Матрица источника света

Для направленного света используем ортографическую проекцию — параллельные лучи:

```rust
let light_view = Mat4::look_to_rh(
    Vec3::new(3.0, 5.0, 3.0),
    Vec3::new(-1.0, -1.0, -1.0).normalize(),
    Vec3::Y,
);
let light_proj = Mat4::orthographic_rh(-6.0, 6.0, -6.0, 6.0, 0.1, 20.0);
let light_view_proj = light_proj * light_view;
```

`look_to_rh` задаёт позицию и направление взгляда света. Ортографическая проекция задаёт
прямоугольную область видимости — все объекты в ней попадут в shadow map.

## Shadow map

Depth-текстура фиксированного размера (1024×1024), не зависящего от размера окна:

```rust
let texture = ctx.device.create_texture(&TextureDescriptor {
    size: Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
    format: TextureFormat::Depth32Float,
    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
    ..
});
```

`RENDER_ATTACHMENT` — для записи глубины, `TEXTURE_BINDING` — для чтения в сцене.

## Shadow pipeline

Shadow pipeline рисует только глубину — нет фрагментного шейдера (`fragment: None`):

```rust
let shadow_pipeline = ctx.device.create_render_pipeline(&RenderPipelineDescriptor {
    fragment: None,
    depth_stencil: Some(DepthStencilState {
        format: TextureFormat::Depth32Float,
        depth_write_enabled: Some(true),
        depth_compare: Some(CompareFunction::Less),
        bias: DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
        ..
    }),
    ..
});
```

### Depth bias

Без bias возникает **shadow acne** — полосы на освещённых поверхностях из-за погрешности сравнения глубин.
Bias сдвигает глубину при записи в shadow map, устраняя самозатенение:

```rust
bias: DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
```

`slope_scale` увеличивает bias для поверхностей, расположенных под углом к лучу света.

## Shadow pass

Render pass без цветового вложения — только depth:

```rust
let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
    color_attachments: &[],  // нет цвета
    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
        view: &self.shadow_texture_view,
        depth_ops: Some(Operations { load: LoadOp::Clear(1.0), store: StoreOp::Store }),
        stencil_ops: None,
    }),
    ..
});
```

## Сравнение глубины в шейдере

Сцена использует `textureSampleCompare` — сравнивает глубину фрагмента с shadow map за одну операцию:

```wgsl
let light_coords = input.light_pos;  // уже после perspective divide
let shadow_uv = vec3<f32>(
    light_coords.x * 0.5 + 0.5,
    1.0 - (light_coords.y * 0.5 + 0.5),
    light_coords.z - 0.005
);
```

Координаты из clip space (−1…1) преобразуются в UV (0…1). Y инвертируется: в clip space Y вверх,
в текстурах — вниз. `shadow_uv.z - 0.005` — дополнительный bias для устранения acne.

В vertex shader позиция в пространстве света вычисляется с perspective divide — делением на `w`:

```wgsl
let light_clip = light.light_view_proj * world_pos;
output.light_pos = light_clip.xyz / light_clip.w;
```

Без деления на `w` координаты в clip space не переходят корректно в NDC, и shadow UV получаются
неправильными.

```wgsl
var shadow = 0.0;
if (shadow_uv.x >= 0.0 && shadow_uv.x <= 1.0 &&
    shadow_uv.y >= 0.0 && shadow_uv.y <= 1.0) {
    shadow = textureSampleCompare(shadow_tex, shadow_sampler, shadow_uv.xy, shadow_uv.z);
} else {
    shadow = 1.0;
}
```

Фрагменты вне shadow map считаются освещёнными (`shadow = 1.0`).

Сэмплер для тени использует `compare: Some(CompareFunction::LessEqual)` и `SamplerBindingType::Comparison`.

## Влияние тени на освещение

Итоговая интенсивность учитывает тень:

```wgsl
let intensity = light.ambient + diffuse * shadow * (1.0 - light.ambient);
```

`shadow` ∈ [0, 1]: 1.0 — полностью освещён, 0.0 — в тени. Ambient всегда виден.

## Пол

Плоский квад на y = −0.5 принимает тени от кубов. Использует тот же pipeline, но свой bind group
с отдельной текстурой и свои вершинные данные:

```rust
rpass.set_vertex_buffer(0, self.floor_vertex_buffer.slice(..));
rpass.set_vertex_buffer(1, self.floor_instance_buffer.slice(..));
rpass.set_index_buffer(self.floor_index_buffer.slice(..), IndexFormat::Uint16);
rpass.set_bind_group(1, &self.floor_light_bind_group, &[]);
rpass.draw_indexed(0..6, 0, 0..1);
```

## Что получилось

Три куба на плоской поверхности. Каждый отбрасывает тень на пол. Камера свободно перемещается —
можно посмотреть на тени с разных сторон.

<div class="tip custom-block" style="padding-top: 8px">
<p class="custom-block-title">Попробуем</p>

- Убрать depth bias (`constant: 0, slope_scale: 0.0`) — увидеть shadow acne
- Увеличить SHADOW_MAP_SIZE до 2048 — более чёткие края теней
- Изменить позицию источника — посмотреть, как тени смещаются
- Убрать shadow pass (закомментировать) — кубы без теней, как в прошлых главах

</div>

[Полный код главы](https://github.com/Bromles/wgpu-tutorial/tree/master/code/guide/lighting/shadows)
