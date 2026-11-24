//! Original Corvus pixel art and kernel chrome (the web keeps its own layout).
use super::*;

const BUTTONS: [(&str, &str); 4] = [("help", "HELP"), ("dio", "DIO"), ("crow", "CROW"), ("clear", "CLEAR")];

impl Console {
    fn mark(&mut self, x: usize, y: usize, scale: usize, phase: usize) {
        let crow = ["          XXX     ", "        XXXXXXX   ", " XXXXX XXXXoXXXX> ",
            "XXXXXXXXXXXXXXX   ", " XXXXXXXXXXXXX    ", "   XXXXXXXXX      ",
            "     XXXXX        ", "     XX XX        ", "    XX   XX       "];
        for (row, line) in crow.iter().enumerate() {
            for (col, byte) in line.bytes().enumerate() {
                if byte == b' ' { continue; }
                let color = if byte == b'o' { PALETTE[15] } else if phase % 2 == 1 && col < 5 { PALETTE[13] } else { GOLD };
                self.fb.fill(x + col * scale, y + row * scale, scale, scale, color);
            }
        }
        let (lx, ly) = (x + 14 * scale, y + 7 * scale);
        self.fb.fill(lx, ly, scale * 3, scale * 3, PURPLE);
        self.fb.fill(lx + scale, ly + scale, scale, scale, if phase % 2 == 0 { GOLD } else { PALETTE[15] });
    }

    pub fn splash(&mut self, elapsed_ms: u64) {
        self.fb.fill(0, 0, WIDTH, HEIGHT, BACKGROUND);
        self.mark(250, 86, 16, (elapsed_ms / 120) as usize);
        self.big_text("CORVUS NOCTIS", 205, 288, 3, GOLD);
        self.big_text("An operating system you can talk to.", 220, 364, 1, TEXT);
        self.fb.fill(200, 416, 400, 8, PURPLE_DARK);
        self.fb.fill(200, 416, (elapsed_ms.min(1500) * 400 / 1500) as usize, 8, GOLD);
        self.big_text("one mind. a thousand caws.", 270, 452, 1, MUTED);
    }

    pub fn draw_chrome(&mut self, logo: Option<&Frames>) {
        self.fb.fill(0, 0, WIDTH, HEIGHT, BACKGROUND);
        if let Some(frames) = logo {
            self.has_logo = true; frames.draw(0, &mut self.fb, LOGO_X, LOGO_Y);
        } else { self.mark(16, 18, 6, 0); }
        self.big_text("CORVUS NOCTIS", 154, 12, 3, GOLD);
        self.big_text("an operating system you can talk to.", 156, 78, 1, TEXT);
        for (i, color) in PALETTE.iter().enumerate() { self.fb.fill(560 + i * 14, 26, 12, 12, *color); }
        for (command, _) in BUTTONS { self.draw_button(command, false); }
        self.fb.fill(0, 154, WIDTH, 2, PURPLE);
        self.redraw_cursor(true);
    }

    pub fn draw_button(&mut self, command: &str, pressed: bool) {
        if let Some(index) = BUTTONS.iter().position(|b| b.0 == command) {
            let (x,y,w,h) = (16 + index * 196,112,180,36);
            let fill = if pressed { GOLD } else { [PURPLE,PALETTE[2],PALETTE[1],PURPLE_DARK][index] };
            self.fb.fill(x,y,w,h,GOLD); self.fb.fill(x+2,y+2,w-4,h-4,fill);
            let label = BUTTONS[index].1;
            self.big_text(label,x+(w-label.len()*GLYPH_W)/2,y+8,1,if pressed { BACKGROUND } else { PALETTE[15] });
        }
    }

    pub fn hit_button(x: usize, y: usize) -> Option<&'static str> {
        if Self::hits_help(x,y) { return Some("help"); }
        BUTTONS.iter().enumerate().skip(1).find(|(i,_)| x >= 16+i*196 && x < 196+i*196 && (112..148).contains(&y)).map(|(_,b)| b.0)
    }

    pub fn update_chrome(&mut self, ms: u64, online: bool) {
        if self.logo_requested && !self.has_logo {
            self.logo_requested = false; self.animation_until = ms + 1500; self.animation_next = ms;
        }
        if !self.has_logo && ms < self.animation_until && ms >= self.animation_next {
            self.fb.fill(12,12,126,88,BACKGROUND); self.mark(16,18,6,(ms / 120) as usize);
            self.animation_next = ms + 120;
        }
        if self.status_second != ms/1000 || self.status_online != online {
            self.status_second = ms/1000; self.status_online = online;
            self.fb.fill(0,574,WIDTH,26,PURPLE_DARK);
            self.big_text("UPTIME",16,578,1,MUTED);
            let mut digits=[0;20];
            self.big_text(crate::calc::format((ms/1000).min(i64::MAX as u64) as i64,&mut digits),96,578,1,GOLD);
            self.big_text("seconds",250,578,1,MUTED);
            self.big_text(if online { "dio ready" } else { "dio offline" },650,578,1,if online { PALETTE[10] } else { MUTED });
        }
    }
}
