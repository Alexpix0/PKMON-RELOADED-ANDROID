//! Vulkan graphics backend for OpenGMK with automatic fallback.

pub mod command;
pub mod context;
pub mod memory;
pub mod pipeline;
pub mod render_pass;
pub mod surface;
pub mod swapchain;
pub mod textures;

use std::any::Any;
use crate::render::{
    atlas::{self, AtlasRect, AtlasRef},
    BlendType, Colour, Fog, Light, PrimitiveBuilder, PrimitiveType, RendererOptions, RendererTrait,
    SavedTexture, Scaling, VertexBuffer,
};
use ramen::{connection::Connection, window::Window};

pub struct VulkanRenderer {
    pub context: context::VulkanContext,
    pub surface: surface::VulkanSurface,
    pub swapchain: swapchain::VulkanSwapchain,
    pub memory: memory::VulkanMemoryManager,
    pub textures: textures::VulkanTextureManager,
    pub pipeline: pipeline::VulkanPipeline,
    pub render_pass: render_pass::VulkanRenderPass,
    pub command: command::VulkanCommandBuffer,
    vsync: bool,
    draw_colour: Colour,
    draw_alpha: f64,
    depth: f32,
    depth_test: bool,
    write_depth: bool,
    culling: bool,
    perspective: bool,
    fog: Option<Fog>,
    gouraud: bool,
    lighting_enabled: bool,
    ambient_colour: i32,
    lights: [(bool, Light); 8],
    options: RendererOptions,
}

impl VulkanRenderer {
    pub fn new(
        options: &RendererOptions,
        _connection: &Connection,
        _window: &Window,
        clear_colour: Colour,
    ) -> Result<Self, String> {
        let context = context::VulkanContext::new()?;
        let surface = surface::VulkanSurface::create_android_surface(&context, std::ptr::null_mut())?;
        let swapchain = swapchain::VulkanSwapchain::new();
        let memory = memory::VulkanMemoryManager::new();
        let textures = textures::VulkanTextureManager::new();
        let pipeline = pipeline::VulkanPipeline::new();
        let render_pass = render_pass::VulkanRenderPass::new();
        let command = command::VulkanCommandBuffer::new();

        Ok(Self {
            context,
            surface,
            swapchain,
            memory,
            textures,
            pipeline,
            render_pass,
            command,
            vsync: options.vsync,
            draw_colour: clear_colour,
            draw_alpha: 1.0,
            depth: 0.0,
            depth_test: false,
            write_depth: true,
            culling: false,
            perspective: false,
            fog: None,
            gouraud: true,
            lighting_enabled: false,
            ambient_colour: 0,
            lights: [(false, Light::default()); 8],
            options: options.clone(),
        })
    }
}

impl RendererTrait for VulkanRenderer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn max_texture_size(&self) -> u32 {
        self.textures.max_texture_dimension
    }

    fn push_atlases(&mut self, _atl: atlas::AtlasBuilder) -> Result<(), String> {
        Ok(())
    }

    fn upload_sprite(
        &mut self,
        _data: Box<[u8]>,
        width: i32,
        height: i32,
        _origin_x: i32,
        _origin_y: i32,
    ) -> Result<AtlasRef, String> {
        let id = self.textures.textures.len() as i32 + 1;
        self.textures.textures.insert(id, AtlasRect {
            tex: id as u32,
            x: 0,
            y: 0,
            w: width,
            h: height,
            orig_x: 0,
            orig_y: 0,
        });
        Ok(AtlasRef(id))
    }

    fn duplicate_sprite(&mut self, atlas_ref: AtlasRef) -> Result<AtlasRef, String> {
        let id = self.textures.textures.len() as i32 + 1;
        if let Some(rect) = self.textures.textures.get(&atlas_ref.0).copied() {
            self.textures.textures.insert(id, rect);
        }
        Ok(AtlasRef(id))
    }

    fn delete_sprite(&mut self, atlas_ref: AtlasRef) {
        self.textures.textures.remove(&atlas_ref.0);
    }

    fn trim_vram(&mut self) {
        self.memory.trim();
    }

    fn resize_framebuffer(&mut self, _width: u32, _height: u32, _store: bool) {}

    fn set_vsync(&self, _vsync: bool) {}

    fn get_vsync(&self) -> bool {
        self.vsync
    }

    fn wait_vsync(&self) {}

    fn get_rect(&self, id: AtlasRef) -> Option<&AtlasRect> {
        self.textures.textures.get(&id.0)
    }

    fn set_sprite_origin(&mut self, id: AtlasRef, origin_x: f32, origin_y: f32) {
        if let Some(rect) = self.textures.textures.get_mut(&id.0) {
            rect.orig_x = origin_x as i32;
            rect.orig_y = origin_y as i32;
        }
    }

    fn draw_sprite_general(
        &mut self,
        _texture: AtlasRef,
        _part_x: f64,
        _part_y: f64,
        _part_w: f64,
        _part_h: f64,
        _x: f64,
        _y: f64,
        _xscale: f64,
        _yscale: f64,
        _angle: f64,
        _col1: i32,
        _col2: i32,
        _col3: i32,
        _col4: i32,
        _alpha: f64,
        _use_origin: bool,
    ) {}

    fn draw_sprite_pos(
        &mut self,
        _texture: AtlasRef,
        _x1: f64,
        _y1: f64,
        _x2: f64,
        _y2: f64,
        _x3: f64,
        _y3: f64,
        _x4: f64,
        _y4: f64,
        _alpha: f64,
    ) {}

    fn set_view_matrix(&mut self, _view: [f32; 16]) {}
    fn set_viewproj_matrix(&mut self, _view: [f32; 16], _proj: [f32; 16]) {}
    fn get_model_matrix(&self) -> [f32; 16] { [0.0; 16] }
    fn set_model_matrix(&mut self, _model: [f32; 16]) {}
    fn mult_model_matrix(&mut self, _model: [f32; 16]) {}
    fn set_projection_ortho(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _angle: f64) {}
    fn set_projection_perspective(&mut self, _x: f64, _y: f64, _w: f64, _h: f64, _angle: f64) {}
    fn set_view(
        &mut self,
        _src_x: i32,
        _src_y: i32,
        _src_w: i32,
        _src_h: i32,
        _src_angle: f64,
        _port_x: i32,
        _port_y: i32,
        _port_w: i32,
        _port_h: i32,
    ) {}
    fn flush_queue(&mut self) {}
    fn present(&mut self, _window_width: u32, _window_height: u32, _scaling: Scaling) {}
    fn draw_stored(&mut self, _target_x: i32, _target_y: i32, _width: u32, _height: u32) {}
    fn stored_size(&self) -> (u32, u32) { (0, 0) }
    fn finish(&mut self, _window_width: u32, _window_height: u32, _clear_colour: Colour) {}

    fn dump_sprite_part(&self, _texture: AtlasRef, _part_x: i32, _part_y: i32, _part_w: i32, _part_h: i32) -> Box<[u8]> {
        Box::new([])
    }

    fn get_alpha_blending(&self) -> bool { true }
    fn set_alpha_blending(&mut self, _alphablend: bool) {}
    fn get_colour_blending(&self) -> bool { true }
    fn set_colour_blending(&mut self, _modulate: bool) {}
    fn get_blend_mode(&self) -> (BlendType, BlendType) { (BlendType::SrcAlpha, BlendType::InvSrcAlpha) }
    fn set_blend_mode(&mut self, _src: BlendType, _dst: BlendType) {}
    fn get_pixel_interpolation(&self) -> bool { false }
    fn set_pixel_interpolation(&mut self, _lerping: bool) {}
    fn get_texture_repeat(&self) -> bool { false }
    fn set_texture_repeat(&mut self, _repeat: bool) {}

    fn get_pixels(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Box<[u8]> { Box::new([]) }
    fn stored_pixels(&self) -> Box<[u8]> { Box::new([]) }
    fn stored_zbuffer(&self) -> Box<[f32]> { Box::new([]) }
    fn set_stored(&mut self, _rgba: Box<[u8]>, _zbuf: Box<[f32]>, _fb_w: u32, _fb_h: u32) {}

    fn dump_dynamic_textures(&self) -> Vec<Option<SavedTexture>> { Vec::new() }
    fn upload_dynamic_textures(&mut self, _textures: &[Option<SavedTexture>]) {}

    fn create_sprite_colour(&mut self, _width: i32, _height: i32, _col: Colour) -> Result<AtlasRef, String> {
        Ok(AtlasRef(0))
    }
    fn create_surface(&mut self, _w: i32, _h: i32, _has_zbuffer: bool) -> Result<AtlasRef, String> {
        Ok(AtlasRef(0))
    }
    fn set_target(&mut self, _atlas_ref: AtlasRef) {}
    fn reset_target(&mut self) {}
    fn copy_surface(&mut self, _dest: AtlasRef, _dest_x: i32, _dest_y: i32, _src: AtlasRef, _src_x: i32, _src_y: i32, _width: i32, _height: i32) {}

    fn set_zbuf_trashed(&mut self, _trashed: bool) {}
    fn get_zbuf_trashed(&self) -> bool { false }

    fn get_texture_id(&mut self, atl_ref: AtlasRef) -> i32 { atl_ref.0 }
    fn get_texture_from_id(&self, id: i32) -> Option<AtlasRef> { Some(AtlasRef(id)) }

    fn get_texture_rects(&self) -> Vec<Option<AtlasRect>> { Vec::new() }
    fn set_texture_rects(&mut self, _rects: &[Option<AtlasRect>]) {}

    fn draw_rectangle(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _colour: i32, _alpha: f64) {}
    fn draw_rectangle_outline(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _colour: i32, _alpha: f64) {}
    fn draw_rectangle_gradient(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _c1: i32, _c2: i32, _c3: i32, _c4: i32, _alpha: f64, _outline: bool) {}
    fn draw_point(&mut self, _x: f64, _y: f64, _colour: i32, _alpha: f64) {}
    fn draw_line(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _width: Option<f64>, _c1: i32, _c2: i32, _alpha: f64) {}
    fn draw_triangle(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _x3: f64, _y3: f64, _c1: i32, _c2: i32, _c3: i32, _alpha: f64, _outline: bool) {}
    fn draw_ellipse(&mut self, _x: f64, _y: f64, _rad_x: f64, _rad_y: f64, _c1: i32, _c2: i32, _alpha: f64, _outline: bool) {}
    fn draw_roundrect(&mut self, _x1: f64, _y1: f64, _x2: f64, _y2: f64, _c1: i32, _c2: i32, _alpha: f64, _outline: bool) {}
    fn set_circle_precision(&mut self, _prec: i32) {}
    fn get_circle_precision(&self) -> i32 { 24 }
    fn reset_primitive_2d(&mut self, _ptype: PrimitiveType, _atlas_ref: Option<AtlasRef>) {}
    fn vertex_2d(&mut self, _x: f64, _y: f64, _xtex: f64, _ytex: f64, _col: i32, _alpha: f64) {}
    fn draw_primitive_2d(&mut self) {}
    fn get_primitive_2d(&self) -> PrimitiveBuilder { PrimitiveBuilder::default() }
    fn set_primitive_2d(&mut self, _prim: PrimitiveBuilder) {}
    fn reset_primitive_3d(&mut self, _ptype: PrimitiveType, _atlas_ref: Option<AtlasRef>) {}
    fn vertex_3d(&mut self, _x: f64, _y: f64, _z: f64, _nx: f64, _ny: f64, _nz: f64, _xtex: f64, _ytex: f64, _col: i32, _alpha: f64) {}
    fn draw_primitive_3d(&mut self) {}
    fn get_primitive_3d(&self) -> PrimitiveBuilder { PrimitiveBuilder::default() }
    fn set_primitive_3d(&mut self, _prim: PrimitiveBuilder) {}
    fn extend_buffers(&self, _buf: &mut VertexBuffer) {}
    fn draw_buffers(&mut self, _atlas_ref: Option<AtlasRef>, _buf: &VertexBuffer) {}
    fn clear_view(&mut self, _colour: Colour, _alpha: f64) {}
    fn clear_view_no_zbuf(&mut self, _colour: Colour, _alpha: f64) {}
    fn clear_zbuf(&mut self) {}

    fn get_3d(&self) -> bool { false }
    fn set_3d(&mut self, _use_3d: bool) {}
    fn get_depth(&self) -> f32 { self.depth }
    fn set_depth(&mut self, depth: f32) { self.depth = depth; }
    fn get_depth_test(&self) -> bool { self.depth_test }
    fn set_depth_test(&mut self, depth_test: bool) { self.depth_test = depth_test; }
    fn get_write_depth(&self) -> bool { self.write_depth }
    fn set_write_depth(&mut self, write_depth: bool) { self.write_depth = write_depth; }
    fn get_culling(&self) -> bool { self.culling }
    fn set_culling(&mut self, culling: bool) { self.culling = culling; }
    fn get_perspective(&self) -> bool { self.perspective }
    fn set_perspective(&mut self, perspective: bool) { self.perspective = perspective; }
    fn get_fog(&self) -> Option<Fog> { self.fog }
    fn set_fog(&mut self, fog: Option<Fog>) { self.fog = fog; }
    fn get_gouraud(&self) -> bool { self.gouraud }
    fn set_gouraud(&mut self, gouraud: bool) { self.gouraud = gouraud; }
    fn get_lighting_enabled(&self) -> bool { self.lighting_enabled }
    fn set_lighting_enabled(&mut self, enabled: bool) { self.lighting_enabled = enabled; }
    fn get_ambient_colour(&self) -> i32 { self.ambient_colour }
    fn set_ambient_colour(&mut self, colour: i32) { self.ambient_colour = colour; }
    fn get_lights(&self) -> [(bool, Light); 8] { self.lights }
    fn set_lights(&mut self, lights: [(bool, Light); 8]) { self.lights = lights; }
    fn set_light_enabled(&mut self, id: usize, enabled: bool) {
        if id < 8 { self.lights[id].0 = enabled; }
    }
    fn set_light(&mut self, id: usize, light: Light) {
        if id < 8 { self.lights[id].1 = light; }
    }
}
