use std::error::Error;
use std::iter;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use wgpu::{
	Backends, BlendState, BufferAddress, BufferUsages, Color, ColorTargetState, ColorWrites,
	CommandEncoderDescriptor, CompositeAlphaMode, Device, DeviceDescriptor, Face, Features,
	FragmentState, FrontFace, include_wgsl, Instance, Limits, LoadOp, MultisampleState, Operations,
	PipelineLayoutDescriptor, PolygonMode, PowerPreference, PresentMode, PrimitiveState,
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
	color: [f32; 3],
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
				},
				VertexAttribute {
					offset: size_of::<[f32; 2]>() as BufferAddress,
					shader_location: 1,
					format: VertexFormat::Float32x3,
				}
			],
		}
	}
}

const VERTICES: &[Vertex] = &[
	Vertex { position: [0.0, 0.5], color: [1.0, 0.0, 0.0] },
	Vertex { position: [-0.5, -0.5], color: [0.0, 1.0, 0.0] },
	Vertex { position: [0.5, -0.5], color: [0.0, 0.0, 1.0] },
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

	event_loop.run(move |event, _, control_flow| match event {
		Event::RedrawRequested(window_id) if window_id == window.id() => {
			let output = surface.get_current_texture().unwrap();
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
				render_pass.draw(0..VERTICES.len() as u32, 0..1);
			}

			queue.submit(iter::once(encoder.finish()));
			output.present();
		}
		Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
			WindowEvent::Resized(new_size) => configure(&mut config, *new_size, &surface, &device),
			WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
			_ => {}
		}
		_ => {}
	});
}
