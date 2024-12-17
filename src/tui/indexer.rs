use crate::tui::Direction;
use animaterm::utilities::message_box;
use animaterm::{glyph, prelude::*};

use super::button::Button;

pub struct Indexer {
    pub g_id: usize,
    display_id: usize,
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
                (cols, 3),
                0,
                (0, (i as isize + 1) * 3),
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
    pub fn show(&mut self, mgr: &mut Manager) {
        // mgr.move_graphic(self.g_id, 2, (0, 0));
        mgr.set_graphic(self.g_id, 0, true);
    }

    pub fn move_selection(&mut self, direction: Direction, mgr: &mut Manager) {
        self.buttons[self.cursor_position].deselect(mgr, false);
        if matches!(direction, Direction::Up) {
            if self.cursor_position == 0 {
                self.cursor_position = self.visible_buttons - 1;
            } else {
                self.cursor_position -= 1;
            }
            self.buttons[self.cursor_position].select(mgr, false);
        } else {
            if self.cursor_position == self.visible_buttons - 1 {
                self.cursor_position = 0;
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
        self.cursor_position = 0;
        self.set_title(mgr, title);
        let h_len = headers.len();
        let b_len = self.buttons.len();
        self.visible_buttons = if h_len < b_len { h_len } else { b_len };
        // eprintln!("Indexer visible buttons: {}", self.visible_buttons);
        for i in 0..self.buttons.len() {
            if let Some(name) = headers.get(i) {
                self.buttons[i].rename(mgr, name);
                self.buttons[i].show(mgr);
            } else {
                //TODO: disable following buttons
                // break;
                self.buttons[i].hide(mgr);
            }
        }
        let result = self.run(mgr);
        mgr.restore_display(main_display, true);
        result
    }

    pub fn run(&mut self, mgr: &mut Manager) -> Option<usize> {
        self.buttons[self.cursor_position].select(mgr, false);
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    Key::Up | Key::CtrlP => self.move_selection(Direction::Up, mgr),
                    Key::Down | Key::CtrlN => self.move_selection(Direction::Down, mgr),
                    Key::Tab => break,
                    Key::Enter => return Some(self.cursor_position),
                    other => {
                        eprintln!("Unsupperted key pressed: {:?}", other);
                    }
                }
            }
        }
        None
    }
}
