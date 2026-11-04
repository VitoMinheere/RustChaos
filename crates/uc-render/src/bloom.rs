//! Bloom (#523): what lights itself glows. The glowing parts of the frame (lit windows, the
//! pages that light themselves, lamps, headlights, muzzle flashes and fire) are drawn again
//! on their own, against the frame's depth, then brought down to a quarter of the size, the
//! too-dim taken off, blurred and added back over the frame, under the HUD. Not in the
//! original. Only what lights itself, not whatever's bright, so a white wall in the sun
//! doesn't glow.

use crate::{DEPTH, SAMPLES};
use uc_world::Vertex;

/// The glow's own textures: room for the glows that add up past 1.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
/// How much smaller than the frame the blur is.
const DOWN: u32 = 4;
/// What's taken off the glow before blurring: dim self-lit pages, far off in the fog, don't
/// glow.
const THRESHOLD: f32 = 0.2;

/// The blur passes' settings, a uniform each.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    texel: [f32; 2],
    dir: [f32; 2],
    k: [f32; 4],
}

struct Targets {
    size: (u32, u32),
    /// The glowing parts, drawn at the frame's samples and resolved into `glow`.
    msaa: wgpu::TextureView,
    glow: wgpu::TextureView,
    glow_bg: wgpu::BindGroup,
    /// A quarter the size: the blur goes back and forth between them.
    a: wgpu::TextureView,
    a_bg: wgpu::BindGroup,
    b: wgpu::TextureView,
    b_bg: wgpu::BindGroup,
}

pub(crate) struct Bloom {
    /// The self-lit pages and lit windows, as they glow: their colour, fading to nothing
    /// (not to the fog's colour) in the fog.
    pub(crate) emit: wgpu::RenderPipeline,
    /// Lamps, headlights and the like (`GLOW`), as they're added to the frame.
    pub(crate) glow: wgpu::RenderPipeline,
    /// The sprites of fire, added rather than blended.
    pub(crate) sprite: wgpu::RenderPipeline,
    prefilter: wgpu::RenderPipeline,
    blur: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    /// Prefilter, across, down, and composite.
    params: [wgpu::Buffer; 4],
    params_bg: [wgpu::BindGroup; 4],
    targets: Option<Targets>,
}

impl Bloom {
    /// `shader` and `layout` are the scene's (globals, texture), `tex_layout` a texture and
    /// its sampler, `format` the frame's.
    pub(crate) fn new(device: &wgpu::Device, shader: &wgpu::ShaderModule, layout: &wgpu::PipelineLayout,
                      tex_layout: &wgpu::BindGroupLayout, format: wgpu::TextureFormat) -> Bloom {
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        // The glowing parts: tested against the frame's depth (on the same triangles, so
        // less-or-equal), writing none.
        let scene = |label, entry| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4, 3 => Snorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: Some(add), write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let emit = scene("bloom emit pipeline", "fs_emit");
        let glow = scene("bloom glow pipeline", "fs_glow");
        let sprite = scene("bloom sprite pipeline", "fs_sprite_emit");

        let post = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        });
        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom params"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom layout"),
            bind_group_layouts: &[Some(tex_layout), Some(&params_layout)],
            immediate_size: 0,
        });
        // The composite goes into the frame's own pass (with the HUD after it), so it has the
        // frame's samples and depth, which it ignores.
        let full = |label, entry, format, blend, frame: bool| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&post_layout),
            vertex: wgpu::VertexState { module: &post, entry_point: Some("vs_full"), compilation_options: Default::default(), buffers: &[] },
            primitive: Default::default(),
            depth_stencil: frame.then(|| wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: if frame { SAMPLES } else { 1 }, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &post,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let prefilter = full("bloom prefilter", "fs_prefilter", FORMAT, None, false);
        let blur = full("bloom blur", "fs_blur", FORMAT, None, false);
        let composite = full("bloom composite", "fs_composite", format, Some(add), true);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let params: [wgpu::Buffer; 4] = std::array::from_fn(|_| device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let params_bg = std::array::from_fn(|k| device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom params"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: params[k].as_entire_binding() }],
        }));
        Bloom { emit, glow, sprite, prefilter, blur, composite, tex_layout: tex_layout.clone(), sampler, params, params_bg, targets: None }
    }

    /// Makes the textures for a frame of `size`, if they aren't that size already.
    pub(crate) fn ensure_targets(&mut self, device: &wgpu::Device, size: (u32, u32)) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let make = |(w, h): (u32, u32), samples, label| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: w.max(1), height: h.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: samples,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | if samples == 1 { wgpu::TextureUsages::TEXTURE_BINDING } else { wgpu::TextureUsages::empty() },
                view_formats: &[],
            }).create_view(&Default::default())
        };
        let bind = |view: &wgpu::TextureView| device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom texture"),
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        let small = (size.0.div_ceil(DOWN), size.1.div_ceil(DOWN));
        let glow = make(size, 1, "bloom glow");
        let (a, b) = (make(small, 1, "bloom a"), make(small, 1, "bloom b"));
        self.targets = Some(Targets {
            size,
            msaa: make(size, SAMPLES, "bloom msaa"),
            glow_bg: bind(&glow),
            glow,
            a_bg: bind(&a),
            a,
            b_bg: bind(&b),
            b,
        })
    }

    /// The pass the glowing parts are drawn in, against the frame's depth `depth`.
    pub(crate) fn begin_glow<'e>(&self, encoder: &'e mut wgpu::CommandEncoder, depth: &wgpu::TextureView) -> wgpu::RenderPass<'e> {
        let t = self.targets.as_ref().expect("bloom targets");
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("bloom glow"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &t.msaa,
                depth_slice: None,
                resolve_target: Some(&t.glow),
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Discard },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
                stencil_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store }),
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        })
    }

    /// Brings the glow down, takes off what's too dim and blurs it, ready for
    /// [`Bloom::composite`] at `strength`.
    pub(crate) fn blur(&self, queue: &wgpu::Queue, encoder: &mut wgpu::CommandEncoder, strength: f32) {
        let t = self.targets.as_ref().expect("bloom targets");
        let small = (t.size.0.div_ceil(DOWN) as f32, t.size.1.div_ceil(DOWN) as f32);
        let full = [1.0 / t.size.0 as f32, 1.0 / t.size.1 as f32];
        let tiny = [1.0 / small.0, 1.0 / small.1];
        let params = [
            Params { texel: full, dir: [0.0; 2], k: [THRESHOLD, 0.0, 0.0, 0.0] },
            Params { texel: tiny, dir: [1.0, 0.0], k: [0.0; 4] },
            Params { texel: tiny, dir: [0.0, 1.0], k: [0.0; 4] },
            Params { texel: tiny, dir: [0.0; 2], k: [0.0, strength, 0.0, 0.0] },
        ];
        for (buf, p) in self.params.iter().zip(params) {
            queue.write_buffer(buf, 0, bytemuck::bytes_of(&p));
        }
        // Prefilter into a, then across into b and down into a, twice over.
        let steps = [
            (&self.prefilter, &t.glow_bg, &self.params_bg[0], &t.a),
            (&self.blur, &t.a_bg, &self.params_bg[1], &t.b),
            (&self.blur, &t.b_bg, &self.params_bg[2], &t.a),
            (&self.blur, &t.a_bg, &self.params_bg[1], &t.b),
            (&self.blur, &t.b_bg, &self.params_bg[2], &t.a),
        ];
        for (pipeline, src, params, dst) in steps {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom blur"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, src, &[]);
            pass.set_bind_group(1, params, &[]);
            pass.draw(0..3, 0..1);
        }
    }

    /// Adds the blurred glow over the frame, in the frame's pass.
    pub(crate) fn composite(&self, pass: &mut wgpu::RenderPass<'_>) {
        let t = self.targets.as_ref().expect("bloom targets");
        pass.set_pipeline(&self.composite);
        pass.set_bind_group(0, &t.a_bg, &[]);
        pass.set_bind_group(1, &self.params_bg[3], &[]);
        pass.draw(0..3, 0..1);
    }
}

/// How strongly the glow is added at `strength`, under a sky of `sky`: fully at night, a
/// quarter of that under a white sky, so daylight barely changes.
pub(crate) fn by_daylight(strength: f32, [r, g, b]: [u8; 3]) -> f32 {
    let lum = (0.3 * r as f32 + 0.59 * g as f32 + 0.11 * b as f32) / 255.0;
    strength * (1.0 - 0.75 * lum)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_glows_most_at_night() {
        assert_eq!(super::by_daylight(1.0, [0, 0, 0]), 1.0);
        assert!((super::by_daylight(1.0, [255, 255, 255]) - 0.25).abs() < 1e-3);
        assert_eq!(super::by_daylight(0.0, [10, 10, 10]), 0.0);
    }
}
