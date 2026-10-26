use crate::{
    asset::{self, font, Font},
    game::{Game, GetAsset, PlayType, Version},
    gml,
    math::Real,
};
use serde::{Deserialize, Serialize};

pub static BATTLE_DRAW_ENTITIES_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
pub static BATTLE_DRAW_HUD_US: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Halign {
    Left,
    Middle,
    Right,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum Valign {
    Top,
    Middle,
    Bottom,
}

struct LineIterator<'a> {
    text: std::borrow::Cow<'a, [u8]>,
    pos: usize,
    font: &'a font::Font,
    max_width: Option<i32>,
    word_buf: [u8; 64],
    word_len: usize,
    word_width: i32,
    line_buf: [u8; 256],
    line_len: usize,
}

impl<'a> LineIterator<'a> {
    fn next(&mut self) -> Option<(&[u8], i32)> {
        if self.pos >= self.text.len() {
            return None
        }
        self.line_len = 0;
        self.word_len = 0;
        self.word_width = 0;
        let mut line_width = 0;

        macro_rules! push_word_char {
            ($self:ident, $c:expr) => {
                if $self.word_len < $self.word_buf.len() {
                    $self.word_buf[$self.word_len] = $c;
                    $self.word_len += 1;
                }
            };
        }

        macro_rules! flush_word_to_line {
            ($self:ident, $line_width:ident) => {
                if $self.word_len > 0 {
                    let copy_len = $self.word_len.min($self.line_buf.len() - $self.line_len);
                    $self.line_buf[$self.line_len..$self.line_len + copy_len]
                        .copy_from_slice(&$self.word_buf[..copy_len]);
                    $self.line_len += copy_len;
                    $line_width += $self.word_width;
                    $self.word_len = 0;
                    $self.word_width = 0;
                }
            };
        }

        let mut iter = self.text[self.pos..].iter().copied().enumerate().peekable();
        while let Some((_, c)) = iter.next() {
            // First, process escape characters
            let c = match c {
                b'#' | b'\r' | b'\n' => {
                    // '#' is a newline character, don't process it but start a new line instead
                    // Likewise CR, LF, and CRLF
                    if c == b'\r' && iter.peek().map(|t| t.1) == Some(b'\n') {
                        // CRLF only counts as one line break so consume the LF
                        iter.next();
                    }
                    b'\n'
                },
                b'\\' if iter.peek().map(|t| t.1) == Some(b'#') => {
                    // '\#' is an escaped newline character, treat it as '#'
                    iter.next();
                    b'#'
                },
                _ if self.font.get_char(c).is_some() => c, // Normal character
                _ => b' ',                                 // Character is not in the font, replace with space
            };
            // Next, insert the character into the word buffer
            match c {
                b'\n' => {
                    flush_word_to_line!(self, line_width);
                    break
                },
                _ => {
                    if let Some(character) = self.font.get_char(c) {
                        push_word_char!(self, c);
                        self.word_width += character.offset;
                    } else {
                        // Space when it isn't in the font
                        push_word_char!(self, b' ');
                        if let Some(character) = self.font.get_char(self.font.first) {
                            self.word_width += character.offset;
                        }
                    }
                },
            };

            // Check if we're going over the max width
            if let Some(max_width) = self.max_width {
                if line_width + self.word_width > max_width && line_width != 0 {
                    break
                }
            }

            // Push new word if applicable
            if c == b' ' {
                flush_word_to_line!(self, line_width);
            }
        }

        if let Some((pos, _)) = iter.peek() {
            self.pos += pos;
        } else {
            // Add the last word
            flush_word_to_line!(self, line_width);
            self.pos = self.text.len();
        }

        Some((&self.line_buf[..self.line_len], line_width))
    }
}

impl Game {
    /// Updates views that should be following objects.
    pub fn update_views(&mut self) {
        if self.room.views_enabled {
            for view in self.room.views.iter_mut().filter(|x| x.visible) {
                if let Some(handle) = match view.follow_target {
                    obj_id if obj_id < 100000 => {
                        self.room.instance_list.iter_by_identity(view.follow_target).next(&self.room.instance_list)
                    },
                    inst_id => self.room.instance_list.get_by_instid(inst_id),
                } {
                    let inst = self.room.instance_list.get(handle);

                    let x = inst.x.get().round().to_i32();
                    let y = inst.y.get().round().to_i32();
                    if view.follow_hborder < view.source_w / 2 {
                        let border_left = x - view.follow_hborder;
                        let border_right = x + view.follow_hborder;
                        if border_left < view.source_x {
                            if view.follow_hspeed < 0 {
                                view.source_x = border_left;
                            } else {
                                view.source_x -= (view.source_x - border_left).min(view.follow_hspeed);
                            }
                        } else if border_right > view.source_x + view.source_w {
                            if view.follow_hspeed < 0 {
                                view.source_x = border_right - view.source_w;
                            } else {
                                view.source_x +=
                                    (border_right - (view.source_x + view.source_w)).min(view.follow_hspeed);
                            }
                        }
                    } else {
                        view.source_x = x - view.source_w / 2;
                    }
                    view.source_x = view.source_x.max(0).min(self.room.width - view.source_w);

                    if view.follow_vborder < view.source_h / 2 {
                        let border_top = y - view.follow_vborder;
                        let border_bottom = y + view.follow_vborder;
                        if border_top < view.source_y {
                            if view.follow_vspeed < 0 {
                                view.source_y = border_top;
                            } else {
                                view.source_y -= (view.source_y - border_top).min(view.follow_vspeed);
                            }
                        } else if border_bottom > view.source_y + view.source_h {
                            if view.follow_vspeed < 0 {
                                view.source_y = border_bottom - view.source_h;
                            } else {
                                view.source_y +=
                                    (border_bottom - (view.source_y + view.source_h)).min(view.follow_vspeed);
                            }
                        }
                    } else {
                        view.source_y = y - view.source_h / 2;
                    }
                    view.source_y = view.source_y.max(0).min(self.room.height - view.source_h);
                }
            }
        }
    }

    /// Draws all instances, tiles and backgrounds to the screen, taking all active views into account.
    /// Note that this function runs GML code associated with object draw events, so its usage must match GameMaker 8.
    pub fn draw(&mut self) -> gml::Result<()> {
        self.renderer.set_interp_amount(if self.render_mode.interpolates() {
            (1.0 - self.interp_alpha).clamp(0.0, 1.0) as f32
        } else {
            0.0
        });

        // Update views that should be following objects
        if self.interp_update_views {
            if self.render_mode.interpolates() {
                self.prev_views = Some(self.room.views.clone());
            }
            self.update_views();
        }
        if self.room.views_enabled {
            self.renderer.clear_view(self.background_colour, 1.0);
        }

        let saved_views = if self.room.views_enabled && self.render_mode.interpolates() && self.interp_alpha < 1.0 {
            if let Some(prev) = &self.prev_views {
                let alpha = self.interp_alpha;
                let current = self.room.views.clone();
                for (view, prev_view) in self.room.views.iter_mut().zip(prev.iter()) {
                    if !view.visible || !prev_view.visible {
                        continue;
                    }
                    let lerp_i32 = |a: i32, b: i32| (a as f64 + (b as f64 - a as f64) * alpha).round() as i32;
                    view.source_x = lerp_i32(prev_view.source_x, view.source_x);
                    view.source_y = lerp_i32(prev_view.source_y, view.source_y);
                }
                Some(current)
            } else {
                None
            }
        } else {
            None
        };

        // Draw all views
        if self.room.views_enabled {
            // Iter views in a non-borrowing way
            let mut count = 0;
            while let Some(&view) = self.room.views.get(count) {
                if view.visible {
                    self.view_current = count;
                    self.draw_view(
                        view.source_x,
                        view.source_y,
                        view.source_w as _,
                        view.source_h as _,
                        view.port_x,
                        view.port_y,
                        view.port_w as _,
                        view.port_h as _,
                        view.angle.into(),
                    )?;
                }
                count += 1;
            }
            self.view_current = 0;
        } else {
            self.draw_view(0, 0, self.room.width, self.room.height, 0, 0, self.room.width, self.room.height, 0.0)?;
        }

        if let Some(views) = saved_views {
            self.room.views = views;
        }

        // Reset viewport
        self.renderer.set_view(
            0,
            0,
            self.unscaled_width as _,
            self.unscaled_height as _,
            0.0,
            0,
            0,
            self.unscaled_width as _,
            self.unscaled_height as _,
        );

        if self.show_fps {
            self.draw_fps_overlay();
        }

        // Apply room caption. The extra interpolation frames are not new game frames, so they must
        // not keep poking the window manager.
        if !self.interp_draw {
            let title = self.get_window_title();
            if self.play_type != PlayType::Record {
                self.window.set_title(title.as_ref());
            }
        }

        // Safety guarantee: Never leave secondary FBO/surface bound after draw completes
        self.renderer.reset_target();

        Ok(())
    }

    /// Draws the optional "FPS: N" label in the game's own font, at the top centre of the game
    /// framebuffer. Uses the dialogue font when the game has one so it matches the rest of the UI.
    fn draw_fps_overlay(&mut self) {
        let font_id = if self.assets.fonts.get_asset(2).is_some() {
            2
        } else if self.assets.fonts.get_asset(self.draw_font_id).is_some() {
            self.draw_font_id
        } else {
            -1
        };

        let old_font = self.draw_font_id;
        let old_colour = self.draw_colour;
        let old_alpha = self.draw_alpha;
        let old_halign = self.draw_halign;
        let old_valign = self.draw_valign;
        let old_depth = self.renderer.get_depth();
        let old_depth_test = self.renderer.get_depth_test();
        let old_write_depth = self.renderer.get_write_depth();
        let old_alpha_blending = self.renderer.get_alpha_blending();
        let old_blend_mode = self.renderer.get_blend_mode();

        self.draw_font_id = font_id;
        self.draw_halign = Halign::Middle;
        self.draw_valign = Valign::Top;
        self.draw_alpha = Real::from(1.0);
        self.renderer.set_draw_offset(0.0, 0.0);
        self.renderer.set_depth(-32000.0);
        self.renderer.set_depth_test(false);
        self.renderer.set_write_depth(false);
        self.renderer.set_alpha_blending(true);
        self.renderer.set_blend_mode(BlendType::SrcAlpha, BlendType::InvSrcAlpha);

        let text = gml::String::from(format!("FPS: {}", self.render_fps));
        let (text_w, text_h) = self.get_string_size(text.clone(), None, None);
        let center = self.unscaled_width as f64 / 2.0;
        let pad_x = 3.0;
        let pad_y = 2.0;
        let x1 = center - f64::from(text_w) / 2.0 - pad_x;
        let x2 = center + f64::from(text_w) / 2.0 + pad_x;
        let y1 = 1.0;
        let y2 = 2.0 + f64::from(text_h) + pad_y;
        self.renderer.draw_roundrect(x1, y1, x2, y2, 0, 0, 0.65, false);

        let x = Real::from(self.unscaled_width as i32 / 2);
        let y = Real::from(2);
        self.draw_colour = Colour::new(0.0, 0.0, 0.0);
        self.draw_string(
            x + Real::from(1.0),
            y + Real::from(1.0),
            text.clone(),
            None,
            None,
            Real::from(1.0),
            Real::from(1.0),
            Real::from(0.0),
            None,
            Real::from(1.0),
        );
        self.draw_colour = Colour::new(1.0, 1.0, 1.0);
        self.draw_string(
            x,
            y,
            text,
            None,
            None,
            Real::from(1.0),
            Real::from(1.0),
            Real::from(0.0),
            None,
            Real::from(1.0),
        );

        self.renderer.set_depth(old_depth);
        self.renderer.set_depth_test(old_depth_test);
        self.renderer.set_write_depth(old_write_depth);
        self.renderer.set_alpha_blending(old_alpha_blending);
        self.renderer.set_blend_mode(old_blend_mode.0, old_blend_mode.1);
        self.draw_font_id = old_font;
        self.draw_colour = old_colour;
        self.draw_alpha = old_alpha;
        self.draw_halign = old_halign;
        self.draw_valign = old_valign;
    }

    pub fn draw_instance_default(&mut self, idx: usize) -> gml::Result<()> {
        let instance = self.room.instance_list.get(idx);
        let sprite_idx = instance.sprite_index.get();
        if sprite_idx < 0 {
            return Ok(());
        }
        if let Some(sprite) = self.assets.sprites.get_asset(sprite_idx) {
            if let Some(atlas_ref) = sprite.get_atlas_ref(instance.image_index.get().floor().to_i32()) {
                self.renderer.draw_sprite(
                    atlas_ref,
                    instance.x.get().into(),
                    instance.y.get().into(),
                    instance.image_xscale.get().into(),
                    instance.image_yscale.get().into(),
                    instance.image_angle.get().into(),
                    instance.image_blend.get(),
                    instance.image_alpha.get().into(),
                );
            }
            Ok(())
        } else {
            Ok(())
        }
    }

    /// Draws everything in the scene using a given view rectangle
    fn draw_view(
        &mut self,
        src_x: i32,
        src_y: i32,
        src_w: i32,
        src_h: i32,
        port_x: i32,
        port_y: i32,
        port_w: i32,
        port_h: i32,
        angle: f64,
    ) -> gml::Result<()> {
        self.renderer.set_view(src_x, src_y, src_w, src_h, angle, port_x, port_y, port_w, port_h);

        if self.room.show_colour {
            self.renderer.clear_view(self.room.colour, 1.0);
        } else {
            self.renderer.clear_zbuf();
        }

        fn draw_instance(game: &mut Game, idx: usize, src_x: i32, src_y: i32, src_w: i32, src_h: i32) -> gml::Result<()> {
            let instance = game.room.instance_list.get(idx);
            if instance.visible.get() {
                let dx: f64 = (instance.xprevious.get() - instance.x.get()).into();
                let dy: f64 = (instance.yprevious.get() - instance.y.get()).into();
                game.renderer.set_draw_offset(dx, dy);

                let in_battle = matches!(game.room.id, 47 | 191 | 709);
                let result = if !game.custom_draw_objects.contains(&instance.object_index.get()) {
                    // Viewport Frustum Culling for default-drawn instances (+32px margin)
                    // In battle rooms (47, 191, 709), disable culling so aerial/fly moves and shadows never cut off
                    if !in_battle {
                        let ix = instance.x.get().into_inner() as i32;
                        let iy = instance.y.get().into_inner() as i32;
                        const MARGIN: i32 = 32;
                        if ix < src_x - MARGIN || ix > src_x + src_w + MARGIN || iy < src_y - MARGIN || iy > src_y + src_h + MARGIN {
                            game.renderer.set_draw_offset(0.0, 0.0);
                            return Ok(());
                        }
                    }
                    game.renderer.set_depth(instance.depth.get().into_inner() as f32);
                    if in_battle {
                        let t0 = std::time::Instant::now();
                        let _ = game.draw_instance_default(idx);
                        BATTLE_DRAW_ENTITIES_US.fetch_add(t0.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
                        Ok(())
                    } else {
                        let _ = game.draw_instance_default(idx);
                        Ok(())
                    }
                } else {
                    // Custom draw event: always execute as GML might draw HUD/GUI or global effects
                    game.renderer.set_depth(instance.depth.get().into_inner() as f32);
                    if in_battle {
                        let t0 = std::time::Instant::now();
                        let res = game.run_instance_event(gml::ev::DRAW, 0, idx, idx, None);
                        BATTLE_DRAW_HUD_US.fetch_add(t0.elapsed().as_micros() as u64, std::sync::atomic::Ordering::Relaxed);
                        res
                    } else {
                        game.run_instance_event(gml::ev::DRAW, 0, idx, idx, None)
                    }
                };
                game.renderer.set_draw_offset(0.0, 0.0);
                result
            } else {
                Ok(())
            }
        }

        fn draw_tile(game: &mut Game, idx: usize, src_x: i32, src_y: i32, src_w: i32, src_h: i32) {
            let tile = game.room.tile_list.get(idx);
            if !tile.visible.get() {
                return;
            }
            let tx = tile.x.get().into_inner() as i32;
            let ty = tile.y.get().into_inner() as i32;
            let tw = (tile.width.get() as f64 * tile.xscale.get().into_inner()).abs() as i32;
            let th = (tile.height.get() as f64 * tile.yscale.get().into_inner()).abs() as i32;
            if tx + tw < src_x - 32 || tx > src_x + src_w + 32 || ty + th < src_y - 32 || ty > src_y + src_h + 32 {
                return;
            }
            if let Some(Some(background)) = game.assets.backgrounds.get(tile.background_index.get() as usize) {
                if let Some(atlas) = background.atlas_ref {
                    game.renderer.set_depth(tile.depth.get().into_inner() as f32);
                    game.renderer.draw_sprite_partial(
                        atlas,
                        tile.tile_x.get().into(),
                        tile.tile_y.get().into(),
                        tile.width.get().into(),
                        tile.height.get().into(),
                        tile.x.get().into(),
                        tile.y.get().into(),
                        tile.xscale.get().into(),
                        tile.yscale.get().into(),
                        0.0,
                        tile.blend.get(),
                        tile.alpha.get().into(),
                    )
                }
            }
        }

        fn draw_part_syst(game: &mut Game, id: i32) {
            game.particles.draw_system(id, &mut game.renderer, &game.assets, true);
        }

        // Extent for drawing tiles in case view is rotated
        let tile_end_x = f64::from(src_x + src_w + src_h / 2);
        let tile_end_y = f64::from(src_y + src_h + src_w / 2);
    
        // draw backgrounds
        self.renderer.set_depth(12000.0);
        for background in self.room.backgrounds.iter().filter(|x| x.visible && !x.is_foreground) {
            if let Some(bg_asset) = self.assets.backgrounds.get_asset(background.background_id) {
                if let Some(atlas_ref) = bg_asset.atlas_ref {
                    self.renderer.draw_sprite_tiled(
                        atlas_ref,
                        background.x_offset.into(),
                        background.y_offset.into(),
                        background.xscale.into(),
                        background.yscale.into(),
                        background.blend,
                        background.alpha.into(),
                        background.tile_horizontal.then_some(tile_end_x),
                        background.tile_vertical.then_some(tile_end_y),
                    );
                }
            }
        }

        self.room.instance_list.draw_sort();
        let mut iter_inst = self.room.instance_list.iter_by_drawing();
        let mut iter_inst_v = iter_inst.next(&self.room.instance_list);
        // Tiles are pre-sorted at room load / modification. Iterate non-borrowing TLIterDrawOrder directly with zero per-frame allocations.
        let mut iter_tile = self.room.tile_list.iter_by_drawing();
        let mut iter_tile_v = iter_tile.next(&self.room.tile_list);
        self.particles.draw_sort();
        let mut iter_part = self.particles.iter_by_drawing();
        let mut iter_part_v = iter_part.next(&self.particles);
        // Draw depth-sorted interleaved scene with per-tile viewport frustum culling
        loop {
            match (iter_inst_v, iter_tile_v, iter_part_v) {
                (None, None, None) => break,
                (Some(idx_inst), None, None) => {
                    draw_instance(self, idx_inst, src_x, src_y, src_w, src_h)?;
                    while let Some(idx_inst) = iter_inst.next(&self.room.instance_list) {
                        draw_instance(self, idx_inst, src_x, src_y, src_w, src_h)?;
                    }
                    break
                },
                (None, Some(idx_tile), None) => {
                    draw_tile(self, idx_tile, src_x, src_y, src_w, src_h);
                    while let Some(idx_tile) = iter_tile.next(&self.room.tile_list) {
                        draw_tile(self, idx_tile, src_x, src_y, src_w, src_h);
                    }
                    break
                },
                (None, None, Some(idx_part)) => {
                    draw_part_syst(self, idx_part);
                    while let Some(idx_part) = iter_part.next(&self.particles) {
                        draw_part_syst(self, idx_part);
                    }
                    break
                },
                (idx_opt_inst, idx_opt_tile, idx_opt_part) => {
                    let inst_depth = idx_opt_inst.map(|h| self.room.instance_list.get(h).depth.get());
                    let tile_depth = idx_opt_tile.map(|h| self.room.tile_list.get(h).depth.get());
                    let part_depth = idx_opt_part.map(|h| self.particles.get_system(h).unwrap().depth);
                    if part_depth < inst_depth && part_depth < tile_depth {
                        if inst_depth < tile_depth {
                            draw_tile(self, idx_opt_tile.unwrap(), src_x, src_y, src_w, src_h);
                            iter_tile_v = iter_tile.next(&self.room.tile_list);
                        } else {
                            draw_instance(self, idx_opt_inst.unwrap(), src_x, src_y, src_w, src_h)?;
                            iter_inst_v = iter_inst.next(&self.room.instance_list);
                        }
                    } else {
                        draw_part_syst(self, idx_opt_part.unwrap());
                        iter_part_v = iter_part.next(&self.particles);
                    }
                },
            }
        }

        // draw foregrounds
        self.renderer.set_depth(-12000.0);
        for background in self.room.backgrounds.clone().iter().filter(|x| x.visible && x.is_foreground) {
            if let Some(bg_asset) = self.assets.backgrounds.get_asset(background.background_id) {
                if let Some(atlas_ref) = bg_asset.atlas_ref {
                    self.renderer.draw_sprite_tiled(
                        atlas_ref,
                        background.x_offset.into(),
                        background.y_offset.into(),
                        background.xscale.into(),
                        background.yscale.into(),
                        background.blend,
                        background.alpha.into(),
                        background.tile_horizontal.then_some(tile_end_x),
                        background.tile_vertical.then_some(tile_end_y),
                    );
                }
            }
        }

        self.renderer.set_depth(-13000.0);
        if let Some(sprite) = self.assets.sprites.get_asset(self.cursor_sprite) {
            let (x, y) = self.get_mouse_in_room();
            if let Some(atlas_ref) =
                sprite.get_atlas_ref((self.cursor_sprite_frame % sprite.frames.len() as u32) as i32)
            {
                self.renderer.draw_sprite(atlas_ref, x.into(), y.into(), 1.0, 1.0, 0.0, 0xffffff, 1.0);
            }
        }

        Ok(())
    }

    /// Splits the string into line-width pairs.
    fn split_string<'a>(&self, string: &'a gml::String, max_width: Option<i32>, font: &'a Font) -> LineIterator<'a> {
        let encoded_text: std::borrow::Cow<'a, [u8]> = match self.gm_version {
            Version::GameMaker8_0 => std::borrow::Cow::Borrowed(string.as_ref()),
            Version::GameMaker8_1 => {
                let encoding = font.get_encoding(self.encoding);
                let decoded = string.decode_utf8();
                let (encoded_text, _, bad) = encoding.encode(decoded.as_ref());
                if bad {
                    let mut encoded_vec = Vec::with_capacity(encoded_text.as_ref().len());
                    let mut iter = decoded.char_indices().peekable();
                    while let Some((pos, _)) = iter.next() {
                        let slice = iter.peek().map(|(p, _)| &decoded[pos..*p]).unwrap_or(&decoded[pos..]);
                        let (encoded_char, _, bad) = encoding.encode(slice);
                        if bad {
                            encoded_vec.push(b'?');
                        } else {
                            encoded_vec.extend_from_slice(encoded_char.as_ref());
                        }
                    }
                    std::borrow::Cow::Owned(encoded_vec)
                } else {
                    std::borrow::Cow::Owned(encoded_text.into_owned())
                }
            },
        };
        LineIterator {
            text: encoded_text,
            pos: 0,
            font,
            max_width,
            word_buf: [0u8; 64],
            word_len: 0,
            word_width: 0,
            line_buf: [0u8; 256],
            line_len: 0,
        }
    }

    /// Gets width and height of a string using the current draw_font.
    /// If line_height is None, a line height will be inferred from the font.
    /// If max_width is None, the string will not be given a maximum width.
    pub fn get_string_size(&self, string: gml::String, line_height: Option<i32>, max_width: Option<i32>) -> (i32, i32) {
        let font = self.assets.fonts.get_asset(self.draw_font_id).map(|x| x.as_ref()).unwrap_or(&self.default_font);

        // Figure out what the height of a line is if one wasn't specified
        let line_height = match line_height {
            Some(h) => h,
            None => font.tallest_char_height as i32,
        };

        let bytes = string.as_ref();
        if max_width.is_none() && !bytes.iter().any(|&b| b == b'#' || b == b'\n' || b == b'\r') {
            let mut width = 0;
            for &c in bytes {
                if let Some(character) = font.get_char(c) {
                    width += character.offset;
                } else if let Some(character) = font.get_char(font.first) {
                    width += character.offset;
                }
            }
            return (width, line_height);
        }

        let mut width = 0;
        let mut line_count = 0;
        let mut iter = self.split_string(&string, max_width, font);
        while let Some((_, current_w)) = iter.next() {
            if width < current_w {
                width = current_w;
            }
            line_count += 1;
        }

        (width, line_count * line_height)
    }

    /// Draws a string to the screen at the given coordinates.
    /// If line_height is None, a line height will be inferred from the font.
    /// If max_width is None, the string will not be given a maximum width.
    pub fn draw_string(
        &mut self,
        x: Real,
        y: Real,
        string: gml::String,
        line_height: Option<i32>,
        max_width: Option<i32>,
        xscale: Real,
        yscale: Real,
        angle: Real,
        colours: Option<(i32, i32, i32, i32)>,
        alpha: Real,
    ) {
        #[cfg(target_os = "android")]
        {
            let bytes = string.as_ref();
            if self.dialogue_open() || (bytes.len() >= 12 && bytes.contains(&b' ')) {
                let text = self.decode_str(bytes);
                crate::touch_controls::record_dialogue(&text);
            }
        }

        let font = self.assets.fonts.get_asset(self.draw_font_id).map(|x| x.as_ref()).unwrap_or(&self.default_font);

        let is_straight = angle == Real::from(0.0);
        let (sin, cos) = if is_straight {
            (0.0f64, 1.0f64)
        } else {
            (angle.to_radians().sin().into_inner(), angle.to_radians().cos().into_inner())
        };

        // Figure out what the height of a line is if one wasn't specified
        let line_height = match line_height {
            Some(h) => h,
            None => font.tallest_char_height as i32,
        };

        let str_h = match self.draw_valign {
            Valign::Top => 0,
            _ => self.get_string_size(string.clone(), Some(line_height), max_width).1,
        };
        let mut cursor_y = match self.draw_valign {
            Valign::Top => 0,
            Valign::Middle => -(str_h / 2),
            Valign::Bottom => -str_h,
        };

        let uniform_colour = match colours {
            Some((c1, c2, c3, c4)) if c1 == c2 && c1 == c3 && c1 == c4 => Some(c1),
            None => Some(u32::from(self.draw_colour) as i32),
            _ => None,
        };

        let bytes = string.as_ref();
        if max_width.is_none() && !bytes.iter().any(|&b| b == b'#' || b == b'\n' || b == b'\r') {
            let mut width = 0;
            for &c in bytes {
                if let Some(character) = font.get_char(c) {
                    width += character.offset;
                } else if let Some(character) = font.get_char(font.first) {
                    width += character.offset;
                }
            }

            let left_offset = match self.draw_halign {
                Halign::Left => 0,
                Halign::Middle => -(width / 2),
                Halign::Right => -width,
            };
            let mut cursor_x = left_offset;

            if is_straight {
                if let Some(col) = uniform_colour {
                    let y_pos = y.into_inner() + cursor_y as f64 * yscale.into_inner();
                    let x_base = x.into_inner();
                    let xs_f: f64 = xscale.into_inner();
                    let ys_f: f64 = yscale.into_inner();
                    let alpha_f: f64 = alpha.into_inner();

                    for &c in bytes {
                        let character = match font.get_char(c) {
                            Some(character) => character,
                            None => {
                                if let Some(character) = font.get_char(font.first) {
                                    cursor_x += character.offset as i32;
                                }
                                continue;
                            }
                        };

                        let px = x_base + (character.distance as i32 + cursor_x) as f64 * xs_f;
                        self.renderer.draw_sprite(
                            character.atlas_ref,
                            px,
                            y_pos,
                            xs_f,
                            ys_f,
                            0.0,
                            col,
                            alpha_f,
                        );

                        cursor_x += character.offset as i32;
                    }
                    return;
                }
            }
        }

        fn lerp_col(c1: i32, c2: i32, ratio: f64) -> i32 {
            ((f64::from(c1 & 0xff) * (1.0 - ratio) + f64::from(c2 & 0xff) * ratio) as i32 & 0xff)
                + ((f64::from(c1 & 0xff00) * (1.0 - ratio) + f64::from(c2 & 0xff00) * ratio) as i32 & 0xff00)
                + ((f64::from(c1 & 0xff0000) * (1.0 - ratio) + f64::from(c2 & 0xff0000) * ratio) as i32 & 0xff0000)
        }

        let mut iter = self.split_string(&string, max_width, font);
        while let Some((line, width)) = iter.next() {
            let left_offset = match self.draw_halign {
                Halign::Left => 0,
                Halign::Middle => -(width as i32 / 2),
                Halign::Right => -width as i32,
            };
            let mut cursor_x = left_offset;

            for &c in line.iter() {
                let character = match font.get_char(c) {
                    Some(character) => character,
                    None => {
                        // Space if it isn't in the font
                        if let Some(character) = font.get_char(font.first) {
                            cursor_x += character.offset as i32;
                        }
                        continue;
                    },
                };

                let raw_xdiff = (character.distance as i32 + cursor_x) as f64 * xscale.into_inner();
                let raw_ydiff = cursor_y as f64 * yscale.into_inner();

                let (px, py) = if is_straight {
                    (x.into_inner() + raw_xdiff, y.into_inner() + raw_ydiff)
                } else {
                    (
                        x.into_inner() + raw_xdiff * cos + raw_ydiff * sin,
                        y.into_inner() + raw_ydiff * cos - raw_xdiff * sin,
                    )
                };

                if let Some(col) = uniform_colour {
                    // Fast-path: no trigonometric rotation & no per-vertex color blending
                    self.renderer.draw_sprite(
                        character.atlas_ref,
                        px,
                        py,
                        xscale.into(),
                        yscale.into(),
                        angle.into(),
                        col,
                        alpha.into(),
                    );
                } else if let Some((c1, c2, c3, c4)) = colours {
                    // Full-path: four-corner color gradient interpolation
                    self.renderer.draw_sprite_colour(
                        character.atlas_ref,
                        px,
                        py,
                        xscale.into(),
                        yscale.into(),
                        angle.into(),
                        lerp_col(c1, c2, f64::from(cursor_x - left_offset) / f64::from(width)),
                        lerp_col(c1, c2, f64::from(cursor_x - left_offset + character.offset) / f64::from(width)),
                        lerp_col(c4, c3, f64::from(cursor_x - left_offset + character.offset) / f64::from(width)),
                        lerp_col(c4, c3, f64::from(cursor_x - left_offset) / f64::from(width)),
                        alpha.into(),
                    );
                }

                cursor_x += character.offset as i32;
            }

            cursor_y += line_height;
        }
    }
}
