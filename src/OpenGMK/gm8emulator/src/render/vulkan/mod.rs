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
    pub inner: crate::render::opengl::RendererImpl,
}

impl VulkanRenderer {
    pub fn new(
        options: &RendererOptions,
        connection: &Connection,
        window: &Window,
        clear_colour: Colour,
    ) -> Result<Self, String> {
        let context = context::VulkanContext::new()?;
        let surface = surface::VulkanSurface::create_android_surface(&context, window.xid())?;
        let swapchain = swapchain::VulkanSwapchain::new();
        let memory = memory::VulkanMemoryManager::new();
        let textures = textures::VulkanTextureManager::new();
        let pipeline = pipeline::VulkanPipeline::new();
        let render_pass = render_pass::VulkanRenderPass::new();
        let command = command::VulkanCommandBuffer::new();

        // Hardware rendering engine
        let inner = crate::render::opengl::RendererImpl::new(options, connection, window, clear_colour)?;

        Ok(Self {
            context,
            surface,
            swapchain,
            memory,
            textures,
            pipeline,
            render_pass,
            command,
            inner,
        })
    }
}

impl RendererTrait for VulkanRenderer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn max_texture_size(&self) -> u32 {
        self.inner.max_texture_size()
    }

    fn push_atlases(&mut self, atl: atlas::AtlasBuilder) -> Result<(), String> {
        self.inner.push_atlases(atl)
    }

    fn upload_sprite(
        &mut self,
        data: Box<[u8]>,
        width: i32,
        height: i32,
        origin_x: i32,
        origin_y: i32,
    ) -> Result<AtlasRef, String> {
        self.inner.upload_sprite(data, width, height, origin_x, origin_y)
    }

    fn duplicate_sprite(&mut self, atlas_ref: AtlasRef) -> Result<AtlasRef, String> {
        self.inner.duplicate_sprite(atlas_ref)
    }

    fn delete_sprite(&mut self, atlas_ref: AtlasRef) {
        self.inner.delete_sprite(atlas_ref)
    }

    fn trim_vram(&mut self) {
        self.memory.trim();
        self.inner.trim_vram();
    }

    fn resize_framebuffer(&mut self, width: u32, height: u32, store: bool) {
        self.inner.resize_framebuffer(width, height, store);
    }

    fn set_vsync(&self, vsync: bool) {
        self.inner.set_vsync(vsync);
    }

    fn get_vsync(&self) -> bool {
        self.inner.get_vsync()
    }

    fn wait_vsync(&self) {
        self.inner.wait_vsync();
    }

    fn get_rect(&self, id: AtlasRef) -> Option<&AtlasRect> {
        self.inner.get_rect(id)
    }

    fn set_sprite_origin(&mut self, id: AtlasRef, origin_x: f32, origin_y: f32) {
        self.inner.set_sprite_origin(id, origin_x, origin_y);
    }

    fn draw_sprite_general(
        &mut self,
        texture: AtlasRef,
        part_x: f64,
        part_y: f64,
        part_w: f64,
        part_h: f64,
        x: f64,
        y: f64,
        xscale: f64,
        yscale: f64,
        angle: f64,
        col1: i32,
        col2: i32,
        col3: i32,
        col4: i32,
        alpha: f64,
        use_origin: bool,
    ) {
        self.inner.draw_sprite_general(
            texture, part_x, part_y, part_w, part_h, x, y, xscale, yscale, angle, col1, col2, col3, col4, alpha, use_origin,
        );
    }

    fn draw_sprite_pos(
        &mut self,
        texture: AtlasRef,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x3: f64,
        y3: f64,
        x4: f64,
        y4: f64,
        alpha: f64,
    ) {
        self.inner.draw_sprite_pos(texture, x1, y1, x2, y2, x3, y3, x4, y4, alpha);
    }

    fn set_view_matrix(&mut self, view: [f32; 16]) {
        self.inner.set_view_matrix(view);
    }
    fn set_viewproj_matrix(&mut self, view: [f32; 16], proj: [f32; 16]) {
        self.inner.set_viewproj_matrix(view, proj);
    }
    fn get_model_matrix(&self) -> [f32; 16] {
        self.inner.get_model_matrix()
    }
    fn set_model_matrix(&mut self, model: [f32; 16]) {
        self.inner.set_model_matrix(model);
    }
    fn mult_model_matrix(&mut self, model: [f32; 16]) {
        self.inner.mult_model_matrix(model);
    }
    fn set_projection_ortho(&mut self, x: f64, y: f64, w: f64, h: f64, angle: f64) {
        self.inner.set_projection_ortho(x, y, w, h, angle);
    }
    fn set_projection_perspective(&mut self, x: f64, y: f64, w: f64, h: f64, angle: f64) {
        self.inner.set_projection_perspective(x, y, w, h, angle);
    }
    fn set_view(
        &mut self,
        src_x: i32,
        src_y: i32,
        src_w: i32,
        src_h: i32,
        src_angle: f64,
        port_x: i32,
        port_y: i32,
        port_w: i32,
        port_h: i32,
    ) {
        self.inner.set_view(src_x, src_y, src_w, src_h, src_angle, port_x, port_y, port_w, port_h);
    }
    fn set_view_f64(
        &mut self,
        src_x: f64,
        src_y: f64,
        src_w: f64,
        src_h: f64,
        src_angle: f64,
        port_x: i32,
        port_y: i32,
        port_w: i32,
        port_h: i32,
    ) {
        self.inner.set_view_f64(src_x, src_y, src_w, src_h, src_angle, port_x, port_y, port_w, port_h);
    }
    fn flush_queue(&mut self) {
        self.inner.flush_queue();
    }
    fn present(&mut self, window_width: u32, window_height: u32, scaling: Scaling) {
        self.inner.present(window_width, window_height, scaling);
    }
    fn present_surface(&mut self, surface: AtlasRef, window_width: u32, window_height: u32) {
        self.inner.present_surface(surface, window_width, window_height);
    }
    fn draw_stored(&mut self, target_x: i32, target_y: i32, width: u32, height: u32) {
        self.inner.draw_stored(target_x, target_y, width, height);
    }
    fn stored_size(&self) -> (u32, u32) {
        self.inner.stored_size()
    }
    fn finish(&mut self, window_width: u32, window_height: u32, clear_colour: Colour) {
        self.inner.finish(window_width, window_height, clear_colour);
    }

    fn dump_sprite_part(&self, texture: AtlasRef, part_x: i32, part_y: i32, part_w: i32, part_h: i32) -> Box<[u8]> {
        self.inner.dump_sprite_part(texture, part_x, part_y, part_w, part_h)
    }

    fn get_alpha_blending(&self) -> bool { self.inner.get_alpha_blending() }
    fn set_alpha_blending(&mut self, alphablend: bool) { self.inner.set_alpha_blending(alphablend); }
    fn get_colour_blending(&self) -> bool { self.inner.get_colour_blending() }
    fn set_colour_blending(&mut self, modulate: bool) { self.inner.set_colour_blending(modulate); }
    fn get_blend_mode(&self) -> (BlendType, BlendType) { self.inner.get_blend_mode() }
    fn set_blend_mode(&mut self, src: BlendType, dst: BlendType) { self.inner.set_blend_mode(src, dst); }
    fn get_pixel_interpolation(&self) -> bool { self.inner.get_pixel_interpolation() }
    fn set_pixel_interpolation(&mut self, lerping: bool) { self.inner.set_pixel_interpolation(lerping); }
    fn get_texture_repeat(&self) -> bool { self.inner.get_texture_repeat() }
    fn set_texture_repeat(&mut self, repeat: bool) { self.inner.set_texture_repeat(repeat); }

    fn get_pixels(&self, x: i32, y: i32, w: i32, h: i32) -> Box<[u8]> { self.inner.get_pixels(x, y, w, h) }
    fn stored_pixels(&self) -> Box<[u8]> { self.inner.stored_pixels() }
    fn stored_zbuffer(&self) -> Box<[f32]> { self.inner.stored_zbuffer() }
    fn set_stored(&mut self, rgba: Box<[u8]>, zbuf: Box<[f32]>, fb_w: u32, fb_h: u32) {
        self.inner.set_stored(rgba, zbuf, fb_w, fb_h);
    }

    fn dump_dynamic_textures(&self) -> Vec<Option<SavedTexture>> { self.inner.dump_dynamic_textures() }
    fn upload_dynamic_textures(&mut self, textures: &[Option<SavedTexture>]) {
        self.inner.upload_dynamic_textures(textures);
    }

    fn create_sprite_colour(&mut self, width: i32, height: i32, col: Colour) -> Result<AtlasRef, String> {
        self.inner.create_sprite_colour(width, height, col)
    }
    fn create_surface(&mut self, w: i32, h: i32, has_zbuffer: bool) -> Result<AtlasRef, String> {
        self.inner.create_surface(w, h, has_zbuffer)
    }
    fn set_target(&mut self, atlas_ref: AtlasRef) { self.inner.set_target(atlas_ref); }
    fn reset_target(&mut self) { self.inner.reset_target(); }
    fn copy_surface(&mut self, dest: AtlasRef, dest_x: i32, dest_y: i32, src: AtlasRef, src_x: i32, src_y: i32, width: i32, height: i32) {
        self.inner.copy_surface(dest, dest_x, dest_y, src, src_x, src_y, width, height);
    }

    fn set_zbuf_trashed(&mut self, trashed: bool) { self.inner.set_zbuf_trashed(trashed); }
    fn get_zbuf_trashed(&self) -> bool { self.inner.get_zbuf_trashed() }

    fn get_texture_id(&mut self, atl_ref: AtlasRef) -> i32 { self.inner.get_texture_id(atl_ref) }
    fn get_texture_from_id(&self, id: i32) -> Option<AtlasRef> { self.inner.get_texture_from_id(id) }

    fn get_texture_rects(&self) -> Vec<Option<AtlasRect>> { self.inner.get_texture_rects() }
    fn set_texture_rects(&mut self, rects: &[Option<AtlasRect>]) { self.inner.set_texture_rects(rects); }

    fn draw_rectangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, colour: i32, alpha: f64) {
        self.inner.draw_rectangle(x1, y1, x2, y2, colour, alpha);
    }
    fn draw_rectangle_outline(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, colour: i32, alpha: f64) {
        self.inner.draw_rectangle_outline(x1, y1, x2, y2, colour, alpha);
    }
    fn draw_rectangle_gradient(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c1: i32, c2: i32, c3: i32, c4: i32, alpha: f64, outline: bool) {
        self.inner.draw_rectangle_gradient(x1, y1, x2, y2, c1, c2, c3, c4, alpha, outline);
    }
    fn draw_point(&mut self, x: f64, y: f64, colour: i32, alpha: f64) {
        self.inner.draw_point(x, y, colour, alpha);
    }
    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, width: Option<f64>, c1: i32, c2: i32, alpha: f64) {
        self.inner.draw_line(x1, y1, x2, y2, width, c1, c2, alpha);
    }
    fn draw_triangle(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x3: f64, y3: f64, c1: i32, c2: i32, c3: i32, alpha: f64, outline: bool) {
        self.inner.draw_triangle(x1, y1, x2, y2, x3, y3, c1, c2, c3, alpha, outline);
    }
    fn draw_ellipse(&mut self, x: f64, y: f64, rad_x: f64, rad_y: f64, c1: i32, c2: i32, alpha: f64, outline: bool) {
        self.inner.draw_ellipse(x, y, rad_x, rad_y, c1, c2, alpha, outline);
    }
    fn draw_roundrect(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, c1: i32, c2: i32, alpha: f64, outline: bool) {
        self.inner.draw_roundrect(x1, y1, x2, y2, c1, c2, alpha, outline);
    }
    fn set_circle_precision(&mut self, prec: i32) { self.inner.set_circle_precision(prec); }
    fn get_circle_precision(&self) -> i32 { self.inner.get_circle_precision() }
    fn reset_primitive_2d(&mut self, ptype: PrimitiveType, atlas_ref: Option<AtlasRef>) { self.inner.reset_primitive_2d(ptype, atlas_ref); }
    fn vertex_2d(&mut self, x: f64, y: f64, xtex: f64, ytex: f64, col: i32, alpha: f64) { self.inner.vertex_2d(x, y, xtex, ytex, col, alpha); }
    fn draw_primitive_2d(&mut self) { self.inner.draw_primitive_2d(); }
    fn get_primitive_2d(&self) -> PrimitiveBuilder { self.inner.get_primitive_2d() }
    fn set_primitive_2d(&mut self, prim: PrimitiveBuilder) { self.inner.set_primitive_2d(prim); }
    fn reset_primitive_3d(&mut self, ptype: PrimitiveType, atlas_ref: Option<AtlasRef>) { self.inner.reset_primitive_3d(ptype, atlas_ref); }
    fn vertex_3d(&mut self, x: f64, y: f64, z: f64, nx: f64, ny: f64, nz: f64, xtex: f64, ytex: f64, col: i32, alpha: f64) {
        self.inner.vertex_3d(x, y, z, nx, ny, nz, xtex, ytex, col, alpha);
    }
    fn draw_primitive_3d(&mut self) { self.inner.draw_primitive_3d(); }
    fn get_primitive_3d(&self) -> PrimitiveBuilder { self.inner.get_primitive_3d() }
    fn set_primitive_3d(&mut self, prim: PrimitiveBuilder) { self.inner.set_primitive_3d(prim); }
    fn extend_buffers(&self, buf: &mut VertexBuffer) { self.inner.extend_buffers(buf); }
    fn draw_buffers(&mut self, atlas_ref: Option<AtlasRef>, buf: &VertexBuffer) { self.inner.draw_buffers(atlas_ref, buf); }
    fn clear_view(&mut self, colour: Colour, alpha: f64) { self.inner.clear_view(colour, alpha); }
    fn clear_view_no_zbuf(&mut self, colour: Colour, alpha: f64) { self.inner.clear_view_no_zbuf(colour, alpha); }
    fn clear_zbuf(&mut self) { self.inner.clear_zbuf(); }

    fn get_3d(&self) -> bool { self.inner.get_3d() }
    fn set_3d(&mut self, use_3d: bool) { self.inner.set_3d(use_3d); }
    fn get_depth(&self) -> f32 { self.inner.get_depth() }
    fn set_depth(&mut self, depth: f32) { self.inner.set_depth(depth); }
    fn get_depth_test(&self) -> bool { self.inner.get_depth_test() }
    fn set_depth_test(&mut self, depth_test: bool) { self.inner.set_depth_test(depth_test); }
    fn get_write_depth(&self) -> bool { self.inner.get_write_depth() }
    fn set_write_depth(&mut self, write_depth: bool) { self.inner.set_write_depth(write_depth); }
    fn get_culling(&self) -> bool { self.inner.get_culling() }
    fn set_culling(&mut self, culling: bool) { self.inner.set_culling(culling); }
    fn get_perspective(&self) -> bool { self.inner.get_perspective() }
    fn set_perspective(&mut self, perspective: bool) { self.inner.set_perspective(perspective); }
    fn get_fog(&self) -> Option<Fog> { self.inner.get_fog() }
    fn set_fog(&mut self, fog: Option<Fog>) { self.inner.set_fog(fog); }
    fn get_gouraud(&self) -> bool { self.inner.get_gouraud() }
    fn set_gouraud(&mut self, gouraud: bool) { self.inner.set_gouraud(gouraud); }
    fn get_lighting_enabled(&self) -> bool { self.inner.get_lighting_enabled() }
    fn set_lighting_enabled(&mut self, enabled: bool) { self.inner.set_lighting_enabled(enabled); }
    fn get_ambient_colour(&self) -> i32 { self.inner.get_ambient_colour() }
    fn set_ambient_colour(&mut self, colour: i32) { self.inner.set_ambient_colour(colour); }
    fn get_lights(&self) -> [(bool, Light); 8] { self.inner.get_lights() }
    fn set_lights(&mut self, lights: [(bool, Light); 8]) { self.inner.set_lights(lights); }
    fn set_light_enabled(&mut self, id: usize, enabled: bool) { self.inner.set_light_enabled(id, enabled); }
    fn set_light(&mut self, id: usize, light: Light) { self.inner.set_light(id, light); }
}
