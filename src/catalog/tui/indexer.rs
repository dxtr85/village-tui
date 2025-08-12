use crate::catalog::tui::Direction;
use animaterm::utilities::message_box;
use animaterm::{glyph, prelude::*};

use super::button::Button;

pub struct Indexer {
    pub g_id: usize,
    display_id: usize,
    header_chunks: Vec<Vec<String>>,
    chunk_idx: usize,
    buttons: Vec<Button>,
    visible_buttons: usize,
    cursor_position: usize,
    max_position: (usize, usize),
    allow_newlines: bool,
    read_only: bool,
    byte_limit: Option<u16>,
}
impl Indexer {
    pub fn new(mgr: &mut Manager) -> Self {
        let display_id = mgr.new_display(true);
        let (cols, rows) = mgr.screen_size();
        let visible_buttons = rows / 3 - 1;
        let mut buttons = Vec::with_capacity(visible_buttons);
        for i in 0..visible_buttons {
            let button = Button::new(
                (cols-2, 3),
                0,
                (1, (i as isize + 1) * 3),
                &format!("Nagłówek {} z {}zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz", i, visible_buttons - 1),
                None,
                mgr,
            );
            button.select(mgr, false);
            if i != 0 {
                button.deselect(mgr, false);
            }
            buttons.push(button);
        }
        let m_box = message_box(
            Some("Text input".to_string()),
            String::new(),
            Glyph::plain(),
            cols,
            rows,
        );
        let g_id = mgr.add_graphic(m_box, 1, (0, 0)).unwrap();
        let mut indexer = Indexer {
            g_id,
            display_id,
            buttons,
            header_chunks: vec![],
            chunk_idx: 0,
            visible_buttons,
            cursor_position: 0,
            max_position: (cols - 2, rows - 2),
            allow_newlines: true,
            read_only: false,
            byte_limit: None,
        };
        indexer.show(mgr);
        indexer
    }
    pub fn cleanup(&self, main_display: usize, mgr: &mut Manager) {
        mgr.restore_display(self.display_id, true);
        mgr.restore_display(main_display, false);
    }

    pub fn show(&mut self, mgr: &mut Manager) {
        // mgr.move_graphic(self.g_id, 2, (0, 0));
        mgr.set_graphic(self.g_id, 0, true);
    }

    pub fn move_selection(&mut self, direction: Direction, mgr: &mut Manager) {
        self.buttons[self.cursor_position].deselect(mgr, false);
        if matches!(direction, Direction::Up) {
            if self.cursor_position == 0 {
                let chunks_count = self.header_chunks.len();
                if chunks_count > 1 {
                    let curr_idx = self.chunk_idx;
                    // eprintln!("ccount: {}(curr idx: {})", chunks_count, curr_idx);
                    self.chunk_idx = if curr_idx == 0 {
                        chunks_count - 1
                    } else {
                        curr_idx - 1
                    };
                    // eprintln!("cidx: {}", self.chunk_idx);
                    self.draw_buttons(mgr);
                } else {
                    self.cursor_position = self.visible_buttons - 1;
                }
            } else {
                self.cursor_position -= 1;
            }
            self.buttons[self.cursor_position].select(mgr, false);
        } else {
            if self.cursor_position == self.visible_buttons - 1 {
                let chunks_count = self.header_chunks.len();
                if chunks_count > 1 {
                    let curr_idx = self.chunk_idx;
                    self.chunk_idx = if curr_idx + 1 < chunks_count {
                        curr_idx + 1
                    } else {
                        0
                    };
                    self.draw_buttons(mgr);
                } else {
                    self.cursor_position = 0;
                }
            } else {
                self.cursor_position += 1;
            }
            self.buttons[self.cursor_position].select(mgr, false);
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

    pub fn serve(
        &mut self,
        main_display: usize,
        title: &str,
        headers: Vec<String>,
        mgr: &mut Manager,
    ) -> Option<usize> {
        mgr.restore_display(self.display_id, true);
        self.header_chunks = vec![];
        let h_len = headers.len();
        let b_len = self.buttons.len();
        let mut curr_chunk = Vec::with_capacity(b_len);
        let mut curr_size = 0;
        let mut head_iter = headers.iter();
        while let Some(text) = head_iter.next() {
            curr_chunk.push(text.clone());
            curr_size += 1;
            if curr_size >= b_len {
                curr_size = 0;
                let cc = std::mem::replace(&mut curr_chunk, Vec::with_capacity(b_len));
                self.header_chunks.push(cc);
            }
        }
        if !curr_chunk.is_empty() {
            self.header_chunks.push(curr_chunk);
        }
        self.set_title(mgr, title);
        self.chunk_idx = 0;
        self.draw_buttons(mgr);
        let result = self.run(mgr);
        mgr.restore_display(main_display, true);
        result
    }

    fn draw_buttons(&mut self, mgr: &mut Manager) {
        self.cursor_position = 0;
        self.visible_buttons = self.header_chunks[self.chunk_idx].len();
        // eprintln!("Indexer visible buttons: {}", self.visible_buttons);
        for i in 0..self.buttons.len() {
            if let Some(name) = self.header_chunks[self.chunk_idx].get(i) {
                self.buttons[i].rename(mgr, name);
                self.buttons[i].show(mgr);
            } else {
                //TODO: disable following buttons
                // break;
                self.buttons[i].hide(mgr);
            }
        }
    }
    pub fn run(&mut self, mgr: &mut Manager) -> Option<usize> {
        self.buttons[self.cursor_position].select(mgr, false);
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    Key::Up | Key::CtrlP => self.move_selection(Direction::Up, mgr),
                    Key::Down | Key::CtrlN => self.move_selection(Direction::Down, mgr),
                    Key::CtrlA => {
                        self.buttons[self.cursor_position].deselect(mgr, false);
                        self.cursor_position = 0;
                        self.buttons[0].select(mgr, false);
                    }
                    Key::CtrlE => {
                        self.buttons[self.cursor_position].deselect(mgr, false);
                        self.cursor_position = self.visible_buttons - 1;
                        self.buttons[self.cursor_position].select(mgr, false);
                    }
                    Key::Tab => break,
                    Key::Enter => {
                        // eprintln!(
                        //     "chunk_idx: {}, but len: {}, cursor pos: {}",
                        //     self.chunk_idx,
                        //     self.buttons.len(),
                        //     self.cursor_position
                        // );
                        return Some((self.chunk_idx * self.buttons.len()) + self.cursor_position);
                    }
                    other => {
                        eprintln!("Unsupperted key pressed: {:?}", other);
                    }
                }
            }
        }
        None
    }
}
