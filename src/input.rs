use crate::tui::Direction;
use animaterm::prelude::*;
use animaterm::utilities::message_box;
//TODO: we need to create a text input logic that will handle screen and update text
// with provided char.
// Input can be also modified/ended when receiving a Key.

pub struct Input {
    pub g_id: usize,
    text: Vec<String>,
    cursor_position: (usize, usize),
    max_position: (usize, usize),
}
impl Input {
    pub fn new(mgr: &mut Manager) -> Self {
        let (cols, rows) = mgr.screen_size();
        let m_box = message_box(
            Some("Text input".to_string()),
            String::new(),
            Glyph::plain(),
            cols,
            rows,
        );
        let g_id = mgr.add_graphic(m_box, 0, (0, 0)).unwrap();
        Input {
            g_id,
            text: vec![String::new(); rows],
            cursor_position: (2, 1),
            max_position: (cols - 2, rows - 2),
        }
    }
    pub fn show(&mut self, mgr: &mut Manager) {
        mgr.move_graphic(self.g_id, 2, (0, 0));
        mgr.set_graphic(self.g_id, 0, true);
    }
    pub fn hide(&mut self, mgr: &mut Manager) {
        mgr.move_graphic(self.g_id, 0, (0, 0));
        // mgr.set_graphic(1, 0, true);
    }
    pub fn insert(&mut self, mgr: &mut Manager, ch: char) {
        let len = self.text[self.cursor_position.1].chars().count();
        if len >= self.max_position.0 - 2 {
            return;
        }
        let glyph = if ch == '\n' {
            Glyph::plain()
        } else {
            Glyph::char(ch)
        };
        if self.cursor_position.0 - 2 == len {
            self.text[self.cursor_position.1].push(ch);
        } else {
            let mut chars = self.text[self.cursor_position.1].chars();
            let mut new_string = String::with_capacity(len);
            for _i in 0..self.cursor_position.0 - 2 {
                if let Some(char) = chars.next() {
                    new_string.push(char);
                } else {
                    new_string.push(' ');
                }
            }
            new_string.push(ch);
            let mut i = 1;
            while let Some(char) = chars.next() {
                new_string.push(char);
                mgr.set_glyph(
                    self.g_id,
                    Glyph::char(char),
                    i + self.cursor_position.0,
                    self.cursor_position.1,
                );
                i += 1;
            }
            self.text[self.cursor_position.1] = new_string;
        }
        mgr.set_glyph(
            self.g_id,
            glyph,
            self.cursor_position.0,
            self.cursor_position.1,
        );
        if ch == '\n' {
            if self.cursor_position.1 < self.max_position.1 {
                self.cursor_position.0 = 2;
                self.cursor_position.1 += 1;
            }
        } else {
            self.cursor_position.0 += 1;
            if self.cursor_position.0 > self.max_position.0 {
                self.cursor_position.0 = 2;
                self.cursor_position.1 += 1;
                if self.cursor_position.1 > self.max_position.1 {
                    self.cursor_position.1 = self.max_position.1;
                }
            }
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(true);
            glyph.set_reverse(true);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
    }

    pub fn backspace(&mut self, mgr: &mut Manager) {
        let glyph = Glyph::plain();
        let mut last_char = false;
        // mgr.set_glyph(
        //     self.g_id,
        //     glyph,
        //     self.cursor_position.0,
        //     self.cursor_position.1,
        // );
        let len = self.text[self.cursor_position.1].chars().count();
        eprintln!("cursor position: {}", self.cursor_position.0);
        if len < self.cursor_position.0 - 2 {
            //Do nothing
            eprintln!("1");
        } else if len == self.cursor_position.0 - 2 {
            eprintln!("2");
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
            last_char = true;
            self.text[self.cursor_position.1].pop();
            mgr.set_glyph(
                self.g_id,
                Glyph::plain(),
                self.cursor_position.0,
                self.cursor_position.1,
            );
        } else {
            if self.cursor_position.0 == 2 {
                return;
            }
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
            eprintln!(
                "3, len: {}, curr position -2 : {}",
                len,
                self.cursor_position.0 - 2
            );
            let mut chars = self.text[self.cursor_position.1].chars();
            let mut new_string = String::with_capacity(len);
            for _i in 0..self.cursor_position.0 - 3 {
                if let Some(char) = chars.next() {
                    new_string.push(char);
                }
            }
            let skip = chars.next();
            eprintln!("Skiping: {:?}", skip);
            let mut i = 0;
            while let Some(char) = chars.next() {
                eprintln!("pushing: {:?} at {}", char, self.cursor_position.0 + i - 1);
                new_string.push(char);
                mgr.set_glyph(
                    self.g_id,
                    Glyph::char(char),
                    self.cursor_position.0 + i - 1,
                    self.cursor_position.1,
                );
                i += 1;
            }
            self.text[self.cursor_position.1] = new_string;
            mgr.set_glyph(
                self.g_id,
                Glyph::plain(),
                self.cursor_position.0 + i - 1,
                self.cursor_position.1,
            );
        }
        self.cursor_position.0 -= 1;
        if self.cursor_position.0 < 2 {
            // eprintln!("p.0 <2");
            self.cursor_position.1 -= 1;
            if self.cursor_position.1 == 0 {
                // eprintln!("p.1 <2, start");
                self.cursor_position = (2, 1);
            } else {
                // eprintln!("end of prev line: {}", self.text[self.cursor_position.1]);
                self.cursor_position.0 = self.text[self.cursor_position.1].chars().count() + 2;
            }
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            eprintln!(
                "Got glyph: {} from pos: {}",
                glyph.character, self.cursor_position.0
            );
            glyph.set_blink(true);
            glyph.set_reverse(true);
            if last_char {
                glyph.set_char(' ');
            }
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
    }

    pub fn delete(&mut self, mgr: &mut Manager) {
        let next_position = (self.cursor_position.0 + 1, self.cursor_position.1);
        eprintln!("cursor position: {}", self.cursor_position.0);
        if next_position.0 > self.max_position.0 {
            return;
            // next_position = (2, next_position.1 + 1);
            // if next_position.1 > self.max_position.1 {
            //     // next_position = self.max_position;
            // }
        }
        eprintln!("next position: {}", next_position.0);
        let len = self.text[self.cursor_position.1].chars().count();
        if len < next_position.0 - 2 {
            //do nothing
            eprintln!("del 1");
        } else if len == next_position.0 - 2 {
            eprintln!("del 2");
            self.text[self.cursor_position.1].pop();
            mgr.set_glyph(
                self.g_id,
                Glyph::plain(),
                self.cursor_position.0,
                self.cursor_position.1,
            );
        } else {
            eprintln!("del 3");
            let mut chars = self.text[next_position.1].chars();
            let mut new_string = String::with_capacity(len);
            for _i in 0..self.cursor_position.0 - 2 {
                if let Some(char) = chars.next() {
                    new_string.push(char);
                }
            }
            let skip = chars.next();
            eprintln!("Skip: {:?}", skip);
            let mut i = next_position.0 - 1;
            while let Some(char) = chars.next() {
                eprintln!("push: '{}'", char);
                new_string.push(char);
                mgr.set_glyph(self.g_id, Glyph::char(char), i, next_position.1);
                i += 1;
            }
            mgr.set_glyph(self.g_id, Glyph::plain(), i, next_position.1);
            self.text[self.cursor_position.1] = new_string;
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            eprintln!("Retrieved: '{}'", glyph.character);
            glyph.set_blink(true);
            glyph.set_reverse(true);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
            // for i in next_position.0 + 1..self.max_position.0 {
            //     mgr.get_glyph(self.g_id, i, next_position.1);
            //     if let Ok(AnimOk::GlyphRetrieved(_u, glyph)) = mgr.read_result() {
            //         mgr.set_glyph(self.g_id, glyph, i - 1, next_position.1);
            //     } else {
            //         mgr.set_glyph(self.g_id, Glyph::plain(), i - 1, next_position.1);
            //     }
            // }
        }
    }
    pub fn remove_chars_from_cursor_to_end(&mut self, mgr: &mut Manager) {
        //TODO: remove all chars from selected till end of line
        let mut chars = self.text[self.cursor_position.1].chars();
        let mut new_string = String::with_capacity(self.max_position.0);
        for _i in 0..self.cursor_position.0 - 2 {
            if let Some(char) = chars.next() {
                new_string.push(char);
            } else {
                break;
            }
        }
        let mut glyph = Glyph::plain();
        glyph.set_blink(true);
        glyph.set_reverse(true);
        mgr.set_glyph(
            self.g_id,
            glyph,
            self.cursor_position.0,
            self.cursor_position.1,
        );
        let mut i = 1;
        while chars.next().is_some() {
            mgr.set_glyph(
                self.g_id,
                Glyph::plain(),
                self.cursor_position.0 + i,
                self.cursor_position.1,
            );
            i += 1;
        }
        self.text[self.cursor_position.1] = new_string;
    }
    pub fn move_to_line_start(&mut self, mgr: &mut Manager) {
        // println!("Move to start");
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(false);
            glyph.set_reverse(false);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
        self.cursor_position.0 = 2;
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(true);
            glyph.set_reverse(true);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
    }
    pub fn move_to_line_end(&mut self, mgr: &mut Manager) {
        // println!("Move to start");
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(false);
            glyph.set_reverse(false);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
        self.cursor_position.0 = self.text[self.cursor_position.1].chars().count() + 2;
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(true);
            glyph.set_reverse(true);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
    }
    pub fn move_cursor(&mut self, direction: Direction, mgr: &mut Manager) {
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(false);
            glyph.set_reverse(false);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
        match direction {
            Direction::Up => {
                self.cursor_position.1 -= 1;
                if self.cursor_position.1 < 1 {
                    self.cursor_position.1 = self.max_position.1;
                }
            }
            Direction::Down => {
                self.cursor_position.1 += 1;
                if self.cursor_position.1 > self.max_position.1 {
                    self.cursor_position.1 = 1;
                }
            }
            Direction::Left => {
                self.cursor_position.0 -= 1;
                if self.cursor_position.0 < 2 {
                    self.cursor_position.0 = self.max_position.0;
                }
            }
            Direction::Right => {
                self.cursor_position.0 += 1;
                if self.cursor_position.0 > self.max_position.0 {
                    self.cursor_position.0 = 2;
                }
            }
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            glyph.set_blink(true);
            glyph.set_reverse(true);
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
        }
    }
}
