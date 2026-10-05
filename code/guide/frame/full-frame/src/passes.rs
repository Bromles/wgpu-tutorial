//! Recording of the four frame passes.

use bytemuck::bytes_of;
use wgpu::Color;
use wgpu::{
    CommandEncoder, IndexFormat, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, StoreOp, TextureView,
};

use crate::geometry::{CUBE_MESH, FLOOR_MESH, PANEL_INDICES};
use crate::sample::FullFrame;

/// The HDR clear color, linear: tone mapping sees the background too.
const CLEAR: Color = Color {
    r: 0.1,
    g: 0.1,
    b: 0.12,
    a: 1.0,
};

impl FullFrame {
    /// Pass 1: shadow depth; only the cube casts.
    pub(crate) fn shadow_pass(&self, encoder: &mut CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Shadow depth pass"),
            color_attachments: &[],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &self.shadow_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.shadow_pipeline);
        pass.set_bind_group(0, &self.light_bind_group, &[]);
        pass.set_bind_group(2, &self.objects_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        let cube: u32 = 0;
        pass.set_immediates(0, bytes_of(&cube));
        pass.draw_indexed(CUBE_MESH.clone(), 0, 0..1);
    }

    /// Pass 2: opaque HDR; no resolve yet - the panel still blends into samples.
    pub(crate) fn opaque_pass(&self, encoder: &mut CommandEncoder, mode: usize) {
        let (color_view, depth_view) = if self.msaa_enabled {
            (
                self.msaa_color_view
                    .as_ref()
                    .expect("msaa kit exists after recreate"),
                self.msaa_depth_view
                    .as_ref()
                    .expect("msaa kit exists after recreate"),
            )
        } else {
            (
                self.resolve_view
                    .as_ref()
                    .expect("resolve view exists after recreate"),
                self.single_depth_view
                    .as_ref()
                    .expect("single kit exists after recreate"),
            )
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Opaque HDR pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(CLEAR),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Clear(1.0),
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.lit_pipelines[mode]);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(2, &self.objects_bind_group, &[]);
        // Group 3: the shadow map for the lit fragment stage.
        pass.set_bind_group(3, &self.shadow_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        // Mesh = draw range, material = group, object = immediate index.
        for (object, mesh, material) in [(0u32, &CUBE_MESH, 0usize), (1u32, &FLOOR_MESH, 1usize)] {
            pass.set_bind_group(1, &self.material_bind_groups[material], &[]);
            pass.set_immediates(0, bytes_of(&object));
            pass.draw_indexed(mesh.clone(), 0, 0..1);
        }
    }

    /// Pass 3: transparent panel; depth writes off, MSAA resolves here.
    pub(crate) fn panel_pass(&self, encoder: &mut CommandEncoder, mode: usize) {
        let resolve_view = self
            .resolve_view
            .as_ref()
            .expect("resolve view exists after recreate");
        let (color_view, depth_view, resolve_target) = if self.msaa_enabled {
            (
                self.msaa_color_view
                    .as_ref()
                    .expect("msaa kit exists after recreate"),
                self.msaa_depth_view
                    .as_ref()
                    .expect("msaa kit exists after recreate"),
                Some(resolve_view),
            )
        } else {
            (
                resolve_view,
                self.single_depth_view
                    .as_ref()
                    .expect("single depth exists after recreate"),
                None,
            )
        };
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Transparent HDR pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_view,
                resolve_target,
                ops: Operations {
                    load: LoadOp::Load,
                    // Resolve happens at pass end regardless; the MSAA
                    // texture itself is never read again, so in the
                    // multisampled mode its store can be discarded.
                    store: if resolve_target.is_some() {
                        StoreOp::Discard
                    } else {
                        StoreOp::Store
                    },
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.panel_pipelines[mode]);
        pass.set_bind_group(0, &self.panel_bind_group, &[]);
        pass.set_vertex_buffer(0, self.panel_vertex_buffer.slice(..));
        pass.set_index_buffer(self.panel_index_buffer.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(0..PANEL_INDICES.len() as u32, 0, 0..1);
    }

    /// Pass 4: tone map the HDR frame onto the surface.
    pub(crate) fn tone_pass(&self, encoder: &mut CommandEncoder, view: &TextureView) {
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Tone map to surface pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: Operations {
                    // Never visible: the triangle covers every pixel.
                    load: LoadOp::Clear(CLEAR),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..RenderPassDescriptor::default()
        });
        pass.set_pipeline(&self.tone_pipeline);
        pass.set_bind_group(
            0,
            self.tone_bind_group.as_ref().expect("bind group exists"),
            &[],
        );
        pass.draw(0..3, 0..1);
    }
}
