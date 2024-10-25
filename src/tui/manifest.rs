use animaterm::Glyph;
use animaterm::Graphic;
// use animaterm::Key;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use std::collections::HashMap;

use crate::logic::Manifest;

pub struct ManifestTui {
    g_id: usize,
    width: usize,
    height: usize,
}

impl ManifestTui {
    pub fn new(_app_type: AppType, mgr: &mut Manager) -> Self {
        let (cols, rows) = mgr.screen_size();
        let width = cols;
        let height = rows;
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(width, height, 0, library, None), 0, (0, 0))
            .unwrap();
        ManifestTui {
            g_id,
            width,
            height,
        }
    }
    pub fn present(&self, manifest: Manifest, mgr: &mut Manager) {
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let mut g = Glyph::plain();
        let text = format!(
            "APPLICATION MANIFEST\n\nType of application: {:?}, {} Tags   \n\n{:?}",
            manifest.app_type,
            manifest.tags.len(),
            manifest.tags
        );
        let mut iter = text.chars();
        'outer: for y in 1..self.height {
            for x in 1..self.width {
                if let Some(c) = iter.next() {
                    if c == '\n' {
                        break;
                    }
                    g.set_char(c);
                    mgr.set_glyph(self.g_id, g, x, y);
                } else {
                    break 'outer;
                }
            }
        }
        loop {
            if let Some(_key) = mgr.read_key() {
                break;
                // match key {
                //     Key::Y | Key::ShiftY => {
                //         answer = true;
                //         break;
                //     }
                //     Key::N | Key::ShiftN => break,
                //     other => {}
                // }
            }
        }
        mgr.move_graphic(self.g_id, 0, (0, 0));
    }
}
