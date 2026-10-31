use egui::Key::P;
use std::sync::Arc;
use wgpu::RenderPassDescriptor;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

struct State {
    window: Arc<Window>,

    surface: wgpu::Surface<'static>,

    device: wgpu::Device,

    queue: wgpu::Queue,

    config: wgpu::SurfaceConfiguration,

    size: winit::dpi::PhysicalSize<u32>,

    render_pipleline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: Arc<Window>) -> State {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

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
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipleline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Piepline"),
            layout: Some(&pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex",
                buffers: &[],
                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment",
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

            multiview: None,
            cache: None,
        });

        State {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipleline,
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

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        let output = self.surface.get_current_texture()?;

        let view = output
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
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                // Not needed yet due since they're for gpu profiling
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipleline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish())); // submit takes an iterator because you typically find several command buffer commands in the encoder finish

        output.present();

        Ok(())
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    let window = Arc::new(
        WindowBuilder::new()
            .with_title("Shader Playground")
            .build(&event_loop)
            .unwrap(),
    );

    let mut state = pollster::block_on(State::new(window.clone()));

    event_loop
        .run(move |event, control_flow| match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() => match event {
                WindowEvent::CloseRequested => control_flow.exit(),
                WindowEvent::Resized(phsyical_size) => state.resize(*phsyical_size),
                WindowEvent::RedrawRequested => {
                    state.window().request_redraw();
                    match (state).render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            let size = state.size;
                            (state).resize(size);
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            control_flow.exit();
                        }
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            },

            _ => {}
        })
        .unwrap()
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
