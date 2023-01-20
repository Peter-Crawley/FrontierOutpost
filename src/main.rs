use std::error::Error;
use std::iter;

use wgpu::{
	Adapter, Backends, Color, CommandEncoderDescriptor, CompositeAlphaMode, Device,
	DeviceDescriptor, Features, Instance, Limits, LoadOp, Operations, PowerPreference, PresentMode,
	RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, Surface,
	SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();

	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("Frontier Outpost")
		.build(&event_loop)?;

	let size = window.inner_size();

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

	configure(&surface, &adapter, &device, size);

	event_loop.run(move |event, _, control_flow| match event {
		Event::RedrawRequested(window_id) if window_id == window.id() => {
			let output = surface.get_current_texture().unwrap();
			let view = output.texture.create_view(&TextureViewDescriptor::default());
			let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());

			encoder.begin_render_pass(&RenderPassDescriptor {
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

			queue.submit(iter::once(encoder.finish()));
			output.present();
		}
		Event::WindowEvent { ref event, window_id } if window_id == window.id() => match event {
			WindowEvent::Resized(new_size) => configure(&surface, &adapter, &device, *new_size),
			WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
			_ => {}
		}
		_ => {}
	});
}

fn configure(surface: &Surface, adapter: &Adapter, device: &Device, new_size: PhysicalSize<u32>) {
	surface.configure(device, &SurfaceConfiguration {
		usage: TextureUsages::RENDER_ATTACHMENT,
		format: surface.get_supported_formats(&adapter)[0],
		width: new_size.width,
		height: new_size.height,
		present_mode: PresentMode::AutoVsync,
		alpha_mode: CompositeAlphaMode::Auto,
	});
}
