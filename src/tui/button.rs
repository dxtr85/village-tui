use animaterm::prelude::Glyph;
use animaterm::prelude::Graphic;
use animaterm::prelude::Manager;
use std::collections::HashMap;

pub struct Button {
    pub g_id: usize,
}

impl Button {
    pub fn new(
        size: (usize, usize),
        layer: usize,
        offset: (isize, isize),
        text: &str,
        alt_text: Option<&str>,
        mgr: &mut Manager,
    ) -> Self {
        let mut g = Glyph::char(' ');
        let mut gr = Glyph::char(' ');
        gr.set_reverse(true);
        let mut frame_deselect = vec![g; size.0 * size.1];
        let mut frame_select = vec![gr; size.0 * size.1];
        let mut c_iter = text.chars().into_iter();
        let row = size.1 >> 1;
        let mut added = 1;
        while let Some(c) = c_iter.next() {
            if added >= size.0 - 1 {
                break;
            }
            g.set_char(c);
            gr.set_char(c);
            let location = size.0 * row + added;
            frame_deselect[location] = g;
            frame_select[location] = gr;
            added = added + 1;
        }

        let mut library = HashMap::new();
        library.insert(0, frame_deselect);
        library.insert(1, frame_select);
        if let Some(text) = alt_text {
            g.set_char(' ');
            gr.set_char(' ');
            frame_deselect = vec![g; size.0 * size.1];
            frame_select = vec![gr; size.0 * size.1];
            let mut c_iter = text.chars().into_iter();
            let row = size.1 >> 1;
            let mut added = 1;
            while let Some(c) = c_iter.next() {
                if added >= size.0 - 1 {
                    break;
                }
                g.set_char(c);
                gr.set_char(c);
                let location = size.0 * row + added;
                frame_deselect[location] = g;
                frame_select[location] = gr;
                added = added + 1;
            }

            library.insert(2, frame_deselect);
            library.insert(3, frame_select);
        }
        let g_id = mgr
            .add_graphic(
                Graphic::new(size.0, size.1, 0, library, None),
                layer,
                offset,
            )
            .unwrap();
        Button { g_id }
    }
    pub fn select(&self, mgr: &mut Manager, alternative: bool) {
        if alternative {
            mgr.set_graphic(self.g_id, 3, false);
        } else {
            mgr.set_graphic(self.g_id, 1, false);
        }
    }
    pub fn deselect(&self, mgr: &mut Manager, alternative: bool) {
        if alternative {
            mgr.set_graphic(self.g_id, 2, false);
        } else {
            mgr.set_graphic(self.g_id, 0, false);
        }
    }
}
