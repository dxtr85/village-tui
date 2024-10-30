use crate::logic::Tag;
use animaterm::prelude::Glyph;
use animaterm::prelude::Graphic;
use animaterm::prelude::Manager;
use std::collections::HashMap;

pub struct TagTui {
    tag_id: u8,
    g_id: usize,
}

impl TagTui {
    pub fn new(offset: (isize, isize), mgr: &mut Manager) -> Self {
        eprintln!("add {:?}", offset);
        let mut library = HashMap::new();
        let mut g = Glyph::char('-');
        // 0 - frame not selected, not activated
        // 1 - frame selected, not activated
        // 2 - frame not selected, activated
        // 3 - frame selected, activated
        let frame = vec![g; 32];
        library.insert(0, frame);
        g.set_reverse(true);
        let frame = vec![g; 32];
        library.insert(1, frame);
        g.set_blink(true);
        let frame = vec![g; 32];
        library.insert(3, frame);
        g.set_reverse(false);
        let frame = vec![g; 32];
        library.insert(2, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(32, 1, 0, library, None), 4, offset)
            .unwrap();
        TagTui { tag_id: 0, g_id }
    }
    pub fn present(&mut self, selected: bool, activated: bool, mgr: &mut Manager) {
        //TODO
        let f_id = match (selected, activated) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        };
        mgr.set_graphic(self.g_id, f_id, false);
        mgr.move_graphic(self.g_id, 4, (0, 0));
    }
    pub fn hide(&mut self, mgr: &mut Manager) {
        //TODO
        mgr.move_graphic(self.g_id, 0, (0, 0));
        // mgr.set_graphic(self.g_id, 0, false);
    }
    pub fn update(&mut self, id: u8, text: &str, mgr: &mut Manager) {
        self.tag_id = id;
        mgr.set_graphic(self.g_id, 0, false);
        let mut c_iter = text.chars();
        let mut g = Glyph::plain();
        let gp = Glyph::plain();
        for i in 0..32 {
            if let Some(c) = c_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, i, 0);
            } else {
                mgr.set_glyph(self.g_id, gp, i, 0);
            }
        }
        mgr.set_graphic(self.g_id, 1, false);
        let mut c_iter = text.chars();
        let mut g = Glyph::plain();
        g.set_reverse(true);
        let mut gp = Glyph::plain();
        gp.set_reverse(true);
        for i in 0..32 {
            if let Some(c) = c_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, i, 0);
            } else {
                mgr.set_glyph(self.g_id, gp, i, 0);
            }
        }
        mgr.set_graphic(self.g_id, 2, false);
        let mut c_iter = text.chars();
        let mut g = Glyph::plain();
        g.set_blink(true);
        let gp = Glyph::plain();
        for i in 0..32 {
            if let Some(c) = c_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, i, 0);
            } else {
                mgr.set_glyph(self.g_id, gp, i, 0);
            }
        }
        mgr.set_graphic(self.g_id, 3, false);
        let mut c_iter = text.chars();
        let mut g = Glyph::plain();
        g.set_reverse(true);
        g.set_blink(true);
        let mut gp = Glyph::plain();
        gp.set_reverse(true);
        for i in 0..32 {
            if let Some(c) = c_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, i, 0);
            } else {
                mgr.set_glyph(self.g_id, gp, i, 0);
            }
        }
    }
}
