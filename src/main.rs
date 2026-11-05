use core::error;
use egui::Key::P;
use egui::frame;
use std::sync::Arc;
use wgpu::{DisplayAndWindowHandle, MemoryBudgetThresholds, RenderPassDescriptor, *};
// use wgpu_core::error;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

mod texture;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
}
// we put the verticies in counter-clockwise both due to tradition and because we want access to backface culling.
const VERTICIES: &[Vertex] = &[
    // Triangle

    // Vertex { position: [ 0.0,  0.5,  0.0],    color: [1.0,0.0,0.0]},
    // Vertex { position: [-0.5, -0.5,  0.0],    color: [0.0,1.0,0.0]},
    // Vertex { position: [ 0.5, -0.5,  0.0],    color: [0.0,0.0,1.0]},

    // rgb_color = ((srgb_color / 255 + 0.055) / 1.055) ^ 2.4 is the forumula to get the proper colors out of sRGB since it chooses to fuck them up due to the way humans percieve colors.

    // Pentagon with upside down textures
    // Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.99240386], }, // A
    // Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.56958647], }, // B
    // Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.05060294], }, // C
    // Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.1526709], }, // D
    // Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.7347359], }, // E

    // Pentagon with right-side-up textures
    // all of them seem to be 1- the previous number give or take. which makes sense
    Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A 
    Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E

    //hexagon
    // Vertex {
    //     position: [0.0, 0.5, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
    // Vertex {
    //     position: [-0.25, 0.25, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
    // Vertex {
    //     position: [-0.25, -0.25, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
    // Vertex {
    //     position: [0.0, -0.5, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
    // Vertex {
    //     position: [0.25, -0.25, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
    // Vertex {
    //     position: [0.25, 0.25, 0.0],
    //     color: [0.5, 0.0, 0.5],
    // },
];

const INDICIES: &[u16] = &[
    // pentagon
    0,1,4,
    1,2,4,
    2,3,4,

    // hexagon
    // 0, 1, 5, 
    // 1, 2, 5,
    // 2, 3, 5,
    // 3, 4, 5,

    //NOTE TO SELF: don't forget to switch the index array because it will NOT throw a goddamn error for some reason and will just used the fucked up indicies. 
];

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2, // Changed from Float32x3 since we're no longer using color coordinates directly
                },
            ],
        }
    }
}

struct State {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>,

    device: wgpu::Device,

    queue: wgpu::Queue,

    config: wgpu::SurfaceConfiguration,

    size: winit::dpi::PhysicalSize<u32>,

    is_surface_configured: bool, //TODO: figure out where the hell this is meant to go 

    render_pipleline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,

    num_verticies: u32,

    index_buffer: wgpu::Buffer,

    num_indicies: u32,

    diffuse_bind_group: wgpu::BindGroup,

    diffuse_texture: texture::Texture,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICIES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICIES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let num_indicies = INDICIES.len() as u32;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let diffuse_bytes = include_bytes!("AngryDog.png"); // our texture
        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "AngryDog.png").unwrap();//image::load_from_memory(diffuse_bytes).unwrap();
        // let diffuse_rgba = diffuse_image.to_rgba8();

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
                ],
                label: Some("texture_bind_group_layout"),
            });
        
        // we can hotswap bind groups on the fly so long as they share the same bind layout
        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1, 
                        resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );


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

        // let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        //     label: Some("Pipeline Layout"),
        //     bind_group_layouts: &[],
        //     immediate_size: 0,
        //     // push_constant_ranges: &[], // TODO: figure out what this even is and why it was included in the previous tutorial
        // });

        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[Some(&texture_bind_group_layout)],
                immediate_size: 0,
            }
        );

        let render_pipleline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                buffers: &[
                    Some(Vertex::desc()),
                    //None, // also valid: // Some(Instance::desc()) // where you actually create the function desc() for all of them of course
                    // An index buffer is used in junction with a vertex buffer to weed out the duplicate verticies that are within a model.
                    //Some(Material::desc()).
                ],
                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
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
                count: 1, // means there's not going to be antialiasing happening during rendering.
                mask: !0, // !0 means all bits set.
                alpha_to_coverage_enabled: false,
            },

            multiview_mask: None,
            cache: None,
        });
        let num_verticies = VERTICIES.len() as u32;

        State {
            surface,
            device,
            queue,
            config,
            size,
            is_surface_configured: false,
            window,
            render_pipleline,
            vertex_buffer,
            num_verticies,
            index_buffer,
            num_indicies,
            diffuse_bind_group,
            diffuse_texture,
        }
    }

    fn window(&self) -> &Window {
        &self.window
    }
    // we must ensure that we recheck the size of the window when we change the size or minimize, keeping in mind that values at or below 0 are invalid and will immediately crash the system.
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // configuring a window with 0 dimension crashes
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self, output: wgpu::SurfaceTexture) {
        let view: wgpu::TextureView = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
                // Couldn't tell you wtf this does
                timestamp_writes: None,
                // Not needed yet due since they're for gpu profiling
                occlusion_query_set: None,
                // Allows for layered effects using masking
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipleline); // The render pipeline must come before the index buffer sine those two are tied together. But the vertex buffer don't care either way.
            render_pass.set_bind_group(0, &self.diffuse_bind_group,&[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // .. sepcifies the entire buffer
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indicies, 0, 0..1); //assuming  that the one gets increased when we go into instantiation or however you spell ts.
            // println!("self.num_verticies: {}", self.num_verticies);
            render_pass.draw(0..self.num_verticies, 0..1); // this used to be 3 that's why it was only drawing the 3 triangles despite me sending more type shi!!!!
        }
        
        self.queue.submit(std::iter::once(encoder.finish())); // submit takes an iterator because you typically find several command buffer commands in the encoder finish
        self.queue.present(output); // puts the frame on screen, a SurfaceTexture dropped without this gets thrown away
    }
}

// State can't exist before its window, and the window now has to be created from inside the
// running loop, so the handler holds an Option that gets filled in resumed.
#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Shader Playground")
            .with_inner_size(winit::dpi::LogicalSize::new(960.0, 540.0));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        // ponytail: desktop only calls resumed once; Android can call it again, guard with self.state.is_some() if you ever target it
        self.state = Some(pollster::block_on(State::new(window)));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        if window_id == state.window.id() {
            match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(phsyical_size) => state.resize(phsyical_size),
                WindowEvent::RedrawRequested => {
                    state.window().request_redraw();
                    match state.surface.get_current_texture() {
                        wgpu::CurrentSurfaceTexture::Success(frame) => state.render(frame),
                        wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                            state.render(frame);
                            // I'm assuming right now that this is the thing that gets called whenever you resize a frame,
                            state.resize(state.size);
                        }
                        wgpu::CurrentSurfaceTexture::Outdated => state.resize(state.size),
                        wgpu::CurrentSurfaceTexture::Lost => event_loop.exit(),
                        other => eprintln!("{:?}", other),
                    }
                }
                _ => {}
            }
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::default()).unwrap();
}

// // use bevy::camera::ScalingMode;
// // use bevy::prelude::*;

// fn main() {
//     // wgpus logging has been enabled here
//     env_logger::init();

//     //Finds the graphics api that's available on this pc
//     let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
//         backends: wgpu::Backends::all(),
//         ..Default::default()
//     });

//     let adapter = pollster::block_on(instance.request_adapter(
//         &wgpu::RequestAdapterOptions{
//             power_preference: wgpu::PowerPreference::HighPerformance, // this tells our program that we want the adapter with the highest performace, uncaring for battery life and power usage.
//             compatible_surface: None, // the place where we draw the images to the screen.
//             force_fallback_adapter: false, // setting this to true would force a software renderer, which isn't required when we have a gpu.
//         }
//     )). expect("No suitable GPU found on this machine");

//     println!("Chose:\t\t{}", adapter.get_info().name);   // println!("-- GPUs found on this machine --");
//     // for adapter in instance.enumerate_adapters(wgpu::Backends::all()) {
//     //     let info = adapter.get_info();

//     //     println!("Name:\t\t{}", info.name);
//     //     println!("Backend:\t\t{:?}", info.backend);
//     //     println!("Type:\t\t{:?}", info.device_type);
//     //     println!("Driver:\t\t{} {}", info.driver, info.driver_info);

//     //     let limits = adapter.limits();
//     //     println!("Max texture size:\t\t{}", limits.max_texture_dimension_2d);
//     //     println!("Max buffer size:\t\t{}", limits.max_buffer_size);
//     //     // the base amount you're allowed to use across all platforms is 4, anything beyond that and you're going to run into porting issues.
//     //     println!("Max bind groups:\t\t{}", limits.max_bind_groups);
//     //     println!("Max vertex attributes:\t\t{}", limits.max_vertex_attributes);
//     //     // The limit to the amount of threads a compute shader can use at once, pretty much limiting the amount of parallel work a compute shader can do
//     //     println!("Max compute groups:\t\t{}", limits.max_compute_workgroup_size_x);

//     //     println!();
//     // }
//     // App::new()
//     //     .add_plugins(DefaultPlugins)
//     //     .add_systems(Startup, setup)
//     //     // .add_systems(Update, environment_map_load_finish)
//     //     .run();
// }

// // set up a simple 3D scene
// // fn setup(
// //     mut commands: Commands,
// //     mut meshes: ResMut<Assets<Mesh>>,
// //     mut materials: ResMut<Assets<StandardMaterial>>,
// //     // asset_server: Res<AssetServer>,
// // ) {
// //     let sphere_mesh = meshes.add(Sphere::new(0.45));
// //     // add entities to the world
// //     for y in -2..=2 {
// //         for x in -5..=5 {
// //             let x01 = (x + 5) as f32 / 10.0;
// //             let y01 = (y + 2) as f32 / 4.0;
// //             // sphere
// //             commands.spawn((
// //                 Mesh3d(sphere_mesh.clone()),
// //                 MeshMaterial3d(materials.add(StandardMaterial {
// //                     base_color: Srgba::hex("#ffd891").unwrap().into(),
// //                     // vary key PBR parameters on a grid of spheres to show the effect
// //                     metallic: y01,
// //                     perceptual_roughness: x01,
// //                     ..default()
// //                 })),
// //                 Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
// //             ));
// //         }
// //     }
// //     // unlit sphere
// //     commands.spawn((
// //         Mesh3d(sphere_mesh),
// //         MeshMaterial3d(materials.add(StandardMaterial {
// //             base_color: Srgba::hex("#ffd891").unwrap().into(),
// //             // vary key PBR parameters on a grid of spheres to show the effect
// //             unlit: true,
// //             ..default()
// //         })),
// //         Transform::from_xyz(-5.0, -2.5, 0.0),
// //     ));

// //     commands.spawn((
// //         DirectionalLight {
// //             illuminance: 1_500.,
// //             ..default()
// //         },
// //         Transform::from_xyz(50.0, 50.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
// //     ));

// //     // labels
// //     commands.spawn((
// //         Text::new("Perceptual Roughness"),
// //         TextFont {
// //             font_size: FontSize::Px(30.0),
// //             ..default()
// //         },
// //         Node {
// //             position_type: PositionType::Absolute,
// //             top: px(20),
// //             left: px(100),
// //             ..default()
// //         },
// //     ));

// //     commands.spawn((
// //         Text::new("Metallic"),
// //         TextFont {
// //             font_size: FontSize::Px(30.0),
// //             ..default()
// //         },
// //         Node {
// //             position_type: PositionType::Absolute,
// //             top: px(130),
// //             right: Val::ZERO,
// //             ..default()
// //         },
// //         UiTransform {
// //             rotation: Rot2::degrees(90.),
// //             ..default()
// //         },
// //     ));

// //     // commands.spawn((
// //     //     Text::new("Loading Environment Map..."),
// //     //     TextFont {
// //     //         font_size: FontSize::Px(30.0),
// //     //         ..default()
// //     //     },
// //     //     Node {
// //     //         position_type: PositionType::Absolute,
// //     //         bottom: px(20),
// //     //         right: px(20),
// //     //         ..default()
// //     //     },
// //     //     EnvironmentMapLabel,
// //     // ));

// //     // camera
// //     commands.spawn((
// //         Camera3d::default(),
// //         Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
// //         Projection::from(OrthographicProjection {
// //             scale: 0.01,
// //             scaling_mode: ScalingMode::WindowSize,
// //             ..OrthographicProjection::default_3d()
// //         }),
// //         // EnvironmentMapLight {
// //         //     diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
// //         //     specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
// //         //     intensity: 900.0,
// //         //     ..default()
// //         // },
// //     ));
// // }

// // fn environment_map_load_finish(
// //     mut commands: Commands,
// //     asset_server: Res<AssetServer>,
// //     environment_map: Single<&EnvironmentMapLight>,
// //     label_entity: Option<Single<Entity, With<EnvironmentMapLabel>>>,
// // ) {
// //     if asset_server
// //         .load_state(&environment_map.diffuse_map)
// //         .is_loaded()
// //         && asset_server
// //             .load_state(&environment_map.specular_map)
// //             .is_loaded()
// //     {
// //         // Do not attempt to remove `label_entity` if it has already been removed.
// //         if let Some(label_entity) = label_entity {
// //             commands.entity(*label_entity).despawn();
// //         }
// //     }
// // }

// // #[derive(Component)]
// // struct EnvironmentMapLabel;
