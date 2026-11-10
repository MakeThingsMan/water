
use std::sync::Arc;
use web_sys::js_sys::Intl::SegmenterGranularity::Sentence;
use winit::event_loop::{ActiveEventLoop,};
use winit::keyboard::{KeyCode,};
use winit::window::{Window};
use wgpu::{MemoryBudgetThresholds};
// use wgpu_core::error;
use wgpu::util::DeviceExt;
use cgmath::prelude::*;

use crate::helpers::{create_render_pipeline};

use crate::resources as resources;
use crate::texture as texture;
use crate::model as model;
use crate::model::{DrawModel,Vertex, Model};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0,0.0,0.0,0.0),
    cgmath::Vector4::new(0.0,1.0,0.0,0.0),
    cgmath::Vector4::new(0.0,0.0,0.5,0.0),
    cgmath::Vector4::new(0.0,0.0,0.5,1.0), //interesting...
);

struct Instance {
    position: cgmath::Vector3<f32>,
    rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    fn to_raw(&self) -> InstanceRaw {
        let model = (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into();
        let normal = cgmath::Matrix3::from(self.rotation).into();
        InstanceRaw { 
            model: model,
            normal: normal,
        }
    }
}

#[repr(C)]
#[derive(Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
#[allow(dead_code)] //TODO: figure out what this does. I'm assuming it's like typescripts ignore command where it turns off compiler errors for certain things. 
struct InstanceRaw {
    model: [[f32;4];4],
    normal: [[f32;3];3],
}

impl model::Vertex for InstanceRaw {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem; 
        wgpu::VertexBufferLayout{ 
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            
            // We need to switch from using a step mode of Vertex to Instance
            // This means that our shaders will only change to use the next instance when the shader starts processing a new instance
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    // While our vertex shader only uses locations 0, and 1 now, in later tutorials, we'll
                    // be using 2, 3, and 4 for Vertex. We'll start at slot 5 to not conflict with them later
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
                // for each vec4. We don't have to do this in code, though.
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // NEW!
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 10;
const SPACE_BETWEEN: f32 = 3.0;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)] // Things are aligned to the closest power of 2, meaning 12 bytes gets padded up to 16. Be careful.
struct LightUniform {
    position: [f32; 3],
    // Due to uniforms requireing 16 byte (4 float) spacing, we need to use a padding field here
    _padding: u32,
    color: [f32; 3],
    // Due to uniforms requireing 16 byte (4 float) spacing, we need to use a padding field here
    _padding2:u32, 
}
// We need this for Rust to store our data correctly for the shaders 
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug,Copy,Clone,bytemuck::Pod,bytemuck::Zeroable)]
struct CameraUniform {
    //We can't use cgmath with bytemuck directly, so we'll have to convert the Matrix44 into a 4x4 f32 array
    view_position: [f32; 4],
    view_proj: [[f32; 4];4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_position: [0.0;4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        // We're using Vector4 because of the uniforms 16 byte spacing requirement
        self.view_position = camera.eye.to_homogeneous().into();
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}
struct Camera {
    eye: cgmath::Point3<f32>,
    target: cgmath::Point3<f32>,
    up: cgmath::Vector3<f32>,
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera { 
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1. 
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2. 
        let proj = cgmath::perspective(cgmath::Deg(self.fovy),self.aspect,self.znear,self.zfar);

        // 3. 
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so 
            // that it doesn't change. The eye, therefore, still 
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}
pub struct State {
    pub  window: Arc<Window>,

    pub surface: wgpu::Surface<'static>,

    pub device: wgpu::Device,

    pub queue: wgpu::Queue,

    pub config: wgpu::SurfaceConfiguration,

    pub size: winit::dpi::PhysicalSize<u32>,

    is_surface_configured: bool, //TODO: figure out where the hell this is meant to go 

    render_pipleline: wgpu::RenderPipeline,

    obj_model: model::Model,

    camera: Camera,

    camera_uniform: CameraUniform,

    camera_buffer: wgpu::Buffer,

    camera_bind_group: wgpu::BindGroup,

    camera_controller: CameraController,
    
    instances: Vec<Instance>,

    instance_buffer: wgpu::Buffer,

    depth_texture: texture::Texture,

    screen_texture: texture::Texture, 

    light_uniform: LightUniform,

    light_buffer: wgpu::Buffer,

    light_bind_group: wgpu::BindGroup,

    light_render_pipeline: wgpu::RenderPipeline,

    filter_bind_group: wgpu::BindGroup,

    filter_bind_group_layout: wgpu::BindGroupLayout,

    filter_render_pipeline: wgpu::RenderPipeline,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: Default::default(),
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
            (0..NUM_INSTANCES_PER_ROW).map(move |x| { //TODO: figure out what 'move' does because it removed an error 
                let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                let position = cgmath::Vector3 {x, y: 0.0, z};

                let rotation = if position.is_zero() {
                    // this is needed so an object at (0,0,0) won't get scaled to zero as quaternions can affect scale if they're not created correctly
                    cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                } else {
                    cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                };

                Instance {
                    position, rotation,
                }
            })
        }).collect::<Vec<_>>(); // TODO: Go learn how to use flat_map, map, and collect please.

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        let instance_buffer = device.create_buffer_init( 
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,  
            }
        );

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo, //Caps the frames to be at the refresh rate of the monitor, it's the one mode that guarenteed to exist everywhere.
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2, // how many frames the gpu may work on before it has to wait for one to be displayed. Defaults to two.
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        
        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        let screen_texture = texture::Texture::create_screen_texture(&device, &config, "screen_texture");

        let light_uniform= LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0,1.0,1.0],
            _padding2: 0,
        };

        // We'll want to update our lights position, so we use COPY_DST
        let light_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Light VB"),
                contents: bytemuck::cast_slice(&[light_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            entries: &[wgpu::BindGroupLayoutEntry{
                binding: 0, 
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false, 
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("light_bind_group_layout"),
        });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry{
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: Some("light_bind_group"),
        });

        let texture_bind_group_layout = 
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, 
                        visibility: wgpu::ShaderStages::FRAGMENT, 
                        ty: wgpu::BindingType::Texture {
                            multisampled: false, // I'm assuming I can turn this on in bevy
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true},
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, 
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the corresponding Texture entry above. 
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                     wgpu::BindGroupLayoutEntry {
                        binding: 2, 
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the corresponding Texture entry above. 
                        ty: wgpu::BindingType::Texture{
                            multisampled: false, 
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                     wgpu::BindGroupLayoutEntry {
                        binding: 3, 
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the corresponding Texture entry above. 
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
              
        let filter_bind_group_layout = device.create_bind_group_layout(
                &wgpu::BindGroupLayoutDescriptor{
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false, 
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
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
                    label: Some("filter_bind_group_layout"),
                }
            );

        let filter_bind_group = device.create_bind_group( &wgpu::BindGroupDescriptor {
            layout:  &filter_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&screen_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&screen_texture.sampler),
            },
            ],
            label: Some("filter_bind_group"),
        });

        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0,1.0,2.0).into(),
            // have it look at the origin
            target: (0.0,0.0,0.0).into(),
            // which way is "up"
            up:cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32, 
            fovy: 45.0,
            znear: 0.1, 
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        let camera_controller = CameraController::new(0.2);

        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );


        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT, // I can see some cool effects related to camera position being put into the fragment shader. <-- You were right past me!
                    // Perhaps that's how people do that chromatic abberation effect?
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false, // tells the gpu that there are data sets of varrying sizes within this one buffer, we'll have to give it offsets later.
                        min_binding_size: None, // Smallest size the buffer can be
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // we can grab the errors that are thrown out by the compilation here
        let info = shader.get_compilation_info().await;
        let errors = info
            .messages
            .iter()
            .filter(|message| message.message_type == wgpu::CompilationMessageType::Error);
        for error in errors {
            eprintln!("{:?}", error.message);
        }

        let obj_model = resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
            .await
            .unwrap();

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group_layout),  
                    Some(&light_bind_group_layout),
                ],
                immediate_size: 0,
            }
        );

        let render_pipleline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
            };

            create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[Some(model::ModelVertex::desc()), Some(InstanceRaw::desc())],
                shader,
            )
        };

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Light Pipeline Layout"),
                    bind_group_layouts: &[Some(&camera_bind_group_layout), Some(&light_bind_group_layout)],
                    immediate_size: 0,
                }
            );
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("light.wgsl").into()),
            };
            create_render_pipeline(
                &device, 
                &layout, 
                config.format, 
                Some(texture::Texture::DEPTH_FORMAT), 
                &[Some(model::ModelVertex::desc())], 
                shader)
        };

        let filter_render_pipeline = {
            let layout = device.create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: Some("Filter Pipleline Layout"),
                    bind_group_layouts: &[Some(&filter_bind_group_layout)],
                    immediate_size: 0,
                }
            );

            let filter_shader = wgpu::ShaderModuleDescriptor{
                label: Some("Filter Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("Kuwahara.wgsl").into()),
            };

            create_render_pipeline(
                &device, 
                &layout, 
                config.format, 
                None, 
                &[], 
                filter_shader)
        };

        State {
            surface,
            device,
            queue,
            config,
            size,
            is_surface_configured: false,
            window,
            render_pipleline,
            obj_model,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            instances,
            instance_buffer,
            depth_texture,
            screen_texture,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            filter_bind_group,
            filter_bind_group_layout,
            filter_render_pipeline,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
    // we must ensure that we recheck the size of the window when we change the size or minimize, keeping in mind that values at or below 0 are invalid and will immediately crash the system.
    // We need to make sure to come back to this function whenever something that's permanently used is created. 
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // configuring a window with 0 dimension crashes
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture"); // needs to be after the config update otherwise the program will crash as the size of the window is different from the texture size. 
            self.screen_texture = texture::Texture::create_screen_texture(&self.device, &self.config, "screen_texture");

            self.filter_bind_group = self.device.create_bind_group( &wgpu::BindGroupDescriptor { // TODO: Potentially swap this out so the code is more readable. Also I hate redundant code!!!
                layout:  &self.filter_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.screen_texture.view), // Technologia!
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.screen_texture.sampler), // BINDING THE NEW RESOURCE! OHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH!
                },
                ],
                label: Some("filter_bind_group"),
            });

            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&mut self, output: wgpu::SurfaceTexture) {
        let view: wgpu::TextureView = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render"),
            });

        {
            // The braces are here to make sure the first render_pass gets thrown out so the borrow checker doesn't throw a fit.
           { let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.screen_texture.view, // We write to our filter texture here. 
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.10,
                            b: 0.25,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                // Couldn't tell you wtf this does
                timestamp_writes: None,
                // Not needed yet due since they're for gpu profiling
                occlusion_query_set: None,
                // Allows for layered effects using masking <-- I think? It kinda looks like the depth_stencil can do the same thing so idk anymore. 
                multiview_mask: None,
            });

            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..)); //When adding new instances to the Vec, we need to recreate the instance_buffer as well as the camera_bind_group. Otherwise the instances won't show up correctly.

            use crate::model::DrawLight; 
            render_pass.set_pipeline(&self.light_render_pipeline);
            render_pass.draw_light_model(
                &self.obj_model, 
                &self.camera_bind_group, 
                &self.light_bind_group
            );
            
            render_pass.set_pipeline(&self.render_pipleline); // The render pipeline must come before the index buffer sine those two are tied together. But the vertex buffer don't care either way.
            render_pass.draw_model_instanced(
                &self.obj_model, 
                0..self.instances.len() as u32, 
                &self.camera_bind_group, 
                &self.light_bind_group
            );

        }

            let mut second_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor{
                label: Some("Filter Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view, // And here we write to our 'screen' 
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.05,
                                g: 0.10,
                                b: 0.25,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                        depth_slice: None,
                    })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            
            second_pass.set_pipeline(&self.filter_render_pipeline);             // Pattern is: Set the pipe, Bind the pipe, Draw the pipe! type shi!!!!
            second_pass.set_bind_group(0, &self.filter_bind_group, &[]);// light and camera don't have to do it because they write to their own buffers and those are just contiguous chunks of memory. They don't leave. 
            second_pass.draw(0..3, 0..1);

        }
        
        self.queue.submit(std::iter::once(encoder.finish())); // submit takes an iterator because you typically find several command buffer commands in the encoder finish
        self.queue.present(output); // puts the frame on screen, a SurfaceTexture dropped without this gets thrown away
    }

    pub fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
        event_loop.exit();
       } else {
        self.camera_controller.handle_key(code, is_pressed);
        self.update();
       }
    }

    pub fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position = (cgmath::Quaternion::from_axis_angle((0.0,1.0,0.0).into(), cgmath::Deg(1.0)) * old_position).into();
        self.queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&[self.light_uniform]));

    }
}

