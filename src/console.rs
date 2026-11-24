//! Graphical Corvus console: CORVUS header, optional bounded logo animation,
//! a large HELP button and a bitmap text terminal mirroring the serial shell.

use crate::frames::Frames;
use crate::ramfb::{rgb, Framebuffer, Scaled, HEIGHT, WIDTH};
use embedded_graphics::mono_font::{ascii::FONT_10X20, MonoTextStyle};
use embedded_graphics::prelude::*;
use embedded_graphics::text::{Baseline, Text};

pub const PALETTE: [u32; 16] = [
    0x0c0a10, 0x364a8c, 0x377d55, 0x3b8793, 0xa34555, 0x5b3f8c, 0xa87840, 0xc4becf,
    0x70677e, 0x809bef, 0x8bd19c, 0x8adce5, 0xef8792, 0xbda0ed, 0xffd166, 0xf5f1ff,
];
const BACKGROUND: u32 = 0x0c0a10;
const PURPLE: u32 = 0x5b3f8c;
const PURPLE_DARK: u32 = 0x2a1f40;
const GOLD: u32 = 0xffd166;
const TEXT: u32 = 0xe8e4f0;
const MUTED: u32 = 0x8a80a0;

const GLYPH_W: usize = 10;
const GLYPH_H: usize = 20;
const TERM_X: usize = 16;
const TERM_Y: usize = 156;
const COLS: usize = (WIDTH - 2 * TERM_X) / GLYPH_W;
const ROWS: usize = (HEIGHT - TERM_Y - 8) / GLYPH_H;

pub const LOGO_X: usize = 16;
pub const LOGO_Y: usize = 12;

/// The large on-screen HELP control, in framebuffer pixels.
pub const HELP_BUTTON: (usize, usize, usize, usize) = (572, 16, 212, 120);

#[derive(Clone, Copy, PartialEq)]
enum Escape {
    None,
    Start,
    Params,
}

pub struct Console {
    fb: Framebuffer,
    cells: [[u8; COLS]; ROWS],
    colors: [[u32; COLS]; ROWS],
    row: usize,
    col: usize,
    color: u32,
    escape: Escape,
    param: u32,
    pub has_logo: bool,
    pub logo_requested: bool,
}

impl Console {
    pub fn new() -> Console {
        Console {
            fb: Framebuffer,
            cells: [[b' '; COLS]; ROWS],
            colors: [[TEXT; COLS]; ROWS],
            row: 0,
            col: 0,
            color: TEXT,
            escape: Escape::None,
            param: 0,
            has_logo: false,
            logo_requested: false,
        }
    }

    pub fn draw_chrome(&mut self, logo: Option<&Frames>) {
        self.fb.fill(0, 0, WIDTH, HEIGHT, BACKGROUND);
        let title_x = match logo {
            Some(frames) => {
                self.has_logo = true;
                frames.draw(0, &mut self.fb, LOGO_X, LOGO_Y);
                LOGO_X + frames.width + 20
            }
            None => 24,
        };
        self.big_text("CORVUS", title_x, 14, 4, GOLD);
        let style = MonoTextStyle::new(&FONT_10X20, rgb(MUTED));
        let _ = Text::with_baseline("OS / ARM64 / type or tap", Point::new(title_x as i32 + 4, 106), style, Baseline::Top)
            .draw(&mut self.fb);
        self.fb.fill(0, 146, WIDTH, 2, PURPLE);
        self.draw_help_button(false);
        self.redraw_cursor(true);
    }

    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let color = if color < 16 { PALETTE[color as usize] } else { color };
        self.fb.fill(x, y, w, h, color);
    }

    pub fn large_banner(&mut self, text: &str) {
        if self.row + 3 >= ROWS { self.newline(); self.newline(); self.newline(); }
        self.big_text(text, TERM_X, TERM_Y + self.row * GLYPH_H, 2, self.color);
        self.row = (self.row + 3).min(ROWS - 1); self.col = 0;
    }

    pub fn frame(&mut self, frames: &Frames, index: usize) {
        frames.draw(index, &mut self.fb, LOGO_X, LOGO_Y);
    }

    fn big_text(&mut self, text: &str, x: usize, y: usize, scale: u32, color: u32) {
        let style = MonoTextStyle::new(&FONT_10X20, rgb(color));
        let mut target = Scaled { inner: &mut self.fb, scale, origin: Point::new(x as i32, y as i32) };
        let _ = Text::with_baseline(text, Point::zero(), style, Baseline::Top).draw(&mut target);
    }

    pub fn draw_help_button(&mut self, pressed: bool) {
        let (x, y, w, h) = HELP_BUTTON;
        let (fill, label) = if pressed { (GOLD, PURPLE_DARK) } else { (PURPLE, GOLD) };
        self.fb.fill(x, y, w, h, GOLD);
        self.fb.fill(x + 4, y + 4, w - 8, h - 8, fill);
        let scale = 3;
        let text_w = 4 * GLYPH_W * scale;
        let text_h = GLYPH_H * scale;
        self.big_text("HELP", x + (w - text_w) / 2, y + (h - text_h) / 2, scale as u32, label);
    }

    pub fn hits_help(x: usize, y: usize) -> bool {
        let (bx, by, bw, bh) = HELP_BUTTON;
        x >= bx && x < bx + bw && y >= by && y < by + bh
    }

    fn draw_cell(&mut self, row: usize, col: usize) {
        let x = TERM_X + col * GLYPH_W;
        let y = TERM_Y + row * GLYPH_H;
        self.fb.fill(x, y, GLYPH_W, GLYPH_H, BACKGROUND);
        let byte = self.cells[row][col];
        if byte != b' ' {
            let buf = [byte];
            let text = core::str::from_utf8(&buf).unwrap_or("?");
            let style = MonoTextStyle::new(&FONT_10X20, rgb(self.colors[row][col]));
            let _ = Text::with_baseline(text, Point::new(x as i32, y as i32), style, Baseline::Top).draw(&mut self.fb);
        }
    }

    fn redraw_cursor(&mut self, visible: bool) {
        if self.row < ROWS && self.col < COLS {
            self.draw_cell(self.row, self.col);
            if visible {
                let x = TERM_X + self.col * GLYPH_W;
                let y = TERM_Y + self.row * GLYPH_H;
                self.fb.fill(x, y + GLYPH_H - 3, GLYPH_W, 2, GOLD);
            }
        }
    }

    fn clear(&mut self) {
        self.cells = [[b' '; COLS]; ROWS];
        self.fb.fill(TERM_X, TERM_Y, COLS * GLYPH_W, ROWS * GLYPH_H, BACKGROUND);
        self.row = 0;
        self.col = 0;
    }

    fn newline(&mut self) {
        self.col = 0;
        self.row += 1;
        if self.row == ROWS {
            self.cells.copy_within(1.., 0);
            self.colors.copy_within(1.., 0);
            self.cells[ROWS - 1] = [b' '; COLS];
            self.row = ROWS - 1;
            for row in 0..ROWS {
                for col in 0..COLS {
                    self.draw_cell(row, col);
                }
            }
        }
    }

    /// Accepts the same byte stream as the serial port, including the small
    /// set of ANSI sequences the shell emits (colour and clear screen).
    pub fn write_byte(&mut self, byte: u8) {
        match self.escape {
            Escape::Start => {
                self.escape = if byte == b'[' { Escape::Params } else { Escape::None };
                self.param = 0;
                return;
            }
            Escape::Params => {
                match byte {
                    b'0'..=b'9' => self.param = self.param.saturating_mul(10).saturating_add((byte - b'0') as u32),
                    b';' => self.param = 0,
                    b'm' => {
                        self.color = match self.param {
                            30..=37 => PALETTE[[0,4,2,6,1,5,3,7][(self.param - 30) as usize]],
                            90..=97 => PALETTE[[8,12,10,14,9,13,11,15][(self.param - 90) as usize]],
                            _ => TEXT,
                        };
                        self.escape = Escape::None;
                    }
                    b'J' => {
                        if self.param == 2 {
                            self.clear();
                        }
                        self.escape = Escape::None;
                    }
                    b'H' => {
                        self.redraw_cursor(false);
                        self.row = 0;
                        self.col = 0;
                        self.escape = Escape::None;
                    }
                    _ => self.escape = Escape::None,
                }
                if self.escape == Escape::None {
                    self.redraw_cursor(true);
                }
                return;
            }
            Escape::None => {}
        }
        self.redraw_cursor(false);
        match byte {
            0x1b => self.escape = Escape::Start,
            b'\n' => self.newline(),
            b'\r' => self.col = 0,
            0x08 => self.col = self.col.saturating_sub(1),
            b' '..=b'~' => {
                if self.col == COLS {
                    self.newline();
                }
                self.cells[self.row][self.col] = byte;
                self.colors[self.row][self.col] = self.color;
                self.draw_cell(self.row, self.col);
                self.col += 1;
            }
            _ => {}
        }
        self.redraw_cursor(true);
    }
}
