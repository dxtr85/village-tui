use super::button::Button;
use super::editor::Editor;
use super::selector::Selector;
use super::serve_editor;
use crate::logic::Manifest;
use animaterm::prelude::Glyph;
use animaterm::prelude::Graphic;
use animaterm::prelude::Manager;
use dapp_lib::prelude::DataType;
use dapp_lib::Data;
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
pub struct Creator {
    g_id: usize,
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
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(width, height, 0, library, None), 0, (0, 0))
            .unwrap();
        let button_dtypes = Button::new((8, 3), 1, (2, 1), "Edytuj", mgr);
        let button_descr = Button::new((8, 3), 1, (2, 5), "Edytuj", mgr);
        let button_tags = Button::new((8, 3), 1, (2, 9), "Edytuj", mgr);
        let button_apply = Button::new((8, 3), 1, (2, 20), "Zapisz", mgr);
        let button_cancel = Button::new((8, 3), 1, (2, 24), "Anuluj", mgr);
        Creator {
            g_id,
            button_dtypes,
            button_tags,
            button_descr,
            button_apply,
            button_cancel,
            width,
            height,
        }
    }

    fn update_tags(&self, mgr: &mut Manager, text: String) {
        // eprintln!("UT: '{}'", text);
        let mut g = Glyph::plain();
        let p = Glyph::plain();
        let mut char_iter = text.chars();
        for x in 12..self.width + 12 {
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
        for x in 12..self.width + 12 {
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
        for x in 12..self.width + 12 {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 2);
            } else {
                mgr.set_glyph(self.g_id, p, x, 2);
            }
        }
    }

    pub fn show(
        &mut self,
        mgr: &mut Manager,
        manifest: &Manifest,
        selector: &mut Selector,
        read_only: bool,
        d_type: Option<DataType>,
        tags: Vec<u8>,
        description: String,
        main_display: usize,
        input_display: usize,
        editor: &mut Editor,
    ) -> Option<(DataType, Data)> {
        let mut selected_dtype = if let Some(dt) = d_type {
            dt
        } else {
            DataType::Data(0)
        };
        let mut selected_description = description;
        let d_type_txt = if let Some(dt) = d_type {
            selected_dtype = dt;
            format!("{:?}", manifest.dtype_string(dt.byte()))
        } else {
            "none".to_string()
        };
        self.update_d_type(mgr, &d_type_txt);
        let mut selected_tags = vec![];
        for t_id in &tags {
            selected_tags.push(*t_id as usize);
        }
        let tags_txt = manifest.tags_string(tags);
        let t_text = format!("Tags: {}", tags_txt);
        self.update_tags(mgr, t_text);
        let s_text = format!("Description: {}", selected_description);
        self.update_description(mgr, &s_text);
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let mut available_buttons = vec![&self.button_cancel];
        if !read_only {
            mgr.move_graphic(self.button_dtypes.g_id, 4, (0, 0));
            mgr.move_graphic(self.button_descr.g_id, 4, (0, 0));
            mgr.move_graphic(self.button_tags.g_id, 4, (0, 0));
            mgr.move_graphic(self.button_apply.g_id, 4, (0, 0));
            available_buttons.push(&self.button_dtypes);
            available_buttons.push(&self.button_descr);
            available_buttons.push(&self.button_tags);
            available_buttons.push(&self.button_apply);
        }
        mgr.move_graphic(self.button_cancel.g_id, 4, (0, 0));
        let available_buttons = available_buttons;
        let buttons_count = available_buttons.len();
        let mut selected_button = 0;
        available_buttons[selected_button].select(mgr);

        let mut cancel_selected = true;
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    animaterm::Key::Escape => {
                        cancel_selected = true;
                        available_buttons[selected_button].deselect(mgr);
                        break;
                    }
                    animaterm::Key::Down | animaterm::Key::CtrlN => {
                        if buttons_count <= 1 {
                            continue;
                        }
                        available_buttons[selected_button].deselect(mgr);
                        selected_button += 1;
                        if selected_button >= buttons_count {
                            selected_button = 0;
                        }
                        cancel_selected = selected_button == 0;
                        available_buttons[selected_button].select(mgr);
                    }
                    animaterm::Key::Up | animaterm::Key::CtrlP => {
                        if buttons_count <= 1 {
                            continue;
                        }
                        available_buttons[selected_button].deselect(mgr);
                        if selected_button == 0 {
                            selected_button = buttons_count - 1;
                        } else {
                            selected_button -= 1;
                        }
                        cancel_selected = selected_button == 0;
                        available_buttons[selected_button].select(mgr);
                    }
                    animaterm::Key::Enter => {
                        match selected_button {
                            1 => {
                                let tag_names = manifest.dtype_names();
                                mgr.move_graphic(self.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_dtypes.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_descr.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_tags.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_apply.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_cancel.g_id, 0, (0, 0));
                                let _selected = selector.select(
                                    "Select Data Type",
                                    &tag_names,
                                    vec![],
                                    mgr,
                                    true,
                                );
                                if !_selected.is_empty() && !tag_names.is_empty() {
                                    selected_dtype = DataType::from(_selected[0] as u8);
                                    self.update_d_type(mgr, &tag_names[_selected[0]]);
                                }
                                mgr.move_graphic(self.g_id, 3, (0, 0));
                                mgr.move_graphic(self.button_dtypes.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_descr.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_tags.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_apply.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_cancel.g_id, 4, (0, 0));
                            }
                            2 => {
                                eprintln!("Type description in");
                                //TODO
                                let edit_result = serve_editor(
                                    input_display,
                                    main_display,
                                    editor,
                                    " Max size: 764  Multiline  Content Description    (TAB to finish)",Some(selected_description.clone()),
                                    true,
                                    // false,
                                    // None,
                                    Some(764),
                                    mgr,
                                );
                                if let Some(text) = edit_result {
                                    eprintln!("Got: '{}'", text);
                                    selected_description = text;
                                    self.update_description(mgr, &selected_description);
                                }
                            }
                            3 => {
                                let tag_names = manifest.tag_names();
                                mgr.move_graphic(self.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_dtypes.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_descr.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_tags.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_apply.g_id, 0, (0, 0));
                                mgr.move_graphic(self.button_cancel.g_id, 0, (0, 0));
                                selected_tags = selector.select(
                                    "Select Tags",
                                    &tag_names,
                                    selected_tags,
                                    mgr,
                                    false,
                                );
                                // if !selected_tags.is_empty() {
                                //TODO
                                eprintln!("Selected tags: {:?}", selected_tags);
                                let mut tags_string = "Tags: ".to_string();
                                for tag_id in &selected_tags {
                                    tags_string.push_str(&tag_names[*tag_id]);
                                    tags_string.push(' ');
                                    tags_string.push(' ');
                                    tags_string.push(' ');
                                    tags_string.push(' ');
                                }
                                self.update_tags(mgr, tags_string);
                                // }
                                mgr.move_graphic(self.g_id, 3, (0, 0));
                                mgr.move_graphic(self.button_dtypes.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_descr.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_tags.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_apply.g_id, 4, (0, 0));
                                mgr.move_graphic(self.button_cancel.g_id, 4, (0, 0));
                            }
                            4 => {
                                available_buttons[selected_button].deselect(mgr);
                                break;
                            }
                            0 => {
                                cancel_selected = true;
                                available_buttons[selected_button].deselect(mgr);
                                break;
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
        mgr.move_graphic(self.g_id, 0, (0, 0));
        mgr.move_graphic(self.button_dtypes.g_id, 0, (0, 0));
        mgr.move_graphic(self.button_descr.g_id, 0, (0, 0));
        mgr.move_graphic(self.button_tags.g_id, 0, (0, 0));
        mgr.move_graphic(self.button_apply.g_id, 0, (0, 0));
        mgr.move_graphic(self.button_cancel.g_id, 0, (0, 0));
        if cancel_selected {
            None
        } else {
            let mut bytes = Vec::with_capacity(1024);
            bytes.push(selected_tags.len() as u8);
            for tag in selected_tags {
                bytes.push(tag as u8);
            }
            for byte in selected_description.bytes() {
                bytes.push(byte as u8);
            }
            Some((selected_dtype, Data::new(bytes).unwrap()))
        }
    }
}
