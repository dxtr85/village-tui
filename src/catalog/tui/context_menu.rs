use std::collections::HashMap;

use animaterm::prelude::*;

#[derive(Copy, Clone)]
struct FrameSet {
    starting_frame: usize,
    last_frame: usize,
}

impl FrameSet {
    fn next_frame(&self, current: usize) -> usize {
        if self.starting_frame == self.last_frame {
            return self.starting_frame;
        } else if current >= self.last_frame {
            self.starting_frame + 1
        } else {
            current + 1
        }
    }
    fn prev_frame(&self, current: usize) -> usize {
        if self.starting_frame == self.last_frame {
            return self.starting_frame;
        } else if current <= self.starting_frame + 1 {
            self.last_frame
        } else {
            current - 1
        }
    }
}
pub struct CMenu {
    g_id: usize,
    width: usize,
    height: usize,
    current_set: FrameSet,
    last_defined_set: usize,
    defined_sets: HashMap<usize, FrameSet>,
}

impl CMenu {
    pub fn new(mgr: &mut Manager) -> Self {
        let (_cols, _rows) = mgr.screen_size();
        let width = 15;
        let height = 8;
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
        let mut defined_sets = HashMap::new();
        defined_sets.insert(
            0,
            FrameSet {
                starting_frame: 0,
                last_frame: 0,
            },
        );
        CMenu {
            g_id,
            width,
            height,
            current_set: FrameSet {
                starting_frame: 0,
                last_frame: 0,
            },
            last_defined_set: 0,
            defined_sets,
        }
    }
    pub fn show(&mut self, mgr: &mut Manager, set_nr: usize, offset: (isize, isize)) -> usize {
        self.current_set(set_nr);
        let mut selection = self.current_set.starting_frame;
        // eprintln!("Set nr: {}", set_nr);
        mgr.set_graphic(self.g_id, self.current_set.starting_frame, false);
        mgr.move_graphic(self.g_id, 4, offset);
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    Key::Up | Key::K | Key::CtrlP => {
                        selection = self.current_set.prev_frame(selection);
                        // eprintln!("UP {}", selection);
                        mgr.set_graphic(self.g_id, selection, false);
                    }
                    Key::Down | Key::J | Key::CtrlN => {
                        selection = self.current_set.next_frame(selection);
                        // eprintln!("DOWN {:?}", selection);
                        mgr.set_graphic(self.g_id, selection, false);
                    }
                    Key::Enter | Key::Space => {
                        mgr.set_graphic(self.g_id, 0, false);
                        mgr.move_graphic(self.g_id, 0, (offset.0 * -1, offset.1 * -1));
                        return selection - self.current_set.starting_frame;
                    }
                    Key::Escape => {
                        mgr.set_graphic(self.g_id, 0, false);
                        mgr.move_graphic(self.g_id, 0, (offset.0 * -1, offset.1 * -1));
                        return 0;
                    }
                    _ => {}
                }
            }
        }
        // selection
    }

    pub fn add_set(&mut self, mgr: &mut Manager, options: Vec<String>) -> usize {
        //TODO
        let new_frames_count = options.len() + 1;
        if new_frames_count == 1 {
            return self.last_defined_set;
        } else if new_frames_count > self.height + 1 {
            return self.last_defined_set;
        }
        mgr.empty_frame(self.g_id);
        if let Ok(AnimOk::FrameAdded(graphic_id, frame_id)) = mgr.read_result() {
            if graphic_id == self.g_id {
                let starting_frame = frame_id;
                let mut last_frame = frame_id;
                let n_options = options.clone();
                let mut n_options = n_options.iter();
                for y in 0..self.height {
                    self.set_row(y, n_options.next(), mgr, false);
                }
                for (row_nr, text) in options.into_iter().enumerate() {
                    mgr.clone_frame(self.g_id, Some(starting_frame));
                    if let Ok(AnimOk::FrameAdded(graphic_id, frame_id)) = mgr.read_result() {
                        if graphic_id == self.g_id {
                            last_frame = frame_id;
                            self.set_row(row_nr, Some(&text), mgr, true);
                        }
                    }
                }

                // eprintln!("Adding set start: {}, stop: {}", starting_frame, last_frame);
                self.last_defined_set += 1;
                self.defined_sets.insert(
                    self.last_defined_set,
                    FrameSet {
                        starting_frame,
                        last_frame,
                    },
                );
            }
        }
        mgr.set_graphic(self.g_id, 0, false);
        self.last_defined_set
    }

    fn current_set(&mut self, id: usize) {
        self.current_set = if id > self.last_defined_set {
            *self.defined_sets.get(&0).unwrap()
        } else {
            *self.defined_sets.get(&id).unwrap()
        };
    }
    fn set_row(&self, row_nr: usize, text: Option<&String>, mgr: &mut Manager, reversed: bool) {
        let text = if let Some(txt) = text {
            txt.clone()
        } else {
            String::new()
        };
        let mut iter = text.chars();
        for x in 0..self.width {
            let mut glyph = if let Some(char) = iter.next() {
                Glyph::char(char)
            } else {
                Glyph::plain()
            };
            if reversed {
                glyph.set_reverse(true);
            }
            mgr.set_glyph(self.g_id, glyph, x, row_nr);
        }
    }
}
