use std::{num::NonZeroU64, ops::Range};

use epaint::ClippedPrimitive;
use glam::Vec2;
use rend3::graph::{RenderGraph, RenderPassTarget, RenderPassTargets, RenderTargetHandle};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BlendComponent, BlendState, Buffer, BufferSlice, BufferUsages, Color, ColorTargetState,
    ColorWrites, DepthStencilState, Device, FragmentState, RenderPipeline, VertexAttribute,
    VertexState,
};

pub struct Locals {
    screen_size: Vec2,
    padding: Vec2,
}

pub struct Meshes {
    pub index_megabuffer: Buffer,
    pub index_ranges: Vec<Range<u32>>,
    pub vertex_megabuffer: Buffer,
    pub vertex_ranges: Vec<Range<u32>>,
}

pub struct EpaintShapeRoutine {
    pub pipeline: RenderPipeline,
    pub meshes: Option<Meshes>,
}

impl EpaintShapeRoutine {
    pub fn new(device: &Device) -> Self {
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("guee"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
                "shapes.wgsl"
            ))),
        });

        let locals_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("guee locals bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(std::mem::size_of::<Locals>() as _),
                },
                count: None,
            }],
        });

        let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("guee textures bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
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
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("guee pipeline layout"),
            bind_group_layouts: &[&locals_bgl, &texture_bgl],
            push_constant_ranges: &[],
        });

        let depth_stencil_state = DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            // TODO: This is disabling the depth test. Should reconsider when we introduce z-index
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        // 0: vec2 position
        // 1: vec2 texture coordinates
        // 2: uint color
        let vertex_attributes =
            &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("guee pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader_module,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 5 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: vertex_attributes,
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                unclipped_depth: false,
                conservative: false,
                cull_mode: None,
                front_face: wgpu::FrontFace::default(),
                polygon_mode: wgpu::PolygonMode::default(),
                strip_index_format: None,
            },
            depth_stencil: Some(depth_stencil_state),
            multisample: wgpu::MultisampleState {
                alpha_to_coverage_enabled: false,
                // TODO: Needs multisampling
                count: 1,
                mask: !0,
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                // TODO: There's two entry points that do the same thing. This
                // is probably something egui does in preparation for an
                // upcoming change that we don't need to care about.
                entry_point: "fs_main_gamma_framebuffer",
                targets: &[Some(ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: wgpu::BlendFactor::OneMinusDstAlpha,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        Self {
            pipeline,
            // Created during `upload_gpu_buffers`
            meshes: None,
        }
    }

    fn upload_gpu_buffers(&mut self, device: Device, paint_jobs: &[ClippedPrimitive]) {
        let mesh_iter = paint_jobs.iter().map(|x| match &x.primitive {
            epaint::Primitive::Mesh(mesh) => mesh,
            epaint::Primitive::Callback(_) => unimplemented!(),
        });

        //let index_buffer_size =
        //mesh_iter.clone().map(|x| x.indices.len()).sum::<usize>() * std::mem::size_of::<u32>();
        //let vertex_buffer_size = mesh_iter.map(|x| x.vertices.len()).sum::<usize>()
        //* std::mem::size_of::<epaint::Vertex>();

        let index_buffer_cpu = mesh_iter
            .clone()
            .flat_map(|x| x.indices.iter().copied())
            .collect::<Vec<_>>();
        let vertex_buffer_cpu = mesh_iter
            .clone()
            .flat_map(|x| x.vertices.iter().copied())
            .collect::<Vec<_>>();

        let (index_ranges, vertex_ranges) = {
            let mut index_ranges = vec![];
            let mut vertex_ranges = vec![];
            let mut index_offset = 0u32;
            let mut vertex_offset = 0u32;
            for mesh in mesh_iter {
                let indices_size = mesh.indices.len() * std::mem::size_of::<u32>();
                index_ranges.push(index_offset..index_offset + indices_size as u32);
                let vertices_size = mesh.vertices.len() * std::mem::size_of::<epaint::Vertex>();
                vertex_ranges.push(vertex_offset..vertex_offset + vertices_size as u32);
                index_offset += indices_size as u32;
                vertex_offset += vertices_size as u32;
            }
            (index_ranges, vertex_ranges)
        };

        self.meshes = Some(Meshes {
            index_megabuffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("guee index megabuffer"),
                contents: bytemuck::cast_slice(&index_buffer_cpu),
                usage: BufferUsages::INDEX,
            }),
            index_ranges,
            vertex_megabuffer: device.create_buffer_init(&BufferInitDescriptor {
                label: Some("guee vertex megabuffer"),
                contents: bytemuck::cast_slice(&vertex_buffer_cpu),
                usage: BufferUsages::VERTEX,
            }),
            vertex_ranges,
        })
    }

    fn add_draw_to_graph<'node>(
        &'node self,
        graph: &mut RenderGraph<'node>,
        paint_jobs: &'node Vec<ClippedPrimitive>,
        color: RenderTargetHandle,
    ) {
        let mut builder = graph.add_node("guee painting");
        let paint_jobs = builder.passthrough_ref(paint_jobs);
        let meshes = builder.passthrough_ref(&self.meshes);
        let color = builder.add_render_target_output(color);
        let render_pass = builder.add_renderpass(RenderPassTargets {
            targets: vec![RenderPassTarget {
                color,
                clear: Color::GREEN,
                // TODO: Multisampling
                resolve: None,
            }],
            depth_stencil: None,
        });

        builder.build(|pt, renderer, pass, temps, ready, graph_data| {
            let paint_jobs = pt.get(paint_jobs);
            let meshes = pt
                .get(meshes)
                .as_ref()
                .expect("Render called before uploading gpu buffers");
            let pass = pass.get_rpass(render_pass);
            for ((paint_job, index_range), vertex_range) in paint_jobs
                .iter()
                .zip(meshes.index_ranges.iter())
                .zip(meshes.vertex_ranges.iter())
            {
                match &paint_job.primitive {
                    // TODO: Use the clip rect
                    epaint::Primitive::Mesh(mesh) => {
                        pass.set_vertex_buffer(
                            0,
                            meshes.vertex_megabuffer.slice(vertex_range.to_u64_range()),
                        );
                        pass.set_index_buffer(
                            meshes.index_megabuffer.slice(index_range.to_u64_range()),
                            wgpu::IndexFormat::Uint32,
                        );
                    }
                    epaint::Primitive::Callback(_) => unimplemented!(),
                }
            }
        });
    }

    pub fn add_to_graph<'node>(
        &'node self,
        graph: &mut RenderGraph<'node>,
        paint_jobs: &'node Vec<ClippedPrimitive>,
    ) {
    }
}

trait CastRange {
    fn to_u64_range(&self) -> Range<u64>;
}

impl CastRange for Range<u32> {
    fn to_u64_range(&self) -> Range<u64> {
        self.start as u64..self.end as u64
    }
}
