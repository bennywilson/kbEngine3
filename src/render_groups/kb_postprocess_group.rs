use wgpu::util::DeviceExt;

use crate::{kb_assets::*, kb_config::*, kb_resource::*};

pub struct KbPostprocessRenderGroup {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub postprocess_uniform: PostProcessUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub bind_group: wgpu::BindGroup,
}

impl KbPostprocessRenderGroup {
    pub async fn new(device_resources: &KbDeviceResources<'_>, asset_manager: &mut KbAssetManager) -> Self {
        let device = &device_resources.device;
        let surface_config = &device_resources.surface_config;
        let render_texture = &device_resources.render_textures[0];

        // Post Process Pipeline
        let postprocess_shader_handle = asset_manager.load_shader("/engine_assets/shaders/postprocess_uber.wgsl", &device_resources).await;
        let postprocess_shader = asset_manager.get_shader(&postprocess_shader_handle);
        
        let postprocess_uniform = PostProcessUniform {
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("uniform_buffer"),
                contents: bytemuck::cast_slice(&[postprocess_uniform]),
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

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
            ],
            label: Some("bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbSpriteDrawInstance::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let postprocess_tex_handle = asset_manager.load_texture("/engine_assets/textures/postprocess_filter.png", &device_resources).await;
        let postprocess_tex = asset_manager.get_texture(&postprocess_tex_handle);
        let bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&postprocess_tex.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&postprocess_tex.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&render_texture.view),
                    },
                ],
                label: Some("bind_group"),
            }
        );
 
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
        KbPostprocessRenderGroup {
            pipeline,
            postprocess_uniform,
            uniform_buffer,
            uniform_bind_group,
            bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render(&mut self, target_view: &wgpu::TextureView, device_resources: &mut KbDeviceResources, game_config: &KbConfig, postprocess_override: Option<KbPostProcessMode>) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbPostprocessRenderGroup::render()"),
		});

        let color_attachment = Some(
            wgpu::RenderPassColorAttachment {
                view: &target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
            }});

        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Postprocess"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        self.postprocess_uniform.time_mode_unused_unused[0] = game_config.start_time.elapsed().as_secs_f32();
        self.postprocess_uniform.time_mode_unused_unused[1] = {
            let postprocess_mode = match postprocess_override {
                Some(p) => { p.clone() }
                None => { game_config.postprocess_mode.clone() }
            };
            match postprocess_mode {
                KbPostProcessMode::Desaturation => { 1.0 }
                KbPostProcessMode::ScanLines => { 2.0 }
                KbPostProcessMode::Warp => { 3.0 }
                _ => { 0.0 }
            }
        };

        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.postprocess_uniform]));
        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);

        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}