use std::mem::size_of;
 
use cgmath::Transform;
use wgpu::util::DeviceExt;

use wgpu::{
    BindGroupLayoutEntry, BindingType, SamplerBindingType, ShaderStages, TextureSampleType, TextureViewDimension
};

use crate::{kb_assets::*, kb_config::*, kb_game_object::*, kb_resource::*, kb_utils::*};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbSunbeamInstance {
    pub pos_scale: [f32; 4],
}

impl KbSunbeamInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<KbSunbeamInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }

}
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbSunbeamUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos:[f32; 4],
    pub camera_dir:[f32; 4],
    pub screen_dimensions: [f32; 4],
    pub uv_scale_offset: [f32; 4],
    pub extra_data: [f32; 4],
}

pub struct KbSunbeamRenderGroup {
    pub mask_pipeline: wgpu::RenderPipeline,
    pub draw_pipeline: wgpu::RenderPipeline,
    pub sunbeam_uniform: KbSunbeamUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub tex_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
}

impl KbSunbeamRenderGroup {
    pub async fn new(device_resources: &KbDeviceResources<'_>, asset_manager: &mut KbAssetManager) -> Self {
        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;

        // Post Process Pipeline
        let mask_shader_handle = asset_manager.load_shader("/engine_assets/shaders/sunbeam_mask.wgsl", &device_resources).await;
        let mask_shader = asset_manager.get_shader(&mask_shader_handle);

        let sunbeam_uniform = KbSunbeamUniform {
            ..Default::default()
        };
        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::cast_slice(&[sunbeam_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
            ],
            label: Some("uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("bind_group"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });
        let mask_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &mask_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &mask_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
           primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        
        // Draw pipeline
        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("KbModel_texture_bind_group_layout"),
        });

        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&device_resources.render_textures[2].view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&device_resources.render_textures[2].sampler),
                    },
                ],
                label: Some("KbModel_tex_bind_group"),
            }
        );
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let additive_blend_state = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };

        let draw_shader_handle = asset_manager.load_shader("/engine_assets/shaders/sunbeam_draw.wgsl", &device_resources).await;
        let draw_shader = asset_manager.get_shader(&draw_shader_handle);
        let draw_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSunbeamInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(additive_blend_state),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
           primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX
            }
        );
    
        let instance_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            mapped_at_creation: false,
            size: (size_of::<KbSunbeamInstance>() * 50 as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        KbSunbeamRenderGroup {
            mask_pipeline,
            draw_pipeline,
            sunbeam_uniform,
            uniform_buffer,
            uniform_bind_group,
            tex_bind_group,
            vertex_buffer,
            index_buffer,
            instance_buffer,
        }
    }

    pub fn render(&mut self, device_resources: &mut KbDeviceResources, camera: &KbCamera, game_config: &KbConfig) {
        self.render_mask(device_resources, camera, game_config);
        self.render_beams(device_resources, camera, game_config);
    }

    pub fn render_mask(&mut self, device_resources: &mut KbDeviceResources, camera: &KbCamera, game_config: &KbConfig) {
        let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSunbeamRenderGroup::render()"),
		});

        // Mask Pass
        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &device_resources.render_textures[2].view,//&device_resources.render_textures[2].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0, }),
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sunbeams Mask"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let proj_matrix = cgmath::perspective(cgmath::Deg(game_config.fov), game_config.window_width as f32 / game_config.window_height as f32, 0.1, 10000.0);
        let (view_matrix, view_dir, _) = camera.calculate_view_matrix();
        let sunbeam_uniform = KbSunbeamUniform {
            view_proj: (proj_matrix * view_matrix).into(),
            camera_pos: [camera.get_position().x, camera.get_position().y, camera.get_position().z, 0.0],
            camera_dir: [view_dir.x, view_dir.y, view_dir.z, 0.0],
            screen_dimensions: [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0],
            uv_scale_offset: [1.0, 1.0, 0.0, 0.0],
            extra_data: [0.0, 0.0, 0.0, 0.0],
        };
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[sunbeam_uniform]));

        render_pass.set_pipeline(&self.mask_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }

    pub fn render_beams(&mut self, device_resources: &mut KbDeviceResources, camera: &KbCamera, game_config: &KbConfig) {
        let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSunbeamRenderGroup::render()"),
		});

        // Mask Pass
        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &device_resources.render_textures[0].view,//&device_resources.render_textures[2].view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,//Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.0, }),
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sunbeams Draw"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: Some(
                wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        let proj_matrix = cgmath::perspective(cgmath::Deg(game_config.fov), game_config.window_width as f32 / game_config.window_height as f32, 0.1, 10000.0);
        let (view_matrix, view_dir, _) = camera.calculate_view_matrix();
        let sunbeam_uniform = KbSunbeamUniform {
            view_proj: (proj_matrix * view_matrix).into(),
            camera_pos: [camera.get_position().x, camera.get_position().y, camera.get_position().z, 0.0],
            camera_dir: [view_dir.x, view_dir.y, view_dir.z, 0.0],
            screen_dimensions: [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0],
            uv_scale_offset: [01.0, 1.0, 0.0, 0.0],
            extra_data: [0.0, 0.0, 0.0, 0.0],
        };
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[sunbeam_uniform]));

        let sun_position = CgPoint::new(500.0, 650.0, 600.0);
        let sun_position = (proj_matrix * view_matrix).transform_point(sun_position);
        let mut beam_instances = Vec::<KbSunbeamInstance>::new();
        let mut scale = 1.0;

        for _ in 0..20 {
            beam_instances.push(KbSunbeamInstance {
                pos_scale: [sun_position.x, sun_position.y, scale, scale],
            });
            scale= scale + 0.1;
        }
        device_resources.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(beam_instances.as_slice()));
   
        render_pass.set_pipeline(&self.draw_pipeline);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &self.tex_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..beam_instances.len() as _);
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}