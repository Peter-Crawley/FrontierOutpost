use std::error::Error;
use std::iter;
use std::mem::size_of;
use std::time::{Duration, Instant};

use bytemuck::{Pod, Zeroable};
use wgpu::{
	Backends, BlendState, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
	CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Face, Features,
	FragmentState, FrontFace, include_wgsl, IndexFormat, Instance, Limits, LoadOp, MultisampleState,
	Operations, PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode, PrimitiveState,
	PrimitiveTopology, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
	RequestAdapterOptions, Surface, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
	VertexAttribute, VertexBufferLayout, VertexFormat, VertexState, VertexStepMode,
};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
	position: [f32; 2],
}

impl Vertex {
	fn descriptor<'a>() -> VertexBufferLayout<'a> {
		VertexBufferLayout {
			array_stride: size_of::<Vertex>() as BufferAddress,
			step_mode: VertexStepMode::Vertex,
			attributes: &[
				VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: VertexFormat::Float32x2,
				}
			],
		}
	}
}

const VERTICES: &[Vertex] = &[
	Vertex { position: [-0.5, -0.5] },
	Vertex { position: [0.5, -0.5] },
	Vertex { position: [0.5, 0.5] },
	Vertex { position: [-0.5, 0.5] },
];

const INDICES: &[u16] = &[
	0, 1, 2,
	0, 2, 3,
];

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Frontier Outpost")
		.build(&event_loop)?;

	let instance = Instance::new(Backends::VULKAN);
	let surface = unsafe { instance.create_surface(&window) };

	let adapter = instance.request_adapter(&RequestAdapterOptions {
		power_preference: PowerPreference::HighPerformance,
		compatible_surface: Some(&surface),
		force_fallback_adapter: false,
	}).await.unwrap();

	let (device, queue) = adapter.request_device(&DeviceDescriptor {
		features: Features::empty(),
		limits: Limits::default(),
		label: None,
	}, None).await?;

	let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
		label: None,
		contents: bytemuck::cast_slice(VERTICES),
		usage: BufferUsages::VERTEX,
	});

	let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
		label: None,
		contents: bytemuck::cast_slice(INDICES),
		usage: BufferUsages::INDEX,
	});

	let mut config = SurfaceConfiguration {
		usage: TextureUsages::RENDER_ATTACHMENT,
		format: surface.get_supported_formats(&adapter)[0],
		width: 0,
		height: 0,
		present_mode: PresentMode::AutoVsync,
		alpha_mode: CompositeAlphaMode::Auto,
	};

	let configure = move |
		config: &mut SurfaceConfiguration,
	    new_size: PhysicalSize<u32>,
	    surface: &Surface,
	    device: &Device
	| {
		config.width = new_size.width;
		config.height = new_size.height;
		surface.configure(device, &config);
	};

	configure(&mut config, window.inner_size(), &surface, &device);

	let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

	let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
		label: None,
		bind_group_layouts: &[],
		push_constant_ranges: &[],
	});

	let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
		label: None,
		layout: Some(&render_pipeline_layout),
		vertex: VertexState {
			module: &shader,
			entry_point: "vertex_main",
			buffers: &[Vertex::descriptor()],
		},
		fragment: Some(FragmentState {
			module: &shader,
			entry_point: "fragment_main",
			targets: &[Some(ColorTargetState {
				format: config.format,
				blend: Some(BlendState::REPLACE),
				write_mask: ColorWrites::ALL,
			})],
		}),
		primitive: PrimitiveState {
			topology: PrimitiveTopology::TriangleList,
			strip_index_format: None,
			front_face: FrontFace::Ccw,
			cull_mode: Some(Face::Back),
			polygon_mode: PolygonMode::Fill,
			unclipped_depth: false,
			conservative: false,
		},
		depth_stencil: None,
		multisample: MultisampleState {
			count: 1,
			mask: !0,
			alpha_to_coverage_enabled: false,
		},
		multiview: None,
	});

	let mut frame_time = Duration::ZERO;
	let mut last_time = Instant::now();
	let mut frames = 0;

	event_loop.run(move |event, _, control_flow| match event {
		Event::MainEventsCleared => window.request_redraw(),
		Event::RedrawRequested(window_id) if window_id == window.id() => {
			let output = surface.get_current_texture().unwrap();

			frames += 1;
			if last_time.elapsed() >= Duration::from_secs(1) {
				println!("{} FPS {:.2}ms Avg", frames, frame_time.as_millis() as f64 / frames as f64);
				frame_time = Duration::ZERO;
				last_time = Instant::now();
				frames = 0;
			}

			let frame_start_time = Instant::now();

			let view = output.texture.create_view(&TextureViewDescriptor::default());
			let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

			{
				let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
					label: None,
					color_attachments: &[Some(RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Clear(Color::BLACK),
							store: true,
						},
					})],
					depth_stencil_attachment: None,
				});

				render_pass.set_pipeline(&render_pipeline);
				render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
				render_pass.set_index_buffer(index_buffer.slice(..), IndexFormat::Uint16);
				render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
			}

			queue.submit(iter::once(encoder.finish()));
			output.present();

			frame_time += frame_start_time.elapsed();
		}
		Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
			WindowEvent::Resized(new_size) => configure(&mut config, *new_size, &surface, &device),
			WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
			_ => {}
		}
		_ => {}
	});
}
