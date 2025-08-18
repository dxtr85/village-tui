use super::button::Button;
// use super::editor::Editor;
// use super::selector::Selector;
// use super::serve_editor;
// use crate::logic::Manifest;
use animaterm::prelude::Glyph;
use animaterm::prelude::Graphic;
use animaterm::prelude::Manager;
// use dapp_lib::prelude::DataType;
// use dapp_lib::Data;
use std::collections::HashMap;
// TODO: A full screen window with options to
// - select data type
// - add description
// - add tags
// - buttons to apply or cancel action
//
// It will be used to both create new content and update existing content.
// It can be also used to view existing content in non-owned swarm,
// but without option to edit it's fields.
// You will be able to copy selected content as a link into your swarm,
// and there you will be able to edit the link that you own.
// When updating, DataType change will be blocked unless this is a Link
// without TransformInfo.
// As a result we get a modified Data block addressed at index 0 that we send to Swarm
// for synchronization

#[derive(Clone)]
pub enum CreatorResult {
    SelectDType,
    SelectTags,
    SelectDescription,
    Cancel,
    Create,
}
pub struct Creator {
    g_id: usize,
    display_id: usize,
    button_dtypes: Button,
    button_tags: Button,
    button_descr: Button,
    button_apply: Button,
    button_cancel: Button,
    width: usize,
    height: usize,
}

impl Creator {
    pub fn new(mgr: &mut Manager) -> Self {
        let (width, height) = mgr.screen_size();
        let display_id = mgr.new_display(true);
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(width, height, 0, library, None), 0, (0, 0))
            .unwrap();
        let button_dtypes = Button::new((8, 3), 1, (2, 1), "Edytuj", None, mgr);
        let button_descr = Button::new((8, 3), 1, (2, 5), "Edytuj", Some("Pokaż"), mgr);
        let button_tags = Button::new((8, 3), 1, (2, 9), "Edytuj", Some("Pokaż"), mgr);
        let button_apply = Button::new((8, 3), 1, (2, 20), "Zapisz", None, mgr);
        let button_cancel = Button::new((8, 3), 1, (2, 24), "Anuluj", Some("Zamknij"), mgr);
        Creator {
            g_id,
            display_id,
            button_dtypes,
            button_tags,
            button_descr,
            button_apply,
            button_cancel,
            width,
            height,
        }
    }

    pub fn cleanup(&self, main_display: usize, mgr: &mut Manager) {
        mgr.restore_display(self.display_id, true);
        mgr.restore_display(main_display, false);
    }

    fn update_tags(&self, mgr: &mut Manager, text: String) {
        // eprintln!("UT: '{}'", text);
        let mut g = Glyph::plain();
        let p = Glyph::plain();
        let mut char_iter = text.chars();
        for x in 12..self.width - 1 {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 10);
            } else {
                mgr.set_glyph(self.g_id, p, x, 10);
            }
        }
    }

    fn update_description(&self, mgr: &mut Manager, text: &str) {
        let mut g = Glyph::plain();
        let p = Glyph::plain();
        let mut char_iter = text.chars();
        for x in 12..self.width - 1 {
            if let Some(c) = char_iter.next() {
                if c == '\n' {
                    g.set_char(' ');
                } else {
                    g.set_char(c);
                }
                mgr.set_glyph(self.g_id, g, x, 6);
            } else {
                mgr.set_glyph(self.g_id, p, x, 6);
            }
        }
    }

    fn update_d_type(&self, mgr: &mut Manager, text: &str) {
        let d_text = &format!("DataType: {}", text);
        let p = Glyph::plain();
        let mut g = Glyph::plain();
        let mut char_iter = d_text.chars();
        for x in 12..self.width - 1 {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 2);
            } else {
                mgr.set_glyph(self.g_id, p, x, 2);
            }
        }
    }
    fn next_button(&self, read_only: bool, curr_button: usize) -> usize {
        if read_only {
            if curr_button == 0 {
                2
            } else if curr_button == 2 {
                3
            } else {
                0
            }
        } else {
            if curr_button == 0 {
                1
            } else if curr_button == 1 {
                2
            } else if curr_button == 2 {
                3
            } else if curr_button == 3 {
                4
            } else {
                0
            }
        }
    }

    fn prev_button(&self, read_only: bool, curr_button: usize) -> usize {
        if read_only {
            if curr_button == 0 {
                3
            } else if curr_button == 3 {
                2
            } else {
                0
            }
        } else {
            if curr_button == 0 {
                4 // buttons_count - 1
            } else {
                curr_button - 1
            }
        }
    }

    pub fn show(
        &mut self,
        main_display: usize,
        mgr: &mut Manager,
        read_only: bool,
        d_type: String,
        tags: String,
        description: String,
    ) -> CreatorResult {
        mgr.restore_display(self.display_id, true);
        self.update_d_type(mgr, &d_type);
        let t_text = format!("Tags: {}", tags);
        self.update_tags(mgr, t_text);
        let s_text = format!("Description: {}", description);
        self.update_description(mgr, &s_text);
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let all_buttons = vec![
            &self.button_cancel,
            &self.button_dtypes,
            &self.button_descr,
            &self.button_tags,
            &self.button_apply,
        ];
        mgr.move_graphic(self.button_descr.g_id, 4, (0, 0));
        mgr.move_graphic(self.button_tags.g_id, 4, (0, 0));
        mgr.move_graphic(self.button_cancel.g_id, 4, (0, 0));
        if !read_only {
            mgr.move_graphic(self.button_dtypes.g_id, 4, (0, 0));
            mgr.move_graphic(self.button_apply.g_id, 4, (0, 0));
        }
        let available_buttons = all_buttons;
        let mut selected_button = 0;
        available_buttons[selected_button].select(mgr, read_only);
        available_buttons[1].deselect(mgr, read_only);
        available_buttons[2].deselect(mgr, read_only);
        available_buttons[3].deselect(mgr, read_only);
        available_buttons[4].deselect(mgr, read_only);

        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    animaterm::Key::Escape => {
                        available_buttons[selected_button].deselect(mgr, read_only);
                        return CreatorResult::Cancel;
                    }
                    animaterm::Key::Down | animaterm::Key::CtrlN => {
                        available_buttons[selected_button].deselect(mgr, read_only);
                        selected_button = self.next_button(read_only, selected_button);
                        available_buttons[selected_button].select(mgr, read_only);
                    }
                    animaterm::Key::Up | animaterm::Key::CtrlP => {
                        available_buttons[selected_button].deselect(mgr, read_only);
                        selected_button = self.prev_button(read_only, selected_button);
                        available_buttons[selected_button].select(mgr, read_only);
                    }
                    animaterm::Key::Enter => {
                        mgr.move_graphic(self.g_id, 0, (0, 0));
                        mgr.move_graphic(self.button_dtypes.g_id, 0, (0, 0));
                        mgr.move_graphic(self.button_descr.g_id, 0, (0, 0));
                        mgr.move_graphic(self.button_tags.g_id, 0, (0, 0));
                        mgr.move_graphic(self.button_apply.g_id, 0, (0, 0));
                        mgr.move_graphic(self.button_cancel.g_id, 0, (0, 0));
                        available_buttons[selected_button].deselect(mgr, read_only);
                        mgr.restore_display(main_display, true);
                        match selected_button {
                            1 => {
                                return CreatorResult::SelectDType;
                            }
                            2 => {
                                return CreatorResult::SelectDescription;
                            }
                            3 => {
                                return CreatorResult::SelectTags;
                            }
                            4 => {
                                available_buttons[selected_button].deselect(mgr, read_only);
                                return CreatorResult::Create;
                            }
                            0 => {
                                return CreatorResult::Cancel;
                            }
                            other => {
                                eprintln!("{}", other);
                            }
                        }
                    }
                    _other => {
                        //TODO
                    }
                }
            }
        }
    }
}
