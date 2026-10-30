//! The glyph shader's GPU objects: pipeline, atlas texture and sampler,
//! viewport uniform, instance buffer.

use crate::error::RenderError;
use crate::glyph_atlas::GlyphAtlas;
use crate::glyph_instance::{BYTES_PER_INSTANCE, GlyphInstance};

/// Most glyphs one frame may hold. Eight bars of notes, chord symbols and
/// the wheel's labels need a few hundred.
const MAX_GLYPHS: usize = 8192;

/// Size of the viewport uniform: two f32 for the size, two f32 padding.
const VIEWPORT_UNIFORM_BYTES: u64 = 16;

/// Where each field of a `GlyphInstance` record sits, and which shader
/// input it feeds: rectangle, atlas rectangle, colour, distance scale.
const GLYPH_ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 16,
        shader_location: 1,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x4,
        offset: 32,
        shader_location: 2,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32,
        offset: 48,
        shader_location: 3,
    },
];

pub struct GlyphPipeline {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

impl GlyphPipeline {
    /// Builds the pipeline and uploads the atlas image to the GPU.
    #[must_use]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
        atlas: &GlyphAtlas,
    ) -> GlyphPipeline {
        let shader: wgpu::ShaderModule =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("glyph shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("glyph_shader.wgsl").into()),
            });
        let (texture_view, sampler): (wgpu::TextureView, wgpu::Sampler) =
            upload_atlas(device, queue, atlas);
        let bind_group_layout: wgpu::BindGroupLayout = glyph_bind_group_layout(device);
        let pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("glyph pipeline layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let pipeline: wgpu::RenderPipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("glyph pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: BYTES_PER_INSTANCE as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &GLYPH_ATTRIBUTES,
                    })],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target_format,
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            });
        let viewport_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph viewport"),
            size: VIEWPORT_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyph bindings"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let instance_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("glyph instances"),
            size: (BYTES_PER_INSTANCE * MAX_GLYPHS) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        GlyphPipeline {
            pipeline,
            viewport_buffer,
            bind_group,
            instance_buffer,
            instance_count: 0,
        }
    }

    /// Uploads this frame's glyphs and the window size they are measured in.
    ///
    /// # Errors
    ///
    /// When there are more glyphs than the instance buffer holds.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        width_pixels: f32,
        height_pixels: f32,
        glyphs: &[GlyphInstance],
    ) -> Result<(), RenderError> {
        if glyphs.len() > MAX_GLYPHS {
            return Err(RenderError::TooManyGlyphs {
                given: glyphs.len(),
                capacity: MAX_GLYPHS,
            });
        }
        let mut viewport_bytes: Vec<u8> = Vec::with_capacity(16);
        for float in [width_pixels, height_pixels, 0.0, 0.0] {
            viewport_bytes.extend_from_slice(&float.to_ne_bytes());
        }
        queue.write_buffer(&self.viewport_buffer, 0, &viewport_bytes);
        let mut bytes: Vec<u8> = Vec::with_capacity(glyphs.len() * BYTES_PER_INSTANCE);
        for glyph in glyphs {
            glyph.write_bytes(&mut bytes);
        }
        queue.write_buffer(&self.instance_buffer, 0, &bytes);
        // Fits: checked against MAX_GLYPHS above.
        #[allow(clippy::cast_possible_truncation)]
        let count: u32 = glyphs.len() as u32;
        self.instance_count = count;
        Ok(())
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}

/// Uploads the atlas image as a one-channel texture and makes the sampler
/// the shader reads it with.
fn upload_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: &GlyphAtlas,
) -> (wgpu::TextureView, wgpu::Sampler) {
    let atlas_size: wgpu::Extent3d = wgpu::Extent3d {
        width: atlas.width_pixels,
        height: atlas.height_pixels,
        depth_or_array_layers: 1,
    };
    // One channel, 0 to 255 read as 0.0 to 1.0: the distance field.
    let texture: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyph atlas"),
        size: atlas_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &atlas.image,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            // One byte per pixel.
            bytes_per_row: Some(atlas.width_pixels),
            rows_per_image: Some(atlas.height_pixels),
        },
        atlas_size,
    );
    let texture_view: wgpu::TextureView =
        texture.create_view(&wgpu::TextureViewDescriptor::default());
    // Linear: the shader reads the field between its pixels, which is
    // what keeps enlarged edges smooth.
    let sampler: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("glyph atlas"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..wgpu::SamplerDescriptor::default()
    });
    (texture_view, sampler)
}

/// Viewport uniform (binding 0), atlas texture (1), sampler (2).
fn glyph_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("glyph layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
    })
}
