use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::Window;
use wgpu::{DeviceDescriptor, TextureFormat, PresentMode};
use futures::executor::block_on;
use std::borrow::BorrowMut;

struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swap_chain: wgpu::SwapChain,
    swap_chain_desc: wgpu::SwapChainDescriptor,
    size: winit::dpi::PhysicalSize<u32>,
    clear_color: wgpu::Color,
    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::BackendBit::VULKAN);
        let surface = unsafe { instance.create_surface(window) };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::Default,
            compatible_surface: Some(&surface),
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &DeviceDescriptor {
                features: wgpu::Features::default(),
                limits: wgpu::Limits::default(),
                shader_validation: true,
            },
            None,
        ).await.unwrap();


        let swap_chain_desc = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
            format: TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: PresentMode::Fifo,
        };

        let swap_chain = device.create_swap_chain(&surface, &swap_chain_desc);

        let vertex_glsl = include_str!("shader.vert");
        let fragment_glsl = include_str!("shader.frag");

        let mut compiler = shaderc::Compiler::new().unwrap();
        let vs_spirv = compiler.compile_into_spirv(vertex_glsl,
                                                   shaderc::ShaderKind::InferFromSource,
                                                   "shader.vert",
                                                   "main", None).unwrap();
        let fs_spirv = compiler.compile_into_spirv(fragment_glsl,
                                                   shaderc::ShaderKind::InferFromSource,
                                                   "shader.frag",
                                                   "main", None).unwrap();

        let vs_module = device.create_shader_module(wgpu::util::make_spirv(vs_spirv.as_binary_u8()));
        let fs_module = device.create_shader_module(wgpu::util::make_spirv(fs_spirv.as_binary_u8()));

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("a pipeline layout of mine"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("my pipeline"),
            layout: Some(&layout),
            vertex_stage: wgpu::ProgrammableStageDescriptor { module: &vs_module, entry_point: "main" },
            fragment_stage: Some(wgpu::ProgrammableStageDescriptor { module: &fs_module, entry_point: "main" }),
            rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                front_face: Default::default(),
                cull_mode: Default::default(),
                clamp_depth: false,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            }),
            primitive_topology: wgpu::PrimitiveTopology::TriangleList,
            color_states: &[wgpu::ColorStateDescriptor {
                format: swap_chain_desc.format,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            }],
            depth_stencil_state: None,
            vertex_state: wgpu::VertexStateDescriptor { index_format: wgpu::IndexFormat::Uint16, vertex_buffers: &[] },
            sample_count: 1,
            sample_mask: !0,
            alpha_to_coverage_enabled: false,
        });

        Self {
            surface,
            device,
            queue,
            swap_chain,
            swap_chain_desc,
            size,
            clear_color: Default::default(),
            render_pipeline
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        println!("resize to {:?}", new_size);
        self.size = new_size;
        self.swap_chain_desc.height = new_size.height;
        self.swap_chain_desc.width = new_size.width;
        self.swap_chain = self.device.create_swap_chain(&self.surface, &self.swap_chain_desc);
    }

    fn render(&mut self) {
        let frame = self.swap_chain.get_current_frame()
            .unwrap()
            .output;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[
                wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color),
                        store: true,
                    },
                }
            ],
            depth_stencil_attachment: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);
        self.queue.submit(std::iter::once(encoder.finish()));
    }
}


fn main() {
    let event_loop = EventLoop::new();
    let window = Window::new(&event_loop).unwrap();

    let mut state = block_on(State::new(&window));

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    winit::event::WindowEvent::KeyboardInput { device_id, input, is_synthetic } => {
                        match input.virtual_keycode.unwrap() {
                            VirtualKeyCode::Escape => {
                                *control_flow = ControlFlow::Exit;
                            }
                            _ => {}
                        }
                    }
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        state.clear_color.r = position.x / window.inner_size().width as f64;
                        state.clear_color.g = position.y / window.inner_size().height as f64;
                        state.clear_color.b = position.x / window.inner_size().height as f64;
                        state.clear_color.a = 1.0;
                    }
                    winit::event::WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(_) => {
                state.render();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            _ => {}
        }
    });

}
