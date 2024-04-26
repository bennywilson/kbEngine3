use ab_glyph::FontRef;
use anyhow::*;
use cgmath::Vector3;
use image::GenericImageView;
use std::sync::Arc;

use wgpu::{BindGroupLayoutEntry, BindingType, Device, SamplerBindingType, SurfaceConfiguration, ShaderStages, TextureSampleType, TextureViewDimension, Queue, util::DeviceExt};
use wgpu_text::{BrushBuilder, TextBrush};

use crate::{kb_config::KbConfig, kb_game_object::GameObject, log, PERF_SCOPE};

#[repr(C)]  // Do what C does. The order, size, and alignment of fields is exactly what you would expect from C or C++""
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbVertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl KbVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<KbVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}
 
pub const VERTICES: &[KbVertex] = &[
    KbVertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0], },
    KbVertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0], },
    KbVertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0], },
    KbVertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0], },
];

pub const INDICES: &[u16] = &[
    0, 1, 3,
    3, 1, 2,
];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbDrawInstance {
    pub pos_scale: [f32; 4],
    pub uv_scale_bias: [f32; 4],
    pub per_instance_data: [f32; 4],
}

impl KbDrawInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<KbDrawInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 2,     // Corresponds to @location in the shader
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: 2 * std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteUniform {
    pub screen_dimensions: [f32; 4],
    pub time: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PostProcessUniform {
    pub time_mode_unused_unused: [f32;4],
}

#[allow(dead_code)] 
pub struct KbTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl KbTexture {
    pub fn new_depth_texture(device: &wgpu::Device, surface_config: &SurfaceConfiguration) -> Result<Self> {
        let size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor { // 4.
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                compare: Some(wgpu::CompareFunction::LessEqual),
                lod_min_clamp: 0.0,
                lod_max_clamp: 100.0,
                ..Default::default()
            }
        );
        Ok(KbTexture {
            texture,
            view,
            sampler
        })
    }

    pub fn new_render_texture(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) ->Result<Self> {        
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label: Some("Render Target"),
                size: wgpu::Extent3d { width: surface_config.width, height: surface_config.height, depth_or_array_layers: 1},
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: surface_config.format,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            }
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );

        Ok(KbTexture {
            texture,
            view,
            sampler
        })
    }

    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8], 
        label: &str
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>
    ) -> Result<Self> {
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
            }
        );
        
        let rgba = img.to_rgba8();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(
            &wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            }
        );
        
        Ok(Self {
            texture,
            view,
            sampler
        })
    }
}

#[allow(dead_code)]
pub enum KbRenderPassType {
    Opaque,
    Transparent,
    PostProcess,
}

#[derive(Clone)]
pub enum KbPostProcessMode {
    Passthrough,
    Desaturation,
    ScanLines,
    Warp,
}

#[allow(dead_code)]
pub struct KbDeviceResources<'a> {
    pub surface: wgpu::Surface<'a>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub instance_buffer: wgpu::Buffer,
    pub brush: TextBrush<FontRef<'a>>,
    pub render_textures: Vec<KbTexture>,    // [0] is color, [1] is depth
}

impl<'a> KbDeviceResources<'a> {
    pub fn resize(&mut self, game_config: &KbConfig) {
        assert!(game_config.window_width > 0 && game_config.window_height > 0);

        self.surface_config.width = game_config.window_width;
        self.surface_config.height = game_config.window_height;
        self.surface.configure(&self.device, &self.surface_config);

        self.render_textures[0] = KbTexture::new_render_texture(&self.device, &self.surface_config).unwrap();
        self.render_textures[1] = KbTexture::new_depth_texture(&self.device, &self.surface_config).unwrap();
    }

     pub async fn new(window: Arc::<winit::window::Window>, game_config: &KbConfig) -> Self {
        log!("Creating instance"); 
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: game_config.graphics_backend,
            ..Default::default()
        });

        log!("Creating surface + adapter");

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: game_config.graphics_power_pref,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        log!("Requesting Device");
		let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: Some("Device Descriptor"),
            },
            None, // Trace path
        ).await.unwrap();

        let surface_config = surface.get_default_config(&adapter, game_config.window_width, game_config.window_height).unwrap();
        surface.configure(&device, &surface_config);

        log!("Loading Texture");

        let max_instances = game_config.max_render_instances;
        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("instance_buffer"),
                mapped_at_creation: false,
                size: (std::mem::size_of::<KbDrawInstance>() * max_instances as usize) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            });
       
        let mut render_textures = Vec::<KbTexture>::new();
        let render_texture = KbTexture::new_render_texture(&device, &surface_config).unwrap();
        render_textures.push(render_texture);

        let depth_texture = KbTexture::new_depth_texture(&device, &surface_config).unwrap();
        render_textures.push(depth_texture);

        log!("Creating Font");

        let brush = BrushBuilder::using_font_bytes(include_bytes!("../game_assets/Bold.ttf")).unwrap()
                .build(&device, surface_config.width, surface_config.height, surface_config.format);

	    KbDeviceResources {
            surface_config,
            surface,
            adapter,
            device,
            queue,
            instance_buffer,
            brush,
            render_textures,
        }
    }
}

pub struct KbSpritePipeline {
    pub opaque_render_pipeline: wgpu::RenderPipeline,
    pub transparent_render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub uniform: SpriteUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub tex_bind_group: wgpu::BindGroup,
}

impl KbSpritePipeline {
    pub fn new(device: &Device, queue: &Queue, surface_config: &SurfaceConfiguration, game_config: &KbConfig) -> Self {
        log!("Creating KbSpritePipeline...");

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
                BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
                },
            ],
            label: Some("kbSpritePipeline: texture_bind_group_layout"),
        });
       
        let texture_bytes = include_bytes!("../game_assets/SpriteSheet.png");
        let sprite_sheet_texture = KbTexture::from_bytes(&device, &queue, texture_bytes, "SpriteSheet.png").unwrap();

        let texture_bytes = include_bytes!("../game_assets/PostProcessFilter.png");
        let postprocess_texture = KbTexture::from_bytes(&device, &queue, texture_bytes, "PostProcessFilter.png").unwrap();

        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&sprite_sheet_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sprite_sheet_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&postprocess_texture.view),
                    },
                ],
                label: Some("kbSpritePipeline: tex_bind_group"),
            }
        );

        let mut textures = Vec::<KbTexture>::new();
        textures.push(postprocess_texture);

        log!("  Creating shader");

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("BasicSprite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/BasicSprite.wgsl").into()),
        });
        
        // Uniform buffer
        let uniform = SpriteUniform {
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("sprite_uniform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
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
            label: Some("sprite_uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("model_bind_group"),
        });

        log!("  Creating pipeline");

        // Render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let opaque_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_compare: wgpu::CompareFunction::Always,
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

        let transparent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("CloudSprite.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/CloudSprite.wgsl").into()),
        });

        let transparent_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &transparent_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &transparent_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_compare: wgpu::CompareFunction::Always,
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

        log!("  Creating vertex/index buffers");

        // Vertex/Index buffer
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let instance_buffer = device.create_buffer(
        &wgpu::BufferDescriptor {
            label: Some("instance_buffer"),
            mapped_at_creation: false,
            size: (std::mem::size_of::<KbDrawInstance>() * game_config.max_render_instances as usize) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
        });

        KbSpritePipeline {
            opaque_render_pipeline,
            transparent_render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            uniform,
            uniform_buffer,
            uniform_bind_group,
            tex_bind_group,
        }
    }

    pub fn render(&mut self, render_pass_type: KbRenderPassType, should_clear: bool, device_resources: &mut KbDeviceResources, game_config: &KbConfig, game_objects: &Vec<GameObject>) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSpritePipeline::render()"),
		});

        let mut frame_instances = Vec::<KbDrawInstance>::new();

        // Create instances
        let u_scale = 1.0 / 8.0;
        let v_scale = 1.0 / 8.0;
        let extra_scale = 1.0;
        let extra_offset: Vector3<f32> = Vector3::<f32>::new(0.0, -0.35, 0.0);

        let game_object_iter = game_objects.iter();
        for game_object in game_object_iter {
            PERF_SCOPE!("Creating instances");
            let game_object_position = game_object.position + extra_offset;
            let sprite_index = game_object.sprite_index + game_object.anim_frame;
            let mut u_offset = ((sprite_index % 8) as f32) * u_scale;
            let v_offset = ((sprite_index / 8) as f32) * v_scale;
            let mul = if game_object.direction.x > 0.0 { 1.0 } else { -1.0 };
            if mul < 0.0 {
                u_offset = u_offset + u_scale;
            }

            let new_instance = KbDrawInstance {
                pos_scale: [game_object_position.x, game_object_position.y, game_object.scale.x * extra_scale, game_object.scale.y * extra_scale],
                uv_scale_bias: [u_scale * mul, v_scale, u_offset, v_offset],
                per_instance_data: [game_object.random_val, 0.0, 0.0, 0.0],
            };
            frame_instances.push(new_instance);
        }
        
        let color_attachment = {
            if should_clear {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.01,
                            b: 0.35,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })
            } else {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })
            }
        };


        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniform]));
        device_resources.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));

        if matches!(render_pass_type, KbRenderPassType::Opaque) {
            render_pass.set_pipeline(&self.opaque_render_pipeline);
        } else {
            render_pass.set_pipeline(&self.transparent_render_pipeline);
        }

        self.uniform.screen_dimensions = [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0];//[self.game_config.window_width as f32, self.game_config.window_height as f32, (self.game_config.window_height as f32) / (self.game_config.window_width as f32), 0.0]));
        self.uniform.time[0] = game_config.start_time.elapsed().as_secs_f32();

        #[cfg(target_arch = "wasm32")]
        {
            self.uniform.time[1] = 1.0 / 2.2;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.uniform.time[1] = 1.0;
        }

        render_pass.set_bind_group(0, &self.tex_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..6, 0, 0..frame_instances.len() as _);
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}

pub struct KbPostprocessPipeline {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub postprocess_pipeline: wgpu::RenderPipeline,
    pub postprocess_uniform: PostProcessUniform,
    pub postprocess_constant_buffer: wgpu::Buffer,
    pub postprocess_uniform_bind_group: wgpu::BindGroup,
    pub postprocess_bind_group: wgpu::BindGroup,
}

impl KbPostprocessPipeline {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, surface_config: &wgpu::SurfaceConfiguration, render_texture: &KbTexture) -> Self {
                        
        // Post Process Pipeline
        let postprocess_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("postprocess_uber.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/postprocess_uber.wgsl").into()),
        });
        
        let postprocess_uniform = PostProcessUniform {
            ..Default::default()
        };

        let postprocess_constant_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("postprocess_constant_buffer"),
                contents: bytemuck::cast_slice(&[postprocess_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let postprocess_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("postprocess_uniform_bind_group_layout"),
        });


        let postprocess_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &postprocess_uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: postprocess_constant_buffer.as_entire_binding(),
                }
            ],
            label: Some("postprocess_bind_group"),
        });

        let postprocess_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("postprocess_bind_group_layout"),
        });

        let postprocess_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("postprocess_pipeline_layout"),
            bind_group_layouts: &[&postprocess_bind_group_layout, &postprocess_uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let postprocess_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("postprocess_pipeline"),
            layout: Some(&postprocess_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &postprocess_shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc(), KbDrawInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &postprocess_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
        let texture_bytes = include_bytes!("../game_assets/PostProcessFilter.png");
        let postprocess_texture = KbTexture::from_bytes(&device, &queue, texture_bytes, "PostProcessFilter.png").unwrap();
        let postprocess_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &postprocess_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&postprocess_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&postprocess_texture.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&render_texture.view),
                    },
                ],
                label: Some("postprocess_bind_group"),
            }
        );
 
        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST
            }
        );
        KbPostprocessPipeline {
            postprocess_pipeline,
            postprocess_uniform,
            postprocess_constant_buffer,
            postprocess_uniform_bind_group,
            postprocess_bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn render(&mut self, target_view: &wgpu::TextureView, device_resources: &mut KbDeviceResources, game_config: &KbConfig) {
		let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbPostprocessPipeline::render()"),
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
            label: Some("postprocess_render_pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&self.postprocess_pipeline);
        render_pass.set_bind_group(0, &self.postprocess_bind_group, &[]);
        render_pass.set_bind_group(1, &self.postprocess_uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, device_resources.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        self.postprocess_uniform.time_mode_unused_unused[0] = game_config.start_time.elapsed().as_secs_f32();
        self.postprocess_uniform.time_mode_unused_unused[1] = {
            match game_config.postprocess_mode {
                KbPostProcessMode::Desaturation => { 1.0 }
                KbPostProcessMode::ScanLines => { 2.0 }
                KbPostProcessMode::Warp => { 3.0 }
                _ => { 0.0 }
            }
        };

        device_resources.queue.write_buffer(&self.postprocess_constant_buffer, 0, bytemuck::cast_slice(&[self.postprocess_uniform]));
        render_pass.draw_indexed(0..6, 0, 0..1);
        drop(render_pass);

        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct KbModelUniform {
    pub view_proj: [[f32; 4]; 4],
    pub screen_dimensions: [f32; 4],
    pub time: [f32; 4],
}

pub struct KbModel {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32
}

impl KbModel {
    pub fn new(device: &wgpu::Device) -> Self {

    let extent = 1.25;
        let vertices = &[
            // Front
            KbVertex { position: [extent, extent, extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [-extent, extent, extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [-extent, -extent, extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [extent, -extent, extent], tex_coords: [1.0, 1.0], },

            // Top
            KbVertex { position: [extent, extent, extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [extent, extent, -extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [-extent, extent, -extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [-extent, extent, extent], tex_coords: [1.0, 1.0], },

            // Left
            KbVertex { position: [extent, extent, extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [extent, -extent, extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [extent, -extent, -extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [extent, extent, -extent], tex_coords: [1.0, 1.0], },

            // Right
            KbVertex { position: [-extent, extent, extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [-extent, -extent, extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [-extent, -extent, -extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [-extent, extent, -extent], tex_coords: [1.0, 1.0], },

            // Back
            KbVertex { position: [extent, extent, -extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [-extent, extent, -extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [-extent, -extent, -extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [extent, -extent, -extent], tex_coords: [1.0, 1.0], },

            // Bottom
            KbVertex { position: [extent, -extent, extent], tex_coords: [1.0, 0.0], },
            KbVertex { position: [extent, -extent, -extent], tex_coords: [0.0, 0.0], },
            KbVertex { position: [-extent, -extent, -extent], tex_coords: [0.0, 1.0], },
            KbVertex { position: [-extent, -extent, extent], tex_coords: [1.0, 1.0], },
        ];

        let indices: [u16;36] = [
            0, 1, 3,
            3, 1, 2,

            4, 5, 7,
            7, 5, 6,

            9, 11, 8,
            9, 10, 11,

            12, 15, 13,
            15, 14, 13,

            16, 19, 17,
            19, 18, 17,

            20, 23, 21,
            23, 22, 21,
        ];      

        let num_indices = indices.len() as u32;

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("KbModel_vertex_buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&indices),
                usage: wgpu::BufferUsages::INDEX
            }
        );
        KbModel {
            vertex_buffer,
            index_buffer,
            num_indices
        }
    }
}

pub struct KbModelPipeline {
    pub opaque_render_pipeline: wgpu::RenderPipeline,
    pub uniform: KbModelUniform,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub tex_bind_group: wgpu::BindGroup,
}

impl KbModelPipeline {
    pub fn new(device: &Device, queue: &Queue, surface_config: &SurfaceConfiguration) -> Self {
        log!("Creating KbModelPipeline...");

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
            label: Some("KbModelPipeline_texture_bind_group_layout"),
        });
       
        let texture_bytes = include_bytes!("../game_assets/SpriteSheet.png");
        let sprite_sheet_texture = KbTexture::from_bytes(&device, &queue, texture_bytes, "SpriteSheet.png").unwrap();

        let tex_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&sprite_sheet_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sprite_sheet_texture.sampler),
                    },
                ],
                label: Some("KbModelPipeline: tex_bind_group"),
            }
        );

        log!("  Creating shader");

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Model.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../game_assets/Model.wgsl").into()),
        });
        
        // Uniform buffer
        let uniform = KbModelUniform {
            ..Default::default()
        };

        let uniform_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("KbModelPipeline_uniform_buffer"),
                contents: bytemuck::cast_slice(&[uniform]),
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
            label: Some("KbModelPipeline_uniform_bind_group_layout"),
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &uniform_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                }
            ],
            label: Some("KbModelPipeline_uniform_bind_group"),
        });

        log!("  Creating pipeline");

        // Render pipeline
        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("KbModelPipeline_render_pipeline_layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let opaque_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("KbModelPipeline_opaque_render_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[KbVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
                depth_compare: wgpu::CompareFunction::Always,
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

        KbModelPipeline {
            opaque_render_pipeline,
            uniform,
            uniform_buffer,
            uniform_bind_group,
            tex_bind_group,
        }
    }

    pub fn render(&mut self, render_pass_type: KbRenderPassType, should_clear: bool, model: &KbModel, device_resources: &mut KbDeviceResources, game_config: &KbConfig) {
    	let mut command_encoder = device_resources.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("KbSpritePipeline::render()"),
		});
      
        let color_attachment = {
            if should_clear {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.01,
                            b: 0.35,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })
            } else {
                Some(wgpu::RenderPassColorAttachment {
                    view: &device_resources.render_textures[0].view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })
            }
        };


        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[color_attachment],
            depth_stencil_attachment:  Some(wgpu::RenderPassDepthStencilAttachment {
                view: &device_resources.render_textures[1].view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });
        device_resources.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[self.uniform]));
        //device_resources.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(frame_instances.as_slice()));

       // if matches!(render_pass_type, KbRenderPassType::Opaque) {
            render_pass.set_pipeline(&self.opaque_render_pipeline);
       // } else {
       //     render_pass.set_pipeline(&self.transparent_render_pipeline);
       // }
       let eye: cgmath::Point3<f32> = (0.0, -1.8, 5.0).into();
       let target: cgmath::Point3<f32> = (0.0, 0.0, -100.0).into();
       let up = cgmath::Vector3::unit_y();
       let radians = cgmath::Rad::from(cgmath::Deg(game_config.start_time.elapsed().as_secs_f32() * 15.0));
       let world = cgmath::Matrix4::from_angle_y(radians);
       let view = cgmath::Matrix4::look_at_rh(eye, target, up);
       let proj = cgmath::perspective(cgmath::Deg(75.0), 1920.0 / 1080.0, 0.1, 1000000.0);

        self.uniform.view_proj = (proj * view * world).into();
        self.uniform.screen_dimensions = [game_config.window_width as f32, game_config.window_height as f32, (game_config.window_height as f32) / (game_config.window_width as f32), 0.0];//[self.game_config.window_width as f32, self.game_config.window_height as f32, (self.game_config.window_height as f32) / (self.game_config.window_width as f32), 0.0]));
        self.uniform.time[0] = game_config.start_time.elapsed().as_secs_f32();

        #[cfg(target_arch = "wasm32")]
        {
            self.uniform.time[1] = 1.0 / 2.2;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.uniform.time[1] = 1.0;
        }

        render_pass.set_bind_group(0, &self.tex_bind_group, &[]);
        render_pass.set_bind_group(1, &self.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        render_pass.set_index_buffer(model.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..model.num_indices, 0, 0..1);
        drop(render_pass);
        device_resources.queue.submit(std::iter::once(command_encoder.finish()));
    }

}