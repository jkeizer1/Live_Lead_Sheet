//! The line shader's GPU objects: pipeline, viewport uniform, instance
//! buffer.

use crate::error::RenderError;
use crate::line_instance::{BYTES_PER_INSTANCE, LineInstance};

/// Most lines one frame may hold. A strip of 8 bars needs a few hundred;
/// this leaves room for stems and beams.
const MAX_LINES: usize = 8192;

/// Size of the viewport uniform: two f32 for the size, two f32 padding.
const VIEWPORT_UNIFORM_BYTES: u64 = 16;

pub struct LinePipeline {
    pipeline: wgpu::RenderPipeline,
    viewport_buffer: wgpu::Buffer,
    viewport_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

impl LinePipeline {
    #[must_use]
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> LinePipeline {
        let shader: wgpu::ShaderModule =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("line shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("line_shader.wgsl").into()),
            });
        let bind_group_layout: wgpu::BindGroupLayout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("line viewport layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("line pipeline layout"),
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size: 0,
            });
        let attributes: [wgpu::VertexAttribute; 3] = [
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
                format: wgpu::VertexFormat::Float32,
                offset: 32,
                shader_location: 2,
            },
        ];
        let pipeline: wgpu::RenderPipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("line pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(wgpu::VertexBufferLayout {
                        array_stride: BYTES_PER_INSTANCE as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &attributes,
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
            label: Some("line viewport"),
            size: VIEWPORT_UNIFORM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let viewport_bind_group: wgpu::BindGroup =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("line viewport"),
                layout: &bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: viewport_buffer.as_entire_binding(),
                }],
            });
        let instance_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("line instances"),
            size: (BYTES_PER_INSTANCE * MAX_LINES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        LinePipeline {
            pipeline,
            viewport_buffer,
            viewport_bind_group,
            instance_buffer,
            instance_count: 0,
        }
    }

    /// Uploads this frame's lines and the window size they are measured in.
    ///
    /// # Errors
    ///
    /// When there are more lines than the instance buffer holds.
    pub fn prepare(
        &mut self,
        queue: &wgpu::Queue,
        width_pixels: u32,
        height_pixels: u32,
        lines: &[LineInstance],
    ) -> Result<(), RenderError> {
        if lines.len() > MAX_LINES {
            return Err(RenderError::TooManyLines {
                given: lines.len(),
                capacity: MAX_LINES,
            });
        }
        // Window sizes are far below 2^24, so the conversion to f32 is exact.
        #[allow(clippy::cast_precision_loss)]
        let size: [f32; 2] = [width_pixels as f32, height_pixels as f32];
        let mut viewport_bytes: Vec<u8> = Vec::with_capacity(16);
        for float in [size[0], size[1], 0.0, 0.0] {
            viewport_bytes.extend_from_slice(&float.to_ne_bytes());
        }
        queue.write_buffer(&self.viewport_buffer, 0, &viewport_bytes);
        let mut bytes: Vec<u8> = Vec::with_capacity(lines.len() * BYTES_PER_INSTANCE);
        for line in lines {
            line.write_bytes(&mut bytes);
        }
        queue.write_buffer(&self.instance_buffer, 0, &bytes);
        // Fits: checked against MAX_LINES above.
        #[allow(clippy::cast_possible_truncation)]
        let count: u32 = lines.len() as u32;
        self.instance_count = count;
        Ok(())
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.viewport_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}
