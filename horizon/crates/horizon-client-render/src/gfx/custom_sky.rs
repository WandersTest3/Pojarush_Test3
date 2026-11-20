//! Draws the resource packs' custom skies (`crate::custom_sky`): one textured cube per layer
//! around the camera, after the sky's colour, each with its pack's blend.
use crate::custom_sky::{Blend, CustomSky, LayerDraw};
use glam::Mat4;
use std::collections::HashMap;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    clip_from_sky: [[f32; 4]; 4],
    color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 3],
    uv: [f32; 2],
}

struct GpuLayer {
    uniform: wgpu::Buffer,
    group: wgpu::BindGroup,
}

pub(super) struct CustomSkyRenderer {
    layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    shader: wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    pipelines: HashMap<Blend, wgpu::RenderPipeline>,
    sampler: wgpu::Sampler,
    vertices: wgpu::Buffer,
    layers: Vec<GpuLayer>,
    draws: Vec<(usize, Blend)>,
}

/// The cube's 36 vertices: FabricSkyboxes' face order and turns (bottom, north, south, top,
/// east, west), each face a cell of the texture's 3 by 2 grid, as OptiFine lays them out.
fn cube() -> Vec<Vertex> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let (third, half) = (1.0 / 3.0, 0.5);
    let cells = [
        (0.0, 0.0),
        (third, half),
        (2.0 * third, 0.0),
        (third, 0.0),
        (2.0 * third, half),
        (0.0, half),
    ];
    let turns = [
        Mat4::IDENTITY,
        Mat4::from_rotation_x(FRAC_PI_2),
        Mat4::from_rotation_x(-FRAC_PI_2) * Mat4::from_rotation_y(PI),
        Mat4::from_rotation_x(PI),
        Mat4::from_rotation_z(FRAC_PI_2) * Mat4::from_rotation_y(-FRAC_PI_2),
        Mat4::from_rotation_z(-FRAC_PI_2) * Mat4::from_rotation_y(FRAC_PI_2),
    ];
    let mut out = Vec::with_capacity(36);
    for ((u0, v0), turn) in cells.into_iter().zip(turns) {
        let (u1, v1) = (u0 + third, v0 + half);
        let corner = |x: f32, z: f32, u: f32, v: f32| Vertex {
            pos: turn.transform_point3(glam::Vec3::new(x, -100.0, z)).to_array(),
            uv: [u, v],
        };
        let q = [
            corner(-100.0, -100.0, u0, v0),
            corner(-100.0, 100.0, u0, v1),
            corner(100.0, 100.0, u1, v1),
            corner(100.0, -100.0, u1, v0),
        ];
        out.extend([q[0], q[1], q[2], q[0], q[2], q[3]]);
    }
    out
}

/// OptiFine's `Blender.setupBlend` for each method; the target's alpha is kept.
fn blend_state(blend: Blend) -> Option<wgpu::BlendState> {
    use wgpu::BlendFactor as F;
    let color = |src_factor, dst_factor| wgpu::BlendComponent {
        src_factor,
        dst_factor,
        operation: wgpu::BlendOperation::Add,
    };
    let keep = wgpu::BlendComponent {
        src_factor: F::Zero,
        dst_factor: F::One,
        operation: wgpu::BlendOperation::Add,
    };
    let c = match blend {
        Blend::Alpha => color(F::SrcAlpha, F::OneMinusSrcAlpha),
        Blend::Add => color(F::SrcAlpha, F::One),
        Blend::Subtract => color(F::OneMinusDst, F::Zero),
        Blend::Multiply => color(F::Dst, F::OneMinusSrcAlpha),
        Blend::Dodge => color(F::One, F::One),
        Blend::Burn => color(F::Zero, F::OneMinusSrc),
        Blend::Screen => color(F::One, F::OneMinusSrc),
        Blend::Overlay => color(F::Dst, F::Src),
        Blend::Replace => return None,
    };
    Some(wgpu::BlendState { color: c, alpha: keep })
}

impl CustomSkyRenderer {
    pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Custom sky bindings"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Custom sky layout"),
            bind_group_layouts: &[Some(&layout)],
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Custom sky shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("custom_sky.wgsl").into()),
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Custom sky sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Custom sky cube"),
            contents: bytemuck::cast_slice(&cube()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            layout,
            pipeline_layout,
            shader,
            format,
            pipelines: HashMap::new(),
            sampler,
            vertices,
            layers: Vec::new(),
            draws: Vec::new(),
        }
    }

    fn pipeline(&mut self, device: &wgpu::Device, blend: Blend) {
        if self.pipelines.contains_key(&blend) {
            return;
        }
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Custom sky"),
            layout: Some(&self.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: Some("vs_custom_sky"),
                compilation_options: Default::default(),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            primitive: wgpu::PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: Some("fs_custom_sky"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.format,
                    blend: blend_state(blend),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        self.pipelines.insert(blend, pipeline);
    }

    /// Uploads the packs' sky textures, replacing the last ones.
    pub(super) fn set(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, sky: &CustomSky) {
        self.layers.clear();
        self.draws.clear();
        let limit = device.limits().max_texture_dimension_2d;
        for layer in sky.all() {
            let (w, h) = (layer.width.min(limit), layer.height.min(limit));
            let usable = w == layer.width && h == layer.height && w > 0 && h > 0;
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Custom sky texture"),
                size: wgpu::Extent3d {
                    width: if usable { w } else { 1 },
                    height: if usable { h } else { 1 },
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            });
            let (data, w, h) = if usable { (&layer.rgba[..], w, h) } else { (&[0u8; 4][..], 1, 1) };
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * w),
                    rows_per_image: Some(h),
                },
                wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            );
            let view = texture.create_view(&Default::default());
            let uniform = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Custom sky layer"),
                size: std::mem::size_of::<Uniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Custom sky layer"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                ],
            });
            self.pipeline(device, layer.blend);
            self.layers.push(GpuLayer { uniform, group });
        }
    }

    pub(super) fn has_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    /// This frame's layers: their turns times `clip_from_view` (camera-relative) and colours.
    pub(super) fn prepare(
        &mut self,
        uploads: &mut super::upload::Uploads,
        encoder: &mut wgpu::CommandEncoder,
        clip_from_view: Mat4,
        draws: &[LayerDraw],
    ) {
        self.draws.clear();
        for d in draws {
            let Some(layer) = self.layers.get(d.index) else { continue };
            if !self.pipelines.contains_key(&d.blend) {
                continue;
            }
            let uniform = Uniform {
                clip_from_sky: (clip_from_view * d.rotation).to_cols_array_2d(),
                color: d.color,
            };
            uploads.buffer(encoder, &layer.uniform, 0, bytemuck::bytes_of(&uniform));
            self.draws.push((d.index, d.blend));
        }
    }

    pub(super) fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.draws.is_empty() {
            return;
        }
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        for (index, blend) in &self.draws {
            let (Some(layer), Some(pipeline)) = (self.layers.get(*index), self.pipelines.get(blend)) else {
                continue;
            };
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &layer.group, &[]);
            pass.draw(0..36, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cube_has_six_faces_at_the_sky_distance() {
        let c = cube();
        assert_eq!(c.len(), 36);
        for v in &c {
            let m = v.pos.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
            assert!((m - 100.0).abs() < 0.01);
        }
    }
}
