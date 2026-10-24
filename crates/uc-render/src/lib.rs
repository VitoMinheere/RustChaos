//! wgpu renderer for `uc-world` geometry.
//!
//! - One static vertex/index buffer for the level, drawn per (chunk, layer, page) range.
//! - Chunks outside the view frustum are skipped.
//! - Textures get CPU-generated mipmaps; alpha below 0.5 is cut out in the shader.
//! - Distance fog fades to the sky colour.
//! - 4x MSAA. Rendering can target a window surface or an offscreen image (screenshots,
//!   headless tests).
//! - A 2D overlay (text, rectangles) on top, for the HUD.

mod overlay;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3, Vec4};
use std::collections::{BTreeMap, HashMap};
use uc_formats::txc::Texture;
use uc_world::{ghost_page, Batch, Layer, Vertex, WorldGeometry, ADDITIVE_PAGES, DECAL_PAGES, EXPLODE_PAGE, GLOW, MIST, MOON_PAGE, SHADOW, SKY_PAGE, UNTEXTURED};
use wgpu::util::DeviceExt;

pub use overlay::Overlay;
pub use wgpu;

const SAMPLES: u32 = 4;
const DEPTH: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    cam_pos: [f32; 4],
    fog_color: [f32; 4],
    fog: [f32; 4],
    lights: [[f32; 4]; MAX_LIGHTS],
    light_rgb: [[f32; 4]; MAX_LIGHTS],
    light_count: [f32; 4],
}

/// How many dynamic lights the shader takes: the nearest to the camera.
pub const MAX_LIGHTS: usize = 16;

/// A dynamic light: where it is and how far it reaches (square units), and what it adds to
/// the light where it reaches (0 to 1, falling off to nothing at its edge).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Light {
    pub pos: Vec3,
    pub radius: f32,
    pub rgb: [f32; 3],
}

/// A camera in square units.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    pub fov_y: f32,
    /// Orthographic, `Some(half height in squares)`, looking straight down with -z up
    /// (for maps).
    pub ortho: Option<f32>,
}

impl Camera {
    pub fn view_proj(&self, size: (u32, u32)) -> Mat4 {
        let aspect = size.0 as f32 / size.1.max(1) as f32;
        if let Some(h) = self.ortho {
            let proj = glam::camera::rh::proj::directx::orthographic(-h * aspect, h * aspect, -h, h, 0.1, 400.0);
            return proj * glam::camera::rh::view::look_at_mat4(self.pos, self.target, Vec3::NEG_Z);
        }
        let proj = glam::camera::rh::proj::directx::perspective(self.fov_y, aspect, 0.02, 200.0);
        proj * glam::camera::rh::view::look_at_mat4(self.pos, self.target, Vec3::Y)
    }

    /// Screen position in pixels of point `p` (square units), and whether it is in front of
    /// the camera. Points behind are mirrored, so the direction stays useful for edge arrows.
    pub fn project(&self, p: Vec3, size: (u32, u32)) -> (glam::Vec2, bool) {
        let c = self.view_proj(size) * p.extend(1.0);
        let w = if c.w.abs() < 1e-4 { 1e-4 } else { c.w };
        let ndc = glam::vec2(c.x / w, c.y / w) * if c.w < 0.0 { -1.0 } else { 1.0 };
        (glam::vec2((ndc.x + 1.0) * 0.5 * size.0 as f32, (1.0 - ndc.y) * 0.5 * size.1 as f32), c.w > 0.0)
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub sky: [u8; 3],
    /// Fog start/end distance in squares; None disables fog.
    pub fog: Option<(f32, f32)>,
    /// Which layers to draw.
    pub layers: Vec<Layer>,
    /// Crinkles are on: the walls with one in chunks near the camera are left out, for the
    /// dynamic batches to draw raised (`uc_world::Relief::draw`).
    pub crinkles: bool,
    /// The dynamic lights this frame; the nearest [`MAX_LIGHTS`] to the camera are drawn.
    pub lights: Vec<Light>,
}

/// Draw statistics for the last frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stats {
    pub chunks_drawn: usize,
    pub chunks_total: usize,
    pub draw_calls: usize,
    pub triangles: usize,
}

struct DrawRange {
    layer: Layer,
    /// A wall with a crinkle, drawn flat.
    crinkled: bool,
    page: u32,
    first: u32,
    count: u32,
}

struct ChunkDraws {
    min: Vec3,
    max: Vec3,
    ranges: Vec<DrawRange>,
}

struct Targets {
    size: (u32, u32),
    msaa: wgpu::TextureView,
    depth: wgpu::TextureView,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    globals_buf: wgpu::Buffer,
    globals_bg: wgpu::BindGroup,
    tex_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    textures: HashMap<u32, wgpu::BindGroup>,
    white: wgpu::BindGroup,
    shadow_pipeline: wgpu::RenderPipeline,
    decal_pipeline: wgpu::RenderPipeline,
    glow_pipeline: wgpu::RenderPipeline,
    additive_pipeline: wgpu::RenderPipeline,
    sprite_pipeline: wgpu::RenderPipeline,
    ghost_depth_pipeline: wgpu::RenderPipeline,
    ghost_pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    moon_pipeline: wgpu::RenderPipeline,
    shadow_tex: wgpu::BindGroup,
    mist_tex: wgpu::BindGroup,
    statics: Option<(wgpu::Buffer, wgpu::Buffer)>,
    chunks: Vec<ChunkDraws>,
    dynamic: Option<(wgpu::Buffer, wgpu::Buffer, usize, usize)>,
    targets: Option<Targets>,
    overlay_pipeline: wgpu::RenderPipeline,
    screen_buf: wgpu::Buffer,
    screen_bg: wgpu::BindGroup,
    font: wgpu::BindGroup,
    overlay_buf: Option<(wgpu::Buffer, usize)>,
    backdrop: Option<wgpu::BindGroup>,
    minimap: Option<wgpu::BindGroup>,
    faces: [Option<wgpu::BindGroup>; 2],
    pub stats: Stats,
}

/// Box-filtered mip chain (each level half the size, down to 1x1). Square power-of-two
/// textures are the norm in this game; other sizes are clamped per axis.
fn mip_chain(t: &Texture) -> Vec<(u32, u32, Vec<u8>)> {
    let mut levels = vec![(t.width, t.height, t.rgba.clone())];
    while let Some((w, h, px)) = levels.last() {
        if *w == 1 && *h == 1 {
            break;
        }
        let (nw, nh) = ((*w / 2).max(1), (*h / 2).max(1));
        let mut out = vec![0u8; (nw * nh * 4) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let mut acc = [0u32; 4];
                for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                    let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                    let i = ((sy * w + sx) * 4) as usize;
                    for k in 0..4 {
                        acc[k] += px[i + k] as u32;
                    }
                }
                let o = ((y * nw + x) * 4) as usize;
                for k in 0..4 {
                    out[o + k] = (acc[k] / 4) as u8;
                }
            }
        }
        levels.push((nw, nh, out));
    }
    levels
}

impl Renderer {
    /// Creates a renderer drawing into targets of `format` (a surface format or an offscreen one).
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat, nearest: bool) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("uc shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("texture"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("uc layout"),
            bind_group_layouts: &[Some(&globals_layout), Some(&tex_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("uc pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            // No culling: the original data doesn't use a consistent winding.
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buf.as_entire_binding() }],
        });
        let filter = if nearest { wgpu::FilterMode::Nearest } else { wgpu::FilterMode::Linear };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("textures"),
            // Every quad maps one whole texture (0..1); clamping avoids filtering seams
            // where the texture's opposite edge would wrap in.
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: filter,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: if nearest { 1 } else { 8 },
            ..Default::default()
        });
        let white = make_texture(&device, &queue, &tex_layout, &sampler,
                                 &Texture { width: 1, height: 1, has_alpha: false, rgba: vec![255; 4] });

        // Blob shadows: blended, no depth writes, pulled slightly towards the camera.
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: -8, slope_scale: -1.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shadow"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Marks on the ground (footprints, tyre tracks): the page's own texture blended the
        // same way, tinted by the vertex colour.
        let decal_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("decal pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: -8, slope_scale: -1.0, clamp: 0.0 },
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_decal"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Explosions: the same round blobs added to the scene instead of blended over it.
        let glow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("glow pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
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
                module: &shader,
                entry_point: Some("fs_glow"),
                compilation_options: Default::default(),
                // The shader premultiplies by the alpha it works out, so both factors are one.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Ribbons of smoke (`POLY_PAGE_SMOKER`): their page's texture added to the scene the same
        // way, the texture wrapping.
        let additive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("additive pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
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
                module: &shader,
                entry_point: Some("fs_additive"),
                compilation_options: Default::default(),
                // The shader premultiplies by the alpha it works out, so both factors are one.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::OVER,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Sprites of fire (`POLY_PAGE_EXPLODE1`): the texture blended over the scene by its
        // alpha, with no depth writes.
        let sprite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
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
                module: &shader,
                entry_point: Some("fs_sprite"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // Ghosts (#186), in two passes over the rest of the scene: their depth alone, then
        // their colour blended by the vertex alpha where they're nearest, so a ghost is one
        // see-through body rather than every limb through every other.
        let ghost = |label, entry, depth_write, blend, write_mask| device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(depth_write),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let ghost_depth_pipeline = ghost("ghost depth pipeline", "fs_ghost_depth", true, None, wgpu::ColorWrites::empty());
        let ghost_pipeline = ghost("ghost pipeline", "fs_ghost", false, Some(wgpu::BlendState::ALPHA_BLENDING), wgpu::ColorWrites::ALL);
        // The sky: drawn first, opaque, with no depth test and no fog (`POLY_PAGE_SKY`).
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        // The moon: added over the sky (`POLY_PAGE_MOON`, `D3DBLEND_ONE` both ways).
        let moon_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("moon pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState { color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add }, alpha: wgpu::BlendComponent::OVER }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_tex = make_texture(&device, &queue, &tex_layout, &sampler, &blob_texture());
        let mist_tex = make_texture(&device, &queue, &tex_layout, &sampler, &cloud_texture());

        // Overlay: same pass (so same MSAA and depth format), no depth test, alpha blending.
        let oshader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("overlay shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("overlay.wgsl").into()),
        });
        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &oshader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<overlay::OVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4],
                })],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState { count: SAMPLES, ..Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &oshader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let screen_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screen"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let screen_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("screen"),
            layout: &globals_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: screen_buf.as_entire_binding() }],
        });
        let font_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("font"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let (fw, fh, fpx) = overlay::font_atlas();
        let font_tex = Texture { width: fw, height: fh, has_alpha: true, rgba: fpx };
        let font = make_texture_levels(&device, &queue, &tex_layout, &font_sampler, &font_tex, 1);
        Renderer {
            device, queue, format, pipeline, globals_buf, globals_bg, tex_layout, sampler,
            textures: HashMap::new(), white, shadow_pipeline, decal_pipeline, glow_pipeline, additive_pipeline, sprite_pipeline, ghost_depth_pipeline, ghost_pipeline, sky_pipeline, moon_pipeline, shadow_tex, mist_tex,
            statics: None, chunks: Vec::new(), dynamic: None, targets: None,
            overlay_pipeline, screen_buf, screen_bg, font, overlay_buf: None, backdrop: None, minimap: None, faces: [None, None], stats: Stats::default(),
        }
    }

    /// Creates a renderer without a window, for offscreen rendering.
    pub fn headless(nearest: bool) -> Result<Renderer, String> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .map_err(|e| format!("no graphics adapter: {e}"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| format!("no graphics device: {e}"))?;
        Ok(Renderer::new(device, queue, wgpu::TextureFormat::Rgba8Unorm, nearest))
    }

    fn make_texture(&self, t: &Texture) -> wgpu::BindGroup {
        make_texture(&self.device, &self.queue, &self.tex_layout, &self.sampler, t)
    }

    /// Uploads the static world and the textures it (and `extra_pages`) use.
    /// Returns (textures loaded, of which from overrides).
    pub fn upload_world(&mut self, geo: &WorldGeometry, extra_pages: &[u32],
                        texture: impl Fn(u32) -> Option<(Texture, bool)>) -> (usize, usize) {
        let (mut vertices, mut indices): (Vec<Vertex>, Vec<u32>) = (Vec::new(), Vec::new());
        self.chunks.clear();
        for c in &geo.chunks {
            let mut ranges = Vec::new();
            let crinkled = c.crinkled.iter().map(|(&page, b)| (Layer::Walls, true, page, b));
            for (layer, crinkled, page, b) in c.batches.iter().map(|(&(l, p), b)| (l, false, p, b)).chain(crinkled) {
                let base = vertices.len() as u32;
                let first = indices.len() as u32;
                vertices.extend_from_slice(&b.vertices);
                indices.extend(b.indices.iter().map(|i| i + base));
                ranges.push(DrawRange { layer, crinkled, page, first, count: b.indices.len() as u32 });
            }
            self.chunks.push(ChunkDraws { min: c.min, max: c.max, ranges });
        }
        let vb = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world vertices"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let ib = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("world indices"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        self.statics = Some((vb, ib));
        let (mut loaded, mut overridden) = (0, 0);
        for page in geo.pages().into_iter().chain(extra_pages.iter().copied()) {
            if self.textures.contains_key(&page) {
                continue;
            }
            if let Some((t, over)) = texture(page) {
                let bg = self.make_texture(&t);
                self.textures.insert(page, bg);
                loaded += 1;
                overridden += over as usize;
            }
        }
        (loaded, overridden)
    }

    fn ensure_targets(&mut self, size: (u32, u32)) {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return;
        }
        let make = |format, label| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: SAMPLES,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        self.targets = Some(Targets { size, msaa: make(self.format, "msaa colour"), depth: make(DEPTH, "depth") });
    }

    fn upload_dynamic(&mut self, batches: &BTreeMap<u32, Batch>) -> Vec<(u32, u32, u32)> {
        let (mut vertices, mut indices, mut ranges) = (Vec::new(), Vec::new(), Vec::new());
        for (&page, b) in batches {
            let base = vertices.len() as u32;
            let first = indices.len() as u32;
            vertices.extend_from_slice(&b.vertices);
            indices.extend(b.indices.iter().map(|i| i + base));
            ranges.push((page, first, b.indices.len() as u32));
        }
        if vertices.is_empty() {
            return ranges;
        }
        let need_v = vertices.len() * std::mem::size_of::<Vertex>();
        let need_i = indices.len() * 4;
        let fits = self.dynamic.as_ref().is_some_and(|d| d.2 >= need_v && d.3 >= need_i);
        if !fits {
            let mk = |size: usize, usage| {
                self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("dynamic"),
                    size: (size * 2) as u64,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                })
            };
            self.dynamic = Some((mk(need_v, wgpu::BufferUsages::VERTEX), mk(need_i, wgpu::BufferUsages::INDEX), need_v * 2, need_i * 2));
        }
        let (vb, ib, _, _) = self.dynamic.as_ref().unwrap();
        self.queue.write_buffer(vb, 0, bytemuck::cast_slice(&vertices));
        self.queue.write_buffer(ib, 0, bytemuck::cast_slice(&indices));
        ranges
    }

    /// The image `Overlay::backdrop` quads show (e.g. a menu's background).
    pub fn set_backdrop(&mut self, t: &Texture) {
        self.backdrop = Some(make_texture_levels(&self.device, &self.queue, &self.tex_layout, &self.sampler, t, 1));
    }

    /// The image `Overlay::minimap` quads show (a level's map).
    pub fn set_minimap(&mut self, t: &Texture) {
        self.minimap = Some(make_texture_levels(&self.device, &self.queue, &self.tex_layout, &self.sampler, t, 1));
    }

    /// The face sheets `Overlay::face` quads show (`face1.tga`, `face2.tga`).
    pub fn set_faces(&mut self, sheet: usize, t: &Texture) {
        self.faces[sheet.min(1)] = Some(make_texture_levels(&self.device, &self.queue, &self.tex_layout, &self.sampler, t, 1));
    }

    /// Forgets every level texture (before loading another level).
    pub fn clear_textures(&mut self) {
        self.textures.clear();
    }

    /// Draws only the overlay (menus), over black.
    pub fn render_2d(&mut self, target: &wgpu::TextureView, size: (u32, u32), overlay: &Overlay) {
        self.ensure_targets(size);
        self.queue.write_buffer(&self.screen_buf, 0, bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0.0, 0.0]));
        let count = self.upload_overlay(overlay);
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let t = self.targets.as_ref().unwrap();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("2d"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.draw_overlay(&mut pass, overlay, count);
        }
        self.queue.submit([encoder.finish()]);
    }

    /// Draws uploaded overlay vertices: backdrop quads first, then text and rectangles.
    fn draw_overlay(&self, pass: &mut wgpu::RenderPass, overlay: &Overlay, count: u32) {
        let Some((buf, _)) = &self.overlay_buf else { return };
        if count == 0 {
            return;
        }
        pass.set_pipeline(&self.overlay_pipeline);
        pass.set_bind_group(0, &self.screen_bg, &[]);
        pass.set_vertex_buffer(0, buf.slice(..));
        let nb = overlay.backdrop.len() as u32;
        let nm = nb + overlay.minimap.len() as u32;
        if let (Some(bd), true) = (&self.backdrop, nb > 0) {
            pass.set_bind_group(1, bd, &[]);
            pass.draw(0..nb, 0..1);
        }
        if let (Some(mm), true) = (&self.minimap, nm > nb) {
            pass.set_bind_group(1, mm, &[]);
            pass.draw(nb..nm, 0..1);
        }
        let nv = nm + overlay.verts.len() as u32;
        pass.set_bind_group(1, &self.font, &[]);
        pass.draw(nm..nv, 0..1);
        // The faces, over everything else.
        let mut at = nv;
        for (sheet, quads) in overlay.faces.iter().enumerate() {
            let n = quads.len() as u32;
            if let (Some(bg), true) = (&self.faces[sheet], n > 0) {
                pass.set_bind_group(1, bg, &[]);
                pass.draw(at..at + n, 0..1);
            }
            at += n;
        }
        let _ = count;
    }

    /// Uploads the overlay's vertices (backdrop quads first); returns how many to draw.
    fn upload_overlay(&mut self, overlay: &Overlay) -> u32 {
        if overlay.is_empty() {
            return 0;
        }
        let all: Vec<overlay::OVertex> =
            overlay.backdrop.iter().chain(&overlay.minimap).chain(&overlay.verts).chain(&overlay.faces[0]).chain(&overlay.faces[1]).copied().collect();
        let bytes: &[u8] = bytemuck::cast_slice(&all);
        if !self.overlay_buf.as_ref().is_some_and(|(_, cap)| *cap >= bytes.len()) {
            let cap = bytes.len().next_power_of_two();
            let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("overlay"),
                size: cap as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.overlay_buf = Some((buf, cap));
        }
        self.queue.write_buffer(&self.overlay_buf.as_ref().unwrap().0, 0, bytes);
        all.len() as u32
    }

    /// Renders one frame into `target` (a view of a texture in this renderer's format).
    pub fn render(&mut self, target: &wgpu::TextureView, size: (u32, u32), cam: &Camera, s: &Settings,
                  dynamic: &BTreeMap<u32, Batch>, overlay: &Overlay) {
        self.ensure_targets(size);
        let vp = cam.view_proj(size);
        let sky = s.sky.map(|c| c as f32 / 255.0);
        let (fog_start, fog_end, fog_on) = s.fog.map_or((0.0, 1.0, 0.0), |(a, b)| (a, b, 1.0));
        let mut near: Vec<&Light> = s.lights.iter().filter(|l| l.radius > 0.0).collect();
        near.sort_by(|a, b| (a.pos - cam.pos).length_squared().total_cmp(&(b.pos - cam.pos).length_squared()));
        near.truncate(MAX_LIGHTS);
        let mut lights = [[0.0; 4]; MAX_LIGHTS];
        let mut light_rgb = [[0.0; 4]; MAX_LIGHTS];
        for (k, l) in near.iter().enumerate() {
            lights[k] = [l.pos.x, l.pos.y, l.pos.z, l.radius];
            light_rgb[k] = [l.rgb[0], l.rgb[1], l.rgb[2], 0.0];
        }
        let globals = Globals {
            view_proj: vp.to_cols_array_2d(),
            cam_pos: [cam.pos.x, cam.pos.y, cam.pos.z, 1.0],
            fog_color: [sky[0], sky[1], sky[2], 1.0],
            fog: [fog_start, fog_end, fog_on, 0.0],
            lights,
            light_rgb,
            light_count: [near.len() as f32, 0.0, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));
        let dyn_ranges = if s.layers.contains(&Layer::Character) { self.upload_dynamic(dynamic) } else { Vec::new() };
        self.queue.write_buffer(&self.screen_buf, 0, bytemuck::cast_slice(&[size.0 as f32, size.1 as f32, 0.0, 0.0]));
        let overlay_count = self.upload_overlay(overlay);

        let planes = frustum_planes(&vp);
        let mut stats = Stats { chunks_total: self.chunks.len(), ..Default::default() };
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let t = self.targets.as_ref().unwrap();
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &t.msaa,
                    depth_slice: None,
                    resolve_target: Some(target),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: sky[0] as f64, g: sky[1] as f64, b: sky[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &t.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bg, &[]);
            // The sky first, behind everything, then the moon added over it.
            if let (Some((vb, ib, _, _)), false) = (&self.dynamic, dyn_ranges.is_empty()) {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                for (page, pipeline) in [(SKY_PAGE, &self.sky_pipeline), (MOON_PAGE, &self.moon_pipeline)] {
                    for &(_, first, count) in dyn_ranges.iter().filter(|r| r.0 == page) {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(1, self.textures.get(&page).unwrap_or(&self.white), &[]);
                        pass.draw_indexed(first..first + count, 0, 0..1);
                        stats.draw_calls += 1;
                        stats.triangles += count as usize / 3;
                    }
                }
            }
            pass.set_pipeline(&self.pipeline);
            if let Some((vb, ib)) = &self.statics {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                for c in &self.chunks {
                    if !aabb_visible(&planes, c.min, c.max) {
                        continue;
                    }
                    stats.chunks_drawn += 1;
                    let raised = s.crinkles && uc_world::crinkle::near(cam.pos, c.min, c.max);
                    for r in &c.ranges {
                        if !s.layers.contains(&r.layer) || (raised && r.crinkled) {
                            continue;
                        }
                        let bg = if r.page == UNTEXTURED { &self.white } else { self.textures.get(&r.page).unwrap_or(&self.white) };
                        pass.set_bind_group(1, bg, &[]);
                        pass.draw_indexed(r.first..r.first + r.count, 0, 0..1);
                        stats.draw_calls += 1;
                        stats.triangles += r.count as usize / 3;
                    }
                }
            }
            if let (Some((vb, ib, _, _)), false) = (&self.dynamic, dyn_ranges.is_empty()) {
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                for &(page, first, count) in dyn_ranges.iter().filter(|r| !matches!(r.0, SHADOW | GLOW | MIST | SKY_PAGE | MOON_PAGE | EXPLODE_PAGE) && !DECAL_PAGES.contains(&r.0) && !ADDITIVE_PAGES.contains(&r.0) && ghost_page(r.0).is_none()) {
                    pass.set_bind_group(1, self.textures.get(&page).unwrap_or(&self.white), &[]);
                    pass.draw_indexed(first..first + count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangles += count as usize / 3;
                }
                // Marks on the ground, over it and under everything blended.
                for &(page, first, count) in dyn_ranges.iter().filter(|r| DECAL_PAGES.contains(&r.0)) {
                    let Some(tex) = self.textures.get(&page) else { continue };
                    pass.set_pipeline(&self.decal_pipeline);
                    pass.set_bind_group(1, tex, &[]);
                    pass.draw_indexed(first..first + count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangles += count as usize / 3;
                }
                // Blobs last, over the solid geometry: shadows, blood and smoke blended
                // first, then the fire of an explosion added on top of them.
                for (page, pipeline, tex) in [
                    (MIST, &self.shadow_pipeline, &self.mist_tex),
                    (SHADOW, &self.shadow_pipeline, &self.shadow_tex),
                    (GLOW, &self.glow_pipeline, &self.shadow_tex),
                ] {
                    for &(_, first, count) in dyn_ranges.iter().filter(|r| r.0 == page) {
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(1, tex, &[]);
                        pass.draw_indexed(first..first + count, 0, 0..1);
                        stats.draw_calls += 1;
                        stats.triangles += count as usize / 3;
                    }
                }
                // Ribbons of smoke, added on top.
                for &(page, first, count) in dyn_ranges.iter().filter(|r| ADDITIVE_PAGES.contains(&r.0)) {
                    let Some(tex) = self.textures.get(&page) else { continue };
                    pass.set_pipeline(&self.additive_pipeline);
                    pass.set_bind_group(1, tex, &[]);
                    pass.draw_indexed(first..first + count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangles += count as usize / 3;
                }
                // Then the sprites of fire, over the rest, in the order they came.
                for &(_, first, count) in dyn_ranges.iter().filter(|r| r.0 == EXPLODE_PAGE) {
                    pass.set_pipeline(&self.sprite_pipeline);
                    pass.set_bind_group(1, self.textures.get(&EXPLODE_PAGE).unwrap_or(&self.white), &[]);
                    pass.draw_indexed(first..first + count, 0, 0..1);
                    stats.draw_calls += 1;
                    stats.triangles += count as usize / 3;
                }
                // Ghosts last: their depth, then their colour over what's in front of them.
                for pipeline in [&self.ghost_depth_pipeline, &self.ghost_pipeline] {
                    for &(page, first, count) in dyn_ranges.iter() {
                        let Some(own) = ghost_page(page) else { continue };
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(1, self.textures.get(&own).unwrap_or(&self.white), &[]);
                        pass.draw_indexed(first..first + count, 0, 0..1);
                        stats.draw_calls += 1;
                        stats.triangles += count as usize / 3;
                    }
                }
            }
            if overlay_count > 0 {
                self.draw_overlay(&mut pass, overlay, overlay_count);
                stats.draw_calls += 1;
            }
        }
        self.queue.submit([encoder.finish()]);
        self.stats = stats;
    }

    /// Renders offscreen and returns the image as RGBA8 rows, top row first.
    pub fn render_to_image(&mut self, size: (u32, u32), cam: &Camera, s: &Settings,
                           dynamic: &BTreeMap<u32, Batch>, overlay: &Overlay) -> Vec<u8> {
        self.capture(size, |r, view| r.render(view, size, cam, s, dynamic, overlay))
    }

    /// Like `render_2d`, offscreen: RGBA8 rows, top row first.
    pub fn render_2d_to_image(&mut self, size: (u32, u32), overlay: &Overlay) -> Vec<u8> {
        self.capture(size, |r, view| r.render_2d(view, size, overlay))
    }

    /// Renders with `draw` into an offscreen texture and reads it back.
    fn capture(&mut self, size: (u32, u32), draw: impl FnOnce(&mut Self, &wgpu::TextureView)) -> Vec<u8> {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen"),
            size: wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        draw(self, &tex.create_view(&Default::default()));
        let row = (size.0 * 4).div_ceil(256) * 256;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * size.1) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(size.1) },
            },
            wgpu::Extent3d { width: size.0, height: size.1, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        buf.map_async(wgpu::MapMode::Read, .., |_| {});
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        let data = buf.get_mapped_range(..).expect("readback buffer mapped");
        let mut out = Vec::with_capacity((size.0 * size.1 * 4) as usize);
        for y in 0..size.1 {
            let start = (y * row) as usize;
            out.extend_from_slice(&data[start..start + (size.0 * 4) as usize]);
        }
        if matches!(self.format, wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb) {
            for px in out.chunks_mut(4) {
                px.swap(0, 2);
            }
        }
        out
    }
}

/// A round shadow: black, alpha falling off smoothly from the middle to the edge.
/// The sheet a patch of mist is drawn with: soft wisps that tile, thinning out in places so
/// the ground shows through.
///
/// The original loads `server\textures\extras\fog.tga` (`TEXTURE_page_fog`), which the
/// retail release doesn't ship, so this is made up instead: value noise at a few octaves,
/// wrapped so the patch's four texture quadrants each read differently, as `MIST_create`'s
/// `type_cycle` expects.
fn cloud_texture() -> Texture {
    let n = 128usize;
    // Value noise: a lattice of random values, smoothly interpolated and wrapped.
    let mut seed = 0x9e37u32;
    let mut rand = || {
        seed = seed.wrapping_mul(69069).wrapping_add(1);
        (seed >> 16) as f32 / 65536.0
    };
    let lattice: Vec<f32> = (0..16 * 16).map(|_| rand()).collect();
    let at = |gx: i32, gy: i32, step: usize| {
        let k = 16 / step as i32;
        lattice[((gy.rem_euclid(k) * k + gx.rem_euclid(k)) as usize).min(lattice.len() - 1)]
    };
    let mut rgba = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let mut v = 0.0;
            let mut amp = 0.55;
            for step in [1usize, 2, 4] {
                let cells = 16 / step;
                let (fx, fy) = (x as f32 * cells as f32 / n as f32, y as f32 * cells as f32 / n as f32);
                let (gx, gy) = (fx.floor() as i32, fy.floor() as i32);
                let (tx, ty) = (fx - gx as f32, fy - gy as f32);
                // Smoothstep between the lattice corners.
                let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
                let (a, b) = (at(gx, gy, step), at(gx + 1, gy, step));
                let (c, d) = (at(gx, gy + 1, step), at(gx + 1, gy + 1, step));
                v += amp * ((a + (b - a) * sx) + ((c + (d - c) * sx) - (a + (b - a) * sx)) * sy);
                amp *= 0.5;
            }
            // Thin it out hard: mist wants more gap than cloud, or it reads as a slab laid
            // over the street rather than something drifting across it.
            let a = ((v - 0.46).max(0.0) * 2.4).min(0.8);
            let o = (y * n + x) * 4;
            rgba[o..o + 4].copy_from_slice(&[255, 255, 255, (a * 255.0) as u8]);
        }
    }
    Texture { width: n as u32, height: n as u32, has_alpha: true, rgba }
}

fn blob_texture() -> Texture {
    let n = 64u32;
    let mut rgba = vec![0u8; (n * n * 4) as usize];
    for y in 0..n {
        for x in 0..n {
            let (dx, dy) = ((x as f32 + 0.5) / n as f32 * 2.0 - 1.0, (y as f32 + 0.5) / n as f32 * 2.0 - 1.0);
            let r2 = (dx * dx + dy * dy).min(1.0);
            rgba[((y * n + x) * 4 + 3) as usize] = ((1.0 - r2).powf(0.7) * 255.0) as u8;
        }
    }
    Texture { width: n, height: n, has_alpha: true, rgba }
}

/// Uploads a texture with its full mip chain and returns its bind group.
fn make_texture(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout, sampler: &wgpu::Sampler,
                t: &Texture) -> wgpu::BindGroup {
    make_texture_levels(device, queue, layout, sampler, t, usize::MAX)
}

/// Like `make_texture`, with at most `max_levels` mip levels.
fn make_texture_levels(device: &wgpu::Device, queue: &wgpu::Queue, layout: &wgpu::BindGroupLayout,
                       sampler: &wgpu::Sampler, t: &Texture, max_levels: usize) -> wgpu::BindGroup {
    let mut mips = mip_chain(t);
    mips.truncate(max_levels);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        mip_level_count: mips.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Plain (non-sRGB) format: texels are used as stored, like the original.
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (w, h, px)) in mips.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(*h) },
            wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        );
    }
    let view = texture.create_view(&Default::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// The six frustum planes (a, b, c, d) of a view-projection matrix (wgpu depth 0..1).
fn frustum_planes(m: &Mat4) -> [Vec4; 6] {
    let r = [m.row(0), m.row(1), m.row(2), m.row(3)];
    [r[3] + r[0], r[3] - r[0], r[3] + r[1], r[3] - r[1], r[2], r[3] - r[2]]
}

fn aabb_visible(planes: &[Vec4; 6], min: Vec3, max: Vec3) -> bool {
    planes.iter().all(|p| {
        // The box corner furthest along the plane normal must be in front of the plane.
        let v = Vec3::new(
            if p.x >= 0.0 { max.x } else { min.x },
            if p.y >= 0.0 { max.y } else { min.y },
            if p.z >= 0.0 { max.z } else { min.z },
        );
        p.truncate().dot(v) + p.w >= 0.0
    })
}
