use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Key;
use animaterm::Manager;
use std::collections::HashMap;

pub struct Question {
    g_id: usize,
    width: usize,
    height: usize,
}

impl Question {
    pub fn new(mgr: &mut Manager) -> Self {
        let (cols, rows) = mgr.screen_size();
        let width = cols;
        let height = rows;
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(
                Graphic::new(width, height, 0, library, None),
                0,
                // ((cols >> 1) as isize - 8, (rows >> 1) as isize - 6),
                (0, 0),
            )
            .unwrap();
        Question {
            g_id,
            width,
            height,
        }
    }
    pub fn ask(&self, question: &str, mgr: &mut Manager) -> bool {
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let mut g = Glyph::plain();
        let mut iter = question.chars();
        'outer: for y in 1..self.height {
            for x in 1..self.width {
                if let Some(c) = iter.next() {
                    g.set_char(c);
                    mgr.set_glyph(self.g_id, g, x, y);
                } else {
                    break 'outer;
                }
            }
        }
        // mgr.read_key()
        //TODO: make visual buttons
        let mut answer = false;
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    Key::Y | Key::ShiftY => {
                        answer = true;
                        break;
                    }
                    Key::N | Key::ShiftN => break,
                    _other => {}
                }
            }
        }
        let mut left_to_clear = question.len();
        let g = Glyph::plain();
        'outer: for y in 1..self.height {
            for x in 1..self.width {
                if left_to_clear > 0 {
                    mgr.set_glyph(self.g_id, g, x, y);
                    left_to_clear -= 1;
                } else {
                    break 'outer;
                }
            }
        }
        mgr.move_graphic(self.g_id, 0, (0, 0));
        answer
    }
}
