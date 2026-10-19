//! The information windows the game opens through `show_info` and `show_image`.
//!
//! GM8 draws these as real dialogs and stops the game until the player closes them. This port has no
//! system dialog (and Android has none either), so the window is drawn with the game renderer over a
//! dimmed backdrop, and the frame loop is spun here until the player closes it. That keeps the game's
//! own sequence - it loads a help text, shows it, and only then shows the picture - intact.
//!
//! The help texts are Word RTF: a colour table, colour changes in the text, paragraphs. The window
//! parses that into coloured runs, wraps and centres them the way the original information window
//! does, and draws them with the game's font. Text is converted to the game's ANSI page first, since
//! the font is looked up by byte.

use crate::{
    gml,
    math::Real,
    render::atlas::AtlasRef,
};

/// What one of the windows is showing.
pub(crate) enum InfoModal {
    Text(RichDoc),
    Image { sprite: AtlasRef, width: u32, height: u32 },
}

/// A parsed help text: paragraphs made of coloured runs.
pub(crate) struct RichDoc {
    paragraphs: Vec<Vec<(String, i32)>>,
}

/// A colour the way GM8 packs it: red in the low byte, then green, then blue.
const fn rgb(r: i32, g: i32, b: i32) -> i32 {
    r | (g << 8) | (b << 16)
}

const DIM: i32 = rgb(0, 0, 0);
const FRAME: i32 = rgb(120, 200, 140);
pub(crate) const INK: i32 = rgb(235, 235, 235);
/// The help text is drawn at the font's own size and then presented with the game, so it keeps the
/// game's own (big) look on every screen.
const SCALE: f64 = 1.0;

impl super::Game {
    /// Shows `modal` and waits for the player to close it, redrawing it every frame.
    pub(crate) fn show_info_modal(&mut self, modal: InfoModal) -> gml::Result<()> {
        let (ww, wh) = self.window_inner_size;
        if ww == 0 || wh == 0 {
            return Ok(())
        }
        let (fb_w, fb_h) = self.renderer.stored_size();
        let (old_h, old_v) = (self.draw_halign, self.draw_valign);
        self.draw_halign = super::draw::Halign::Left;
        self.draw_valign = super::draw::Valign::Top;
        let old_font = self.draw_font_id;
        self.draw_font_id = self.info_font_id();
        let image = matches!(&modal, InfoModal::Image { .. });
        // Pictures are drawn at window resolution and blitted: from the game's small framebuffer they
        // came out tiny or blurry. The help text keeps the game's own presentation, which is bigger
        // and reads much better.
        let surface = if image {
            match self.info_surface {
                Some((sw, sh, surface)) if (sw, sh) == (ww, wh) => Some(surface),
                old => {
                    if let Some((_, _, previous)) = old {
                        self.renderer.delete_sprite(previous);
                    }
                    match self.renderer.create_surface(ww as i32, wh as i32, false) {
                        Ok(surface) => {
                            self.info_surface = Some((ww, wh, surface));
                            Some(surface)
                        },
                        Err(_) => None,
                    }
                },
            }
        } else {
            None
        };
        if let Some(font) = self.assets.fonts.get(self.draw_font_id as usize).and_then(|f| f.as_deref()) {
            eprintln!("[info] fuente ventana id={} size={} tallest={}", self.draw_font_id, font.size, font.tallest_char_height);
        }
        let mut scroll = 0i32;
        // The on-screen controls step aside while this page is up.
        crate::touch_controls::set_info_window(true);
        let mut drag_last: Option<i32> = None;
        let mut drag_accum = 0i32;
        let mut dragging = false;
        loop {
            self.process_window_events();
            if self.close_requested {
                break
            }
            let mut close = self.input.keyboard_check_pressed(super::input::Button::Z as u8)
                || self.input.keyboard_check_pressed(super::input::Button::X as u8)
                || self.input.keyboard_check_pressed(super::input::Button::Return as u8)
                || self.input.keyboard_check_pressed(super::input::Button::Escape as u8)
                || self.input.keyboard_check_pressed(super::input::Button::Space as u8);
            if self.input.keyboard_check_pressed(super::input::Button::DownArrow as u8)
                || self.input.mouse_wheel_down()
            {
                scroll += 1;
            }
            if self.input.keyboard_check_pressed(super::input::Button::UpArrow as u8) || self.input.mouse_wheel_up() {
                scroll = (scroll - 1).max(0);
            }
            // Touch has no wheel: dragging the page scrolls it, and a tap (with no drag) closes it.
            // Drag distance is measured in window pixels and turned into whole lines.
            const DRAG_STEP: i32 = 40;
            const DRAG_START: i32 = 8;
            let mouse_y = self.input.mouse_y();
            if self.input.mouse_check_button_pressed(1) {
                drag_last = Some(mouse_y);
                drag_accum = 0;
                dragging = false;
            }
            if self.input.mouse_check_button(1) {
                if let Some(last) = drag_last {
                    drag_accum += mouse_y - last;
                    drag_last = Some(mouse_y);
                    if drag_accum.abs() >= DRAG_START {
                        dragging = true;
                    }
                    while drag_accum <= -DRAG_STEP {
                        scroll += 1;
                        drag_accum += DRAG_STEP;
                    }
                    while drag_accum >= DRAG_STEP {
                        scroll = (scroll - 1).max(0);
                        drag_accum -= DRAG_STEP;
                    }
                }
            }
            if self.input.mouse_check_button_released(1) {
                if drag_last.is_some() && !dragging {
                    close = true;
                }
                drag_last = None;
                drag_accum = 0;
                dragging = false;
            }

            match &modal {
                InfoModal::Image { sprite, width, height } => {
                    if let Some(surface) = surface {
                        self.renderer.set_target(surface);
                        self.renderer.clear_view((0.0, 0.0, 0.0).into(), 1.0);
                        self.paint_image_window(*sprite, *width, *height, f64::from(ww), f64::from(wh));
                        self.renderer.reset_target();
                        self.renderer.present_surface(surface, ww, wh);
                    }
                },
                InfoModal::Text(doc) => {
                    // The game's own framebuffer, so the text keeps the size it always had.
                    self.renderer.set_view(0, 0, fb_w as i32, fb_h as i32, 0.0, 0, 0, fb_w as i32, fb_h as i32);
                    self.renderer.draw_rectangle(0.0, 0.0, f64::from(fb_w), f64::from(fb_h), DIM, 1.0);
                    let max_scroll = self.paint_text_window(doc, f64::from(fb_w), f64::from(fb_h), &mut scroll);
                    scroll = scroll.clamp(0, max_scroll);
                    self.renderer.present(ww, wh, self.scaling);
                },
            }
            self.input.step();
            if close {
                break
            }
        }
        crate::touch_controls::set_info_window(false);
        self.input.step();
        self.draw_halign = old_h;
        self.draw_valign = old_v;
        self.draw_font_id = old_font;
        Ok(())
    }

    /// The smallest game font that is still readable, used for the help text. The room's font is
    /// usually a big display face, and scaling it down breaks the glyphs.
    fn info_font_id(&self) -> i32 {
        let current = self
            .assets
            .fonts
            .get(self.draw_font_id as usize)
            .and_then(|f| f.as_deref())
            .map(|f| f.tallest_char_height as i32)
            .unwrap_or(0);
        if (8..=20).contains(&current) {
            return self.draw_font_id
        }
        let mut best = self.draw_font_id;
        let mut best_h = i32::MAX;
        for (index, font) in self.assets.fonts.iter().enumerate() {
            if let Some(font) = font {
                let height = font.tallest_char_height as i32;
                if (8..=20).contains(&height) && height < best_h {
                    best_h = height;
                    best = index as i32;
                }
            }
        }
        best
    }

    /// Builds the string the way the renderer will read it.
    ///
    /// GM8.1 stores strings as UTF-8 and `split_string` re-encodes them with the font's own page, so
    /// it expects UTF-8 from us. GM8.0 takes the bytes as they are, so they must already be in the
    /// font's page. Getting this wrong turns every accent into extra glyphs, and the advances stop
    /// matching the widths, so words overlap.
    pub(crate) fn ansi_string(&self, text: &str) -> gml::String {
        match self.gm_version {
            super::Version::GameMaker8_1 => gml::String::from(text),
            _ => {
                // GM8.0 keeps strings as raw ANSI bytes, and this game's data - like its font
                // glyphs - is cp1252. The font resource declares the Shift_JIS charset, but that is
                // only metadata: encoding to it turns every accent into an `&#N;` reference.
                let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(text);
                gml::String::from(bytes.into_owned())
            },
        }
    }

    fn text_width(&self, text: &str) -> i32 {
        (f64::from(self.get_string_size(self.ansi_string(text), None, None).0) * SCALE).round() as i32
    }

    /// Height the current font needs for one line, at the size the window draws it.
    fn info_line_height(&self) -> i32 {
        ((f64::from(self.get_string_size(self.ansi_string("Ag"), None, None).1) * SCALE).round() as i32).max(1)
    }

    /// The help uses the whole window. The OK button is small enough to sit over the last corner
    /// without a reserved strip, so no line is hidden behind it.
    fn info_frame(&mut self, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let margin = (w * 0.02).max(10.0);
        (margin, margin, w - margin, h - margin)
    }

    fn draw_label(&mut self, text: &str, cx: f64, cy: f64, colour: i32) {
        let string = self.ansi_string(text);
        let sw = self.text_width(text);
        let sh = self.info_line_height();
        self.draw_string(
            Real::from(cx - f64::from(sw) / 2.0),
            Real::from(cy - f64::from(sh) / 2.0),
            string,
            None,
            None,
            Real::from(SCALE),
            Real::from(SCALE),
            Real::from(0.0),
            Some((colour, colour, colour, colour)),
            Real::from(1.0),
        );
    }

    fn draw_span(&mut self, text: &str, x: f64, y: f64, colour: i32) {
        self.draw_string(
            Real::from(x),
            Real::from(y),
            self.ansi_string(text),
            None,
            None,
            Real::from(SCALE),
            Real::from(SCALE),
            Real::from(0.0),
            Some((colour, colour, colour, colour)),
            Real::from(1.0),
        );
    }

    /// Text window: wraps and centres the coloured paragraphs, scrolled by `scroll` lines. Returns
    /// the maximum scroll so the caller can clamp.
    fn paint_text_window(&mut self, doc: &RichDoc, w: f64, h: f64, scroll: &mut i32) -> i32 {
        let (left, top, right, bottom) = self.info_frame(w, h);
        let max_w = (right - left).max(40.0);
        let line_h = self.info_line_height();
        let lines = self.wrap_doc(doc, max_w as i32);
        let visible = (((bottom - top) / f64::from(line_h)).floor() as i32).max(1);
        let max_scroll = (lines.len() as i32 - visible).max(0);
        *scroll = (*scroll).clamp(0, max_scroll);
        for (row, line) in lines.iter().skip(*scroll as usize).take(visible as usize).enumerate() {
            let y = top + f64::from(row as i32 * line_h);
            let total: i32 = line
                .iter()
                .map(|(text, _, spaced)| self.text_width(text) + if *spaced { self.space_width() } else { 0 })
                .sum();
            let mut x = left + ((right - left) - f64::from(total)) / 2.0;
            for (text, colour, spaced) in line.iter() {
                if *spaced {
                    x += f64::from(self.space_width());
                }
                self.draw_span(text, x, y, *colour);
                x += f64::from(self.text_width(text));
            }
        }
        // Only the OK button is left over the page.
        self.paint_ok_button(w, h);
        max_scroll
    }

    fn space_width(&self) -> i32 {
        self.text_width(" ")
    }

    /// Image window: scales the picture to fit the content area, keeping its shape.
    fn paint_image_window(&mut self, sprite: AtlasRef, width: u32, height: u32, w: f64, h: f64) {
        let (left, top, right, bottom) = self.info_frame(w, h);
        let (avail_w, avail_h) = (right - left, bottom - top);
        // Like the original's image window: the picture fills it. It is drawn on a window-sized
        // surface now, so this upscale is the only one (the old one also scaled the whole 320x180
        // framebuffer up, which is what blurred it). Nearest keeps the pixels sharp.
        let scale = (avail_w / f64::from(width.max(1))).min(avail_h / f64::from(height.max(1))).max(0.01);
        let (dw, dh) = (f64::from(width) * scale, f64::from(height) * scale);
        let (x, y) = (left + (right - left - dw) / 2.0, top + (bottom - top - dh) / 2.0);
        let old_interp = self.renderer.get_pixel_interpolation();
        self.renderer.set_pixel_interpolation(false);
        self.renderer.draw_sprite(sprite, x, y, scale, scale, 0.0, -1, 1.0);
        self.renderer.set_pixel_interpolation(old_interp);
        self.paint_ok_button(w, h);
    }

    fn paint_ok_button(&mut self, w: f64, h: f64) {
        let (x2, y2) = (w - (w * 0.02).max(10.0), h - (h * 0.02).max(10.0));
        let (bw, bh) = ((w * 0.06).max(64.0), (h * 0.030).max(22.0));
        let (bx2, by2) = (x2 - 6.0, y2 - 6.0);
        let (bx1, by1) = (bx2 - bw, by2 - bh);
        self.renderer.draw_rectangle(bx1, by1, bx2, by2, rgb(18, 18, 28), 1.0);
        self.renderer.draw_rectangle_outline(bx1, by1, bx2, by2, FRAME, 1.0);
        self.draw_label("OK", (bx1 + bx2) / 2.0, (by1 + by2) / 2.0, INK);
    }

    /// Wraps each paragraph into lines of (word, colour, preceded by a space) spans, keeping its own
    /// line breaks. A word longer than the panel - a URL, usually - is cut into pieces so it cannot
    /// run off the sides.
    fn wrap_doc(&self, doc: &RichDoc, max_width: i32) -> Vec<Vec<(String, i32, bool)>> {
        let mut lines: Vec<Vec<(String, i32, bool)>> = Vec::new();
        let space = self.space_width();
        for paragraph in &doc.paragraphs {
            let mut line: Vec<(String, i32, bool)> = Vec::new();
            let mut line_w = 0;
            for (text, colour) in paragraph {
                for (index, chunk) in self.split_to_fit(text, max_width).into_iter().enumerate() {
                    let w = self.text_width(&chunk);
                    if index > 0 {
                        // Continuation of a long word: a fresh line, with no space before it.
                        if !line.is_empty() {
                            lines.push(std::mem::take(&mut line));
                            line_w = 0;
                        }
                        line.push((chunk, *colour, false));
                        line_w = w;
                        continue
                    }
                    let needed = if line.is_empty() { w } else { line_w + space + w };
                    if needed > max_width && !line.is_empty() {
                        lines.push(std::mem::take(&mut line));
                        line_w = 0;
                    }
                    let spaced = !line.is_empty();
                    line_w += w + if spaced { space } else { 0 };
                    line.push((chunk, *colour, spaced));
                }
            }
            lines.push(line);
        }
        lines
    }

    /// Cuts `word` into pieces no wider than `max_width`, so a long URL wraps instead of overflowing.
    fn split_to_fit(&self, word: &str, max_width: i32) -> Vec<String> {
        if self.text_width(word) <= max_width {
            return vec![word.to_string()]
        }
        let mut chunks = Vec::new();
        let mut chunk = String::new();
        for c in word.chars() {
            let mut candidate = chunk.clone();
            candidate.push(c);
            if self.text_width(&candidate) > max_width && !chunk.is_empty() {
                chunks.push(std::mem::take(&mut chunk));
                chunk.push(c);
            } else {
                chunk = candidate;
            }
        }
        if !chunk.is_empty() {
            chunks.push(chunk);
        }
        if chunks.is_empty() {
            chunks.push(word.to_string());
        }
        chunks
    }
}

/// Parses the RTF the game loads into coloured paragraphs. Handles the colour table, `\cfN`,
/// paragraphs, tabs and the accented-byte escapes; metadata groups are ignored.
pub(crate) fn parse_rtf(rtf: &str, auto_colour: i32) -> RichDoc {
    let chars: Vec<char> = rtf.chars().collect();
    let palette = parse_colour_table(&chars, auto_colour);
    let mut paragraphs: Vec<Vec<(String, i32)>> = Vec::new();
    let mut paragraph: Vec<(String, i32)> = Vec::new();
    let mut buffer = String::new();
    let mut colour = auto_colour;
    let mut skips: Vec<bool> = Vec::new();
    let skipping = |skips: &Vec<bool>| skips.iter().any(|&s| s);

    let flush = |buffer: &mut String, colour: i32, paragraph: &mut Vec<(String, i32)>| {
        if !buffer.is_empty() {
            paragraph.push((std::mem::take(buffer), colour));
        }
    };

    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '{' => {
                let rest: String = chars[i..(i + 14).min(chars.len())].iter().collect();
                let metadata = rest.starts_with("{\\fonttbl")
                    || rest.starts_with("{\\colortbl")
                    || rest.starts_with("{\\stylesheet")
                    || rest.starts_with("{\\info")
                    || rest.starts_with("{\\*")
                    || rest.starts_with("{\\pict")
                    || rest.starts_with("{\\header")
                    || rest.starts_with("{\\footer");
                skips.push(metadata);
                i += 1;
            },
            '}' => {
                skips.pop();
                i += 1;
            },
            '\\' => {
                if i + 3 < chars.len() && chars[i + 1] == '\'' {
                    let hex: String = chars[i + 2..=i + 3].iter().collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        if !skipping(&skips) {
                            buffer.push(char::from_u32(u32::from(byte)).unwrap_or('?'));
                        }
                    }
                    i += 4;
                    continue
                }
                if i + 1 < chars.len() && (chars[i + 1] == '\\' || chars[i + 1] == '{' || chars[i + 1] == '}') {
                    if !skipping(&skips) {
                        buffer.push(chars[i + 1]);
                    }
                    i += 2;
                    continue
                }
                let mut j = i + 1;
                let mut word = String::new();
                while j < chars.len() && chars[j].is_ascii_alphabetic() {
                    word.push(chars[j]);
                    j += 1;
                }
                let mut param = String::new();
                if j < chars.len() && chars[j] == '-' {
                    param.push('-');
                    j += 1;
                }
                while j < chars.len() && chars[j].is_ascii_digit() {
                    param.push(chars[j]);
                    j += 1;
                }
                if j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                if !skipping(&skips) {
                    match word.as_str() {
                        "par" | "line" => {
                            flush(&mut buffer, colour, &mut paragraph);
                            paragraphs.push(std::mem::take(&mut paragraph));
                        },
                        "tab" => buffer.push(' '),
                        "cf" => {
                            flush(&mut buffer, colour, &mut paragraph);
                            colour = param
                                .parse::<usize>()
                                .ok()
                                .and_then(|n| palette.get(n).copied())
                                .unwrap_or(auto_colour);
                        },
                        "u" => {
                            if let Ok(n) = param.parse::<i32>() {
                                let code = if n < 0 { n + 65536 } else { n };
                                if let Some(c) = char::from_u32(code as u32) {
                                    buffer.push(c);
                                }
                            }
                            if j < chars.len() && chars[j] == '?' {
                                j += 1;
                            }
                        },
                        _ => {},
                    }
                }
                i = j;
            },
            '\r' => i += 1,
            '\n' => i += 1,
            c => {
                if !skipping(&skips) {
                    buffer.push(c);
                }
                i += 1;
            },
        }
    }
    flush(&mut buffer, colour, &mut paragraph);
    if !paragraph.is_empty() {
        paragraphs.push(paragraph);
    }

    // Split each paragraph into words. The split walks characters, not runs: the game's text often
    // changes colour in the middle of a word, and splitting run by run would break that word in two.
    // A word keeps the colour of the run its first character came from.
    let paragraphs = paragraphs
        .into_iter()
        .map(|runs| {
            let mut words = Vec::new();
            let mut word = String::new();
            let mut word_colour = auto_colour;
            for (text, colour) in runs {
                for c in text.chars() {
                    if c.is_whitespace() {
                        if !word.is_empty() {
                            words.push((std::mem::take(&mut word), word_colour));
                        }
                    } else {
                        if word.is_empty() {
                            word_colour = colour;
                        }
                        word.push(c);
                    }
                }
            }
            if !word.is_empty() {
                words.push((word, word_colour));
            }
            words
        })
        .collect();
    RichDoc { paragraphs }
}

/// Reads the `\colortbl` group into a palette of GM8 colours; index 0 is the automatic colour.
fn parse_colour_table(chars: &[char], auto_colour: i32) -> Vec<i32> {
    let mut palette = vec![auto_colour];
    let Some(at) = find_sequence(chars, "\\colortbl") else { return palette };
    // The group runs from the '{' before the word to its matching '}'.
    let mut group_start = at;
    while group_start > 0 && chars[group_start] != '{' {
        group_start -= 1;
    }
    let mut depth = 0usize;
    let mut group_end = group_start;
    while group_end < chars.len() {
        match chars[group_end] {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break
                }
            },
            _ => {},
        }
        group_end += 1;
    }
    // Entries are separated by ';'. The first one is empty and stands for the automatic colour,
    // which `palette` already holds, so it is skipped.
    let body: String = chars[at..group_end.min(chars.len())].iter().collect();
    for entry in body.split(';').skip(1) {
        let mut r = 0;
        let mut g = 0;
        let mut b = 0;
        let bytes: Vec<char> = entry.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == '\\' {
                let mut j = i + 1;
                let mut word = String::new();
                while j < bytes.len() && bytes[j].is_ascii_alphabetic() {
                    word.push(bytes[j]);
                    j += 1;
                }
                let mut param = String::new();
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    param.push(bytes[j]);
                    j += 1;
                }
                let value = param.parse::<i32>().unwrap_or(0);
                match word.as_str() {
                    "red" => r = value,
                    "green" => g = value,
                    "blue" => b = value,
                    _ => {},
                }
                i = j;
                continue
            }
            i += 1;
        }
        palette.push(rgb(r, g, b));
    }
    palette
}

fn find_sequence(chars: &[char], needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (0..chars.len().saturating_sub(needle.len()))
        .find(|&start| chars[start..start + needle.len()] == needle[..])
}
