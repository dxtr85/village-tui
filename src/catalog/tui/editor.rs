use crate::catalog::tui::Direction;
use animaterm::utilities::message_box;
use animaterm::{glyph, prelude::*};

pub struct Editor {
    pub g_id: usize,
    display_id: usize,
    lines: Vec<String>,
    cursor_position: (usize, usize),
    max_position: (usize, usize),
    allow_newlines: bool,
    read_only: bool,
    can_edit: bool,
    byte_limit: Option<u16>,
}
impl Editor {
    pub fn new(mgr: &mut Manager) -> Self {
        let display_id = mgr.new_display(true);
        let (cols, rows) = mgr.screen_size();
        let m_box = message_box(
            Some("Text input".to_string()),
            String::new(),
            Glyph::plain(),
            cols,
            rows,
        );
        let g_id = mgr.add_graphic(m_box, 0, (0, 0)).unwrap();
        let mut editor = Editor {
            g_id,
            display_id,
            lines: vec![String::new(); rows],
            cursor_position: (2, 1),
            max_position: (cols - 2, rows - 2),
            allow_newlines: true,
            read_only: false,
            can_edit: true,
            byte_limit: None,
        };
        editor.show(mgr);
        editor
    }
    pub fn cleanup(&self, main_display: usize, mgr: &mut Manager) {
        eprintln!("Editor cleanup");
        mgr.restore_display(self.display_id, true);
        mgr.restore_display(main_display, false);
    }

    pub fn allow_newlines(&mut self, allow: bool) {
        self.allow_newlines = allow;
    }
    pub fn set_limit(&mut self, limit: Option<u16>) {
        self.byte_limit = limit;
    }
    pub fn show(&mut self, mgr: &mut Manager) {
        // mgr.move_graphic(self.g_id, 2, (0, 0));
        mgr.set_graphic(self.g_id, 0, true);
    }
    pub fn take_text(&mut self, mgr: &mut Manager) -> String {
        let mut result = String::new();
        // for i in 0..self.text.len() {
        //     let line = std::mem::replace(&mut self.text[i], String::new());
        //     result.push_str(line.trim_end_matches(' '));
        //     result.push(' ');
        // }
        // self.clear(mgr);
        // self.move_to_line_start(mgr);
        let mut glyph = Glyph::plain();
        for row in 1..=self.max_position.1 {
            self.cursor_position = (2, row);
            let taken = self.remove_chars_from_cursor_to_end(mgr);
            mgr.set_glyph(self.g_id, glyph, 2, row);
            // eprintln!("Pushing: '{}'", taken);
            result.push_str(&taken);
        }
        self.cursor_position = (2, 1);
        glyph.set_blink(true);
        glyph.set_reverse(true);
        mgr.set_glyph(
            self.g_id,
            glyph,
            self.cursor_position.0,
            self.cursor_position.1,
        );
        //A primitive way to enforce max String size
        // TODO: find a better solution
        if let Some(byte_limit) = self.byte_limit {
            while result.len() > byte_limit as usize {
                result.pop();
            }
        }
        result
    }
    pub fn insert(&mut self, mgr: &mut Manager, ch: char) {
        //TODO: we need to take into consideration self.byte_limit
        //
        if let Some(byte_limit) = self.byte_limit {
            let mut cur_size = 0;
            for line in &self.lines {
                cur_size = cur_size + line.len();
            }
            if cur_size >= byte_limit as usize {
                return;
            }
        }
        let line_chars_count = self.lines[self.cursor_position.1].chars().count();
        if line_chars_count >= self.max_position.0 - 2 {
            return;
        }
        let glyph = if ch == '\n' {
            Glyph::plain()
        } else {
            Glyph::char(ch)
        };
        if self.cursor_position.0 - 2 == line_chars_count {
            self.lines[self.cursor_position.1].push(ch);
        } else {
            let mut chars = self.lines[self.cursor_position.1].chars();
            let mut new_string = String::with_capacity(line_chars_count);
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
            self.lines[self.cursor_position.1] = new_string;
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
        let len = self.lines[self.cursor_position.1].chars().count();
        // eprintln!("cursor position: {}", self.cursor_position.0);
        if len < self.cursor_position.0 - 2 {
            //Do nothing
            // eprintln!("1");
        } else if len == self.cursor_position.0 - 2 {
            // eprintln!("2");
            mgr.set_glyph(
                self.g_id,
                glyph,
                self.cursor_position.0,
                self.cursor_position.1,
            );
            last_char = true;
            self.lines[self.cursor_position.1].pop();
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
            // eprintln!(
            //     "3, len: {}, curr position -2 : {}",
            //     len,
            //     self.cursor_position.0 - 2
            // );
            let mut chars = self.lines[self.cursor_position.1].chars();
            let mut new_string = String::with_capacity(len);
            for _i in 0..self.cursor_position.0 - 3 {
                if let Some(char) = chars.next() {
                    new_string.push(char);
                }
            }
            let _skip = chars.next();
            // eprintln!("Skiping: {:?}", skip);
            let mut i = 0;
            while let Some(char) = chars.next() {
                // eprintln!("pushing: {:?} at {}", char, self.cursor_position.0 + i - 1);
                new_string.push(char);
                mgr.set_glyph(
                    self.g_id,
                    Glyph::char(char),
                    self.cursor_position.0 + i - 1,
                    self.cursor_position.1,
                );
                i += 1;
            }
            self.lines[self.cursor_position.1] = new_string;
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
                self.cursor_position.0 = self.lines[self.cursor_position.1].chars().count() + 2;
            }
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            // eprintln!(
            //     "Got glyph: {} from pos: {}",
            //     glyph.character, self.cursor_position.0
            // );
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
        // eprintln!("cursor position: {}", self.cursor_position.0);
        if next_position.0 > self.max_position.0 {
            return;
        }
        // eprintln!("next position: {}", next_position.0);
        let len = self.lines[self.cursor_position.1].chars().count();
        if len < next_position.0 - 2 {
            //do nothing
            // eprintln!("del 1");
        } else if len == next_position.0 - 2 {
            // eprintln!("del 2");
            self.lines[self.cursor_position.1].pop();
            mgr.set_glyph(
                self.g_id,
                Glyph::plain(),
                self.cursor_position.0,
                self.cursor_position.1,
            );
        } else {
            // eprintln!("del 3");
            let mut chars = self.lines[next_position.1].chars();
            let mut new_string = String::with_capacity(len);
            for _i in 0..self.cursor_position.0 - 2 {
                if let Some(char) = chars.next() {
                    new_string.push(char);
                }
            }
            let _skip = chars.next();
            // eprintln!("Skip: {:?}", _skip);
            let mut i = next_position.0 - 1;
            while let Some(char) = chars.next() {
                // eprintln!("push: '{}'", char);
                new_string.push(char);
                mgr.set_glyph(self.g_id, Glyph::char(char), i, next_position.1);
                i += 1;
            }
            mgr.set_glyph(self.g_id, Glyph::plain(), i, next_position.1);
            self.lines[self.cursor_position.1] = new_string;
        }
        mgr.get_glyph(self.g_id, self.cursor_position.0, self.cursor_position.1);
        if let Ok(AnimOk::GlyphRetrieved(_u, mut glyph)) = mgr.read_result() {
            // eprintln!("Retrieved: '{}'", glyph.character);
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
    pub fn remove_chars_from_cursor_to_end(&mut self, mgr: &mut Manager) -> String {
        let mut chars = self.lines[self.cursor_position.1].chars();
        let mut new_string = String::with_capacity(self.max_position.0);
        let mut old_string = String::with_capacity(self.max_position.0);
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
        let plain = Glyph::plain();
        while let Some(char) = chars.next() {
            // eprintln!(
            //     "Clearing {} {}",
            //     self.cursor_position.0 + i,
            //     self.cursor_position.1
            // );
            mgr.set_glyph(
                self.g_id,
                plain,
                self.cursor_position.0 + i,
                self.cursor_position.1,
            );
            old_string.push(char);
            i += 1;
        }
        // eprint!("NS: {} ;", new_string);
        self.lines[self.cursor_position.1] = new_string;
        if self.allow_newlines && !old_string.is_empty() && !old_string.ends_with('\n') {
            old_string.push('\n');
        }
        old_string
    }
    pub fn move_to_line_start(&mut self, mgr: &mut Manager) {
        // eprintln!("Move to start");
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
        // eprintln!("Move to end");
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
        self.cursor_position.0 = self.lines[self.cursor_position.1].chars().count() + 2;
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
    pub fn set_title(&mut self, mgr: &mut Manager, title: &str) {
        let mut chars = title.chars();
        let mut g = glyph::Glyph::char('*');
        for i in 1..self.max_position.0 {
            if let Some(char) = chars.next() {
                g.set_char(char);
            } else {
                g.set_char('─');
            }
            mgr.set_glyph(self.g_id, g, i, 0);
        }
    }
    // TODO: also put lines into self.lines
    pub fn set_text(&mut self, mgr: &mut Manager, text: &str) {
        let mut lines = text.lines();
        // let gp = Glyph::plain();
        let mut g = Glyph::plain();
        let mut curr_x_position = 2;
        let mut curr_y_position = 1;
        while let Some(line) = lines.next() {
            let mut chars = line.chars();
            let ll = chars.clone().count();
            if ll < self.max_position.0 - 2 {
                while let Some(c) = chars.next() {
                    if c == '\n' {
                        continue;
                    }
                    g.set_char(c);
                    self.lines[curr_y_position].push(c);
                    mgr.set_glyph(self.g_id, g, curr_x_position, curr_y_position);
                    curr_x_position += 1;
                }
                // for i in curr_x_position..self.max_position.0 {
                //     mgr.set_glyph(self.g_id, gp, i, curr_y_position);
                // }
                curr_y_position += 1;
                curr_x_position = 2;
            } else {
                while let Some(c) = chars.next() {
                    eprint!("C:{} ", c);
                    g.set_char(c);
                    mgr.set_glyph(self.g_id, g, curr_x_position, curr_y_position);
                    curr_x_position += 1;
                    if curr_x_position > self.max_position.0 {
                        curr_x_position = 2;
                        curr_y_position += 1;
                    }
                }
                // for i in curr_x_position..self.max_position.0 {
                //     mgr.set_glyph(self.g_id, gp, i, curr_y_position);
                // }
                // TODO: how do we split those longer lines?
                eprintln!(
                    "dupa {}> {} (line:{})",
                    ll,
                    self.max_position.0 - 2,
                    curr_y_position
                );
                curr_y_position += 1;
                curr_x_position = 2;
            }
        }
    }
    pub fn serve(
        &mut self,
        main_display: usize,
        title: &str,
        initial_text: Option<String>,
        allow_newlines: bool,
        byte_limit: Option<u16>,
        mgr: &mut Manager,
    ) -> Option<String> {
        mgr.restore_display(self.display_id, true);
        self.set_title(mgr, title);
        self.allow_newlines(allow_newlines);
        self.set_limit(byte_limit);
        if let Some(text) = initial_text {
            self.set_text(mgr, &text);
        }
        // print!("Type text in (press TAB to finish): ");
        let result = self.run(mgr);
        mgr.restore_display(main_display, true);
        result
    }

    pub fn set_mode(&mut self, (read_only, can_edit): (bool, bool)) {
        self.read_only = read_only;
        self.can_edit = can_edit;
    }
    pub fn run(&mut self, mgr: &mut Manager) -> Option<String> {
        loop {
            if let Some(ch) = mgr.read_char() {
                // eprintln!("Some ch: {}", ch);
                if let Some(key) = map_private_char_to_key(ch) {
                    // eprintln!("Some key: {:?}", key);
                    match key {
                        Key::Up => {
                            if self.allow_newlines {
                                self.move_cursor(Direction::Up, mgr)
                            }
                        }
                        Key::Down => {
                            if self.allow_newlines {
                                self.move_cursor(Direction::Down, mgr)
                            }
                        }
                        Key::Left => self.move_cursor(Direction::Left, mgr),
                        Key::Right => self.move_cursor(Direction::Right, mgr),
                        Key::Home => self.move_to_line_start(mgr),
                        Key::End => self.move_to_line_end(mgr),
                        Key::Delete => {
                            if self.read_only {
                                continue;
                            }
                            self.delete(mgr)
                        }
                        Key::F8 => {
                            eprintln!("Read-only:{}, can_edit: {}", self.read_only, self.can_edit);
                            if self.read_only && self.can_edit {
                                self.read_only = false;
                                eprintln!("Read-write enabled");
                            }
                        }
                        Key::AltB => {
                            self.move_cursor(Direction::Left, mgr);
                            self.move_cursor(Direction::Left, mgr);
                            self.move_cursor(Direction::Left, mgr);
                            self.move_cursor(Direction::Left, mgr);
                        }
                        Key::AltF => {
                            self.move_cursor(Direction::Right, mgr);
                            self.move_cursor(Direction::Right, mgr);
                            self.move_cursor(Direction::Right, mgr);
                            self.move_cursor(Direction::Right, mgr);
                        }
                        other => eprint!("Other: {}", other),
                    }
                } else if ch == '\t' {
                    let taken = self.take_text(mgr);
                    // mgr.restore_display(main_display, true);
                    if self.read_only {
                        return None;
                    } else {
                        return Some(taken);
                    }
                } else if ch == '\u{7f}' {
                    if self.read_only {
                        continue;
                    }
                    self.backspace(mgr);
                } else if ch == '\u{1}' {
                    self.move_to_line_start(mgr);
                } else if ch == '\u{5}' {
                    self.move_to_line_end(mgr);
                // } else if ch == '\u{a}' {
                //     // Enter or newline
                //     eprintln!("Enter!");
                //     if self.allow_newlines {
                //         self.move_cursor(Direction::Down, mgr);
                //         self.move_to_line_start(mgr);
                //     }
                } else if ch == '\u{b}' {
                    if self.read_only {
                        continue;
                    }
                    self.remove_chars_from_cursor_to_end(mgr);
                } else if ch == '\u{e}' {
                    if self.allow_newlines {
                        self.move_cursor(Direction::Down, mgr);
                    }
                } else if ch == '\u{10}' {
                    if self.allow_newlines {
                        self.move_cursor(Direction::Up, mgr);
                    }
                } else if ch == '\u{2}' {
                    self.move_cursor(Direction::Left, mgr);
                } else if ch == '\u{6}' {
                    self.move_cursor(Direction::Right, mgr);
                } else {
                    // eprint!("code: {:?}", ch);
                    if ch == '\n' && !self.allow_newlines {
                        // Do nothing
                    } else if !self.read_only {
                        self.insert(mgr, ch);
                    }
                }
            }
        }
    }
}
