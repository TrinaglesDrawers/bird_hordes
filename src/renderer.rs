use arcana::{
    anim::graph,
    bumpalo::collections::Vec as BVec,
    camera::Camera3,
    graphics::{Color, Graphics, Material, Mesh, Normal3, Position3, Scale, VertexType as _},
    graphics::{Renderer, RendererContext},
    na, Global3, Viewport,
};
use sierra::{
    descriptors, graphics_pipeline_desc, mat4, pass, pipeline, shader_repr, vec3, AccessFlags,
    Blending, ClearColor, ClearDepth, ColorBlend, DepthTest, DescriptorsInput,
    DynamicGraphicsPipeline, Fence, Format, FragmentShader, Image, ImageInfo, ImageUsage,
    ImageView, Layout, PipelineInput, PipelineStageFlags, Sampler, SamplerInfo, Samples::Samples1,
    ShaderModuleInfo, VertexInputAttribute, VertexInputBinding, VertexInputRate, VertexShader,
};

pub struct MyRenderer {
    main_pipeline_layout: <MainPipeline as PipelineInput>::Layout,
    main_pipeline: DynamicGraphicsPipeline,
    main_render_pass: MainRenderPassInstance,

    light_pipeline_layout: <LightPipeline as PipelineInput>::Layout,
    light_pipeline: DynamicGraphicsPipeline,
    light_render_pass: LightRenderPassInstance,

    light_descriptors: <LightDescriptors as DescriptorsInput>::Instance,

    fences: [Option<Fence>; 3],
    fence_index: usize,

    norm_depths: Option<Image>,
    albedos: Option<Image>,
}

#[pass]
#[subpass(color = norm_depths, color = albedos, depth = depth)]
struct MainRenderPass {
    #[attachment(store(const Layout::ShaderReadOnlyOptimal), clear(const ClearColor(0.0, 0.0, 0.0, 0.0)))]
    norm_depths: Image,

    #[attachment(store(const Layout::ShaderReadOnlyOptimal), clear(const ClearColor(0.0, 0.0, 0.0, 0.0)))]
    albedos: Image,

    #[attachment(clear(const ClearDepth(1.0)))]
    depth: Format,
}

#[pass]
#[subpass(color = color)]
struct LightRenderPass {
    #[attachment(store(const Layout::Present), clear(const ClearColor(0.2, 0.1, 0.1, 1.0)))]
    color: Image,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct MainUniforms {
    camera_iview: mat4,
    camera_proj: mat4,
    transform: mat4,
}

impl Default for MainUniforms {
    fn default() -> Self {
        MainUniforms {
            camera_iview: mat4::default(),
            camera_proj: mat4::default(),
            transform: mat4::default(),
        }
    }
}

#[shader_repr]
#[derive(Clone, Copy)]
struct DirLight {
    dir: vec3,
    color: vec3,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct PointLight {
    pos: vec3,
    color: vec3,
}

#[shader_repr]
#[derive(Clone, Copy)]
struct LightUniforms {
    camera_view: mat4,
    camera_iproj: mat4,
    directional: DirLight,
    point: [PointLight; 4],
}

impl Default for LightUniforms {
    fn default() -> Self {
        LightUniforms {
            camera_view: mat4::default(),
            camera_iproj: mat4::default(),
            directional: DirLight {
                dir: vec3::default(),
                color: vec3::default(),
            },
            point: [PointLight {
                pos: vec3::default(),
                color: vec3::default(),
            }; 4],
        }
    }
}

#[descriptors]
struct MainDescriptors {
    #[uniform]
    #[stages(Vertex, Fragment)]
    uniforms: MainUniforms,
}

#[descriptors]
struct LightDescriptors {
    #[sampled_image]
    #[stages(Fragment)]
    norm_depths: Image,

    #[sampled_image]
    #[stages(Fragment)]
    albedos: Image,

    #[uniform]
    #[stages(Vertex, Fragment)]
    uniforms: LightUniforms,
}

#[pipeline]
struct MainPipeline {
    #[set]
    set: MainDescriptors,
}

#[pipeline]
struct LightPipeline {
    #[set]
    set: LightDescriptors,
}

struct MyRenderable {
    descriptors: <MainDescriptors as DescriptorsInput>::Instance,
}

impl MyRenderer {
    fn render(&mut self, cx: RendererContext<'_>, viewport: &mut Viewport) -> eyre::Result<()> {
        if let Some(fence) = &mut self.fences[self.fence_index] {
            cx.graphics.wait_fences(&mut [fence], true);
            cx.graphics.reset_fences(&mut [fence]);
        }

        let view = cx
            .world
            .get_mut::<Global3>(viewport.camera())?
            .iso
            .to_homogeneous();

        let iview = cx
            .world
            .get_mut::<Global3>(viewport.camera())?
            .iso
            .inverse()
            .to_homogeneous();

        let proj = cx
            .world
            .get_mut::<Camera3>(viewport.camera())?
            .proj()
            .to_homogeneous();

        let iproj = cx
            .world
            .get_mut::<Camera3>(viewport.camera())?
            .proj()
            .inverse()
            .to_homogeneous();

        let mut swapchain_image = viewport.acquire_image(true)?;

        let mut main_uniforms = MainUniforms::default();
        main_uniforms.camera_iview = mat4_na_to_sierra(iview);
        main_uniforms.camera_proj = mat4_na_to_sierra(proj);

        let mut light_uniforms = LightUniforms::default();
        light_uniforms.camera_view = mat4_na_to_sierra(view);
        light_uniforms.camera_iproj = mat4_na_to_sierra(iproj);

        if let Some(directional) = cx.res.get::<crate::light::DirLight>() {
            light_uniforms.directional = DirLight {
                dir: vec3_na_to_sierra(directional.dir),
                color: directional.color.into(),
            }
        }

        for (i, (_, (p, g))) in cx
            .world
            .query_mut::<(&crate::light::PointLight, &Global3)>()
            .into_iter()
            .take(4)
            .enumerate()
        {
            light_uniforms.point[i] = PointLight {
                pos: vec3_na_to_sierra(g.iso.translation.vector),
                color: p.color.into(),
            }
        }

        let mut new_entities = BVec::new_in(cx.bump);

        for (e, ()) in cx
            .world
            .query_mut::<()>()
            .with::<Mesh>()
            .with::<Global3>()
            .without::<MyRenderable>()
        {
            new_entities.push(e);
        }

        for e in new_entities {
            cx.world
                .insert_one(
                    e,
                    MyRenderable {
                        descriptors: self.main_pipeline_layout.set.instance(),
                    },
                )
                .unwrap();
        }

        let mut encoder = cx.graphics.create_encoder(cx.bump)?;
        let mut render_pass_encoder = cx.graphics.create_encoder(cx.bump)?;

        let extent = swapchain_image.image().info().extent;

        let norm_depths = match &self.norm_depths {
            Some(image) if image.info().extent == extent => image,
            _ => {
                self.norm_depths = None;
                self.norm_depths
                    .get_or_insert(cx.graphics.create_image(ImageInfo {
                        extent,
                        format: Format::RGBA32Sfloat,
                        levels: 1,
                        layers: 1,
                        samples: Samples1,
                        usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED,
                    })?)
            }
        };

        let albedos = match &self.albedos {
            Some(image) if image.info().extent == extent => image,
            _ => {
                self.albedos = None;
                self.albedos
                    .get_or_insert(cx.graphics.create_image(ImageInfo {
                        extent,
                        format: Format::RGBA32Sfloat,
                        levels: 1,
                        layers: 1,
                        samples: Samples1,
                        usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED,
                    })?)
            }
        };

        let mut render_pass = render_pass_encoder.with_render_pass(
            &mut self.main_render_pass,
            &MainRenderPass {
                norm_depths: norm_depths.clone(),
                albedos: albedos.clone(),
                depth: Format::D16Unorm,
            },
            cx.graphics,
        )?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.main_pipeline, cx.graphics)?;

        let mut writes = BVec::new_in(cx.bump);
        for (_, (mesh, global, renderable, scale)) in
            cx.world
                .query_mut::<(&Mesh, &Global3, &mut MyRenderable, Option<&Scale>)>()
        {
            match scale {
                Some(scale) => {
                    let m = na::Matrix4::<f32>::new_nonuniform_scaling(&scale.0);
                    main_uniforms.transform = mat4_na_to_sierra(global.iso.to_homogeneous() * m);
                }
                None => {
                    main_uniforms.transform = mat4_na_to_sierra(global.iso.to_homogeneous());
                }
            }

            let updated = renderable.descriptors.update(
                &MainDescriptors {
                    uniforms: main_uniforms,
                },
                self.fence_index,
                cx.graphics,
                &mut writes,
                &mut encoder,
            )?;

            render_pass.bind_graphics_descriptors(&self.main_pipeline_layout, updated);

            let drawn = mesh.draw(
                0..1,
                &[Position3::layout(), Normal3::layout(), Color::layout()],
                &mut render_pass,
                cx.bump,
            );

            if drawn {
                tracing::info!("Mesh drawn");
            } else {
                tracing::warn!("Mesh is not drawn");
            }
        }

        drop(render_pass);

        render_pass_encoder.memory_barrier(
            PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            AccessFlags::COLOR_ATTACHMENT_WRITE,
            PipelineStageFlags::FRAGMENT_SHADER,
            AccessFlags::SHADER_READ,
        );

        let mut render_pass = render_pass_encoder.with_render_pass(
            &mut self.light_render_pass,
            &LightRenderPass {
                color: swapchain_image.image().clone(),
            },
            cx.graphics,
        )?;

        render_pass.bind_dynamic_graphics_pipeline(&mut self.light_pipeline, cx.graphics)?;

        let updated = self.light_descriptors.update(
            &LightDescriptors {
                norm_depths: norm_depths.clone(),
                albedos: albedos.clone(),
                uniforms: light_uniforms,
            },
            self.fence_index,
            cx.graphics,
            &mut writes,
            &mut encoder,
        )?;

        render_pass.bind_graphics_descriptors(&self.light_pipeline_layout, updated);
        render_pass.draw(0..3, 0..1);
        drop(render_pass);

        cx.graphics.update_descriptor_sets(&writes, &[]);

        let fence = match &mut self.fences[self.fence_index] {
            Some(fence) => fence,
            None => self.fences[self.fence_index].get_or_insert(cx.graphics.create_fence()?),
        };

        let [wait, signal] = swapchain_image.wait_signal();

        cx.graphics.submit(
            &mut [(PipelineStageFlags::BOTTOM_OF_PIPE, wait)],
            std::array::IntoIter::new([encoder.finish(), render_pass_encoder.finish()]),
            &mut [signal],
            Some(fence),
            cx.bump,
        );

        cx.graphics.present(swapchain_image)?;

        Ok(())
    }
}

impl Renderer for MyRenderer {
    fn new(graphics: &mut Graphics) -> eyre::Result<Self> {
        let main_shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("main.wgsl").to_vec().into_boxed_slice(),
        ))?;

        let light_shader_module = graphics.create_shader_module(ShaderModuleInfo::wgsl(
            std::include_bytes!("light.wgsl")
                .to_vec()
                .into_boxed_slice(),
        ))?;

        let main_pipeline_layout = MainPipeline::layout(graphics)?;
        let light_pipeline_layout = LightPipeline::layout(graphics)?;

        Ok(MyRenderer {
            main_pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_bindings: vec![
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 12,
                    },
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 12,
                    },
                    VertexInputBinding {
                        rate: VertexInputRate::Vertex,
                        stride: 16,
                    },
                ],
                vertex_attributes: vec![
                    VertexInputAttribute { location: 0, format: Format::RGB32Sfloat, binding: 0, offset: 0 },
                    VertexInputAttribute { location: 1, format: Format::RGB32Sfloat, binding: 1, offset: 0 },
                    VertexInputAttribute { location: 2, format: Format::RGBA32Sfloat, binding: 2, offset: 0 },
                ],
                vertex_shader: VertexShader::new(main_shader_module.clone(), "vs_main"),
                fragment_shader: Some(FragmentShader::new(main_shader_module.clone(), "fs_main")),
                layout: main_pipeline_layout.raw().clone(),
                depth_test: Some(DepthTest::LESS_WRITE),
                color_blend: ColorBlend::Blending {
                    blending: Some(sierra::Blending {
                            color_src_factor: sierra::BlendFactor::One,
                            color_dst_factor: sierra::BlendFactor::Zero,
                            color_op: sierra::BlendOp::Add,
                            alpha_src_factor: sierra::BlendFactor::One,
                            alpha_dst_factor: sierra::BlendFactor::Zero,
                            alpha_op: sierra::BlendOp::Add,
                        }),
                        write_mask: sierra::ComponentMask::RGBA,
                    constants: sierra::State::Static { value: [0.0.into(); 4] }
                },
            }),
            main_render_pass: MainRenderPass::instance(),
            main_pipeline_layout,

            light_descriptors: light_pipeline_layout.set.instance(),
            light_pipeline: DynamicGraphicsPipeline::new(graphics_pipeline_desc! {
                vertex_shader: VertexShader::new(light_shader_module.clone(), "vs_light"),
                fragment_shader: Some(FragmentShader::new(light_shader_module.clone(), "fs_light")),
                layout: light_pipeline_layout.raw().clone(),
            }),
            light_render_pass: LightRenderPass::instance(),
            light_pipeline_layout,

            fences: [None, None, None],
            fence_index: 0,

            norm_depths: None,
            albedos: None,
        })
    }

    fn render(
        &mut self,
        mut cx: RendererContext<'_>,
        viewports: &mut [&mut Viewport],
    ) -> eyre::Result<()> {
        for viewport in viewports {
            let viewport = &mut **viewport;
            if viewport.needs_redraw() {
                self.render(cx.reborrow(), viewport)?;
            }
        }

        Ok(())
    }
}

#[inline(always)]
fn mat4_na_to_sierra(m: na::Matrix4<f32>) -> arcana::sierra::mat4<f32> {
    let array: [[f32; 4]; 4] = m.into();
    arcana::sierra::mat4::from(array)
}

#[inline(always)]
fn vec3_na_to_sierra(v: na::Vector3<f32>) -> arcana::sierra::vec3<f32> {
    let array: [f32; 3] = v.into();
    arcana::sierra::vec3::from(array)
}
