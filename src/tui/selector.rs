use crate::tui::tag;
use animaterm::prelude::map_private_char_to_key;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Key;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use std::collections::HashMap;
use tag::Option;

use crate::logic::Manifest;

pub struct Selector {
    g_id: usize,
    width: usize,
    height: usize,
    options: Vec<Option>,
    selected_option: usize,
    last_updated_row: usize,
}

impl Selector {
    pub fn new(_app_type: AppType, mgr: &mut Manager) -> Self {
        let (cols, rows) = mgr.screen_size();
        let width = cols;
        let height = rows;
        let options_in_row = (width >> 5) as isize;
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(width, height, 0, library, None), 0, (0, 0))
            .unwrap();
        let mut tags = Vec::new();
        for y in (2..height).step_by(2) {
            for x in 0..options_in_row {
                tags.push(Option::new((x * 33, y as isize), mgr));
            }
        }
        Selector {
            g_id,
            width,
            height,
            options: tags,
            // selected_tag: usize::MAX,
            selected_option: 0,
            last_updated_row: 1,
        }
    }

    pub fn select(&mut self, options: &Vec<String>, mgr: &mut Manager) -> Vec<usize> {
        let mut selected = vec![];
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let gp = Glyph::plain();
        let mut g = Glyph::plain();
        let filter_header = format!(
            "Filter: ",
            // options.app_type,
            // options.len(),
        );
        let mut filter = String::with_capacity(64);
        let mut filter_len = filter_header.len();
        let mut iter = "Catalog Application's Tags".chars();
        for x in 1..self.width {
            if let Some(c) = iter.next() {
                if c == '\n' {
                    break;
                }
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 0);
            } else {
                mgr.set_glyph(self.g_id, gp, x, 0);
            }
        }
        let mut iter = filter_header.chars();
        for x in 1..self.width {
            if let Some(c) = iter.next() {
                if c == '\n' {
                    break;
                }
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 1);
            } else {
                mgr.set_glyph(self.g_id, gp, x, 1);
            }
        }
        let mut options_len = options.len();
        let options_in_row = self.width >> 5;
        let opt_rows_count = (self.height >> 1) - 1;
        let opts_per_page = options_in_row * opt_rows_count;
        let mut option_pages = vec![];
        let mut page = Vec::with_capacity(opts_per_page);
        let mut total_added = 0;
        for i in 0..options_len {
            if let Some(option) = options.get(i) {
                page.push((i, (option, false)));
                total_added += 1;
                if total_added % opts_per_page == 0 {
                    option_pages.push(page);
                    page = Vec::with_capacity(opts_per_page);
                }
                if total_added >= options_len {
                    if !page.is_empty() {
                        option_pages.push(page);
                    }
                    break;
                }
            }
        }
        if option_pages.is_empty() {
            option_pages.push(vec![]);
        }
        let mut curr_page = 0;
        let mut options_count = total_added;
        options_len = option_pages[curr_page].len();
        let mut opt_iter = option_pages[curr_page].iter();
        for p_id in 0..self.options.len() {
            if let Some((index, (name, active))) = &opt_iter.next() {
                let sel = self.selected_option == p_id;
                self.options[p_id].update(*index as usize, &name, mgr);
                self.options[p_id].present(sel, *active, mgr);
            } else {
                self.options[p_id].hide(mgr);
            }
        }
        loop {
            if let Some(char) = mgr.read_char() {
                if let Some(key) = map_private_char_to_key(char) {
                    match key {
                        Key::AltTab => {
                            self.selected_option = 0;
                            break;
                        }
                        Key::Space => {
                            let idx = self.options[self.selected_option].index;
                            if let Some(position) = selected.iter().position(|&x| x == idx) {
                                selected.remove(position);
                                self.options[self.selected_option].present(true, false, mgr);
                                let (index, (name, _active)) =
                                    option_pages[curr_page][self.selected_option];
                                option_pages[curr_page][self.selected_option] =
                                    (index, (name, false));
                                //TODO change frame
                            } else {
                                selected.push(idx);
                                self.options[self.selected_option].present(true, true, mgr);
                                let (index, (name, _active)) =
                                    option_pages[curr_page][self.selected_option];
                                option_pages[curr_page][self.selected_option] =
                                    (index, (name, true));
                                //TODO change frame
                            }
                        }
                        Key::PgDn => {
                            if option_pages.len() <= 1 {
                                continue;
                            }
                            self.selected_option = 0;
                            curr_page += 1;
                            if curr_page >= option_pages.len() {
                                curr_page = 0;
                            }
                            options_len = option_pages[curr_page].len();
                            let mut tag_iter = option_pages[curr_page].iter();
                            for p_id in 0..self.options.len() {
                                if let Some((k, (v, a))) = &tag_iter.next() {
                                    let sel = self.selected_option == p_id;
                                    let activ = a;
                                    self.options[p_id].hide(mgr);
                                    self.options[p_id].update(*k as usize, &v, mgr);
                                    self.options[p_id].present(sel, *activ, mgr);
                                } else {
                                    self.options[p_id].hide(mgr);
                                }
                            }
                        }
                        Key::PgUp => {
                            if option_pages.len() <= 1 {
                                continue;
                            }
                            if curr_page == 0 {
                                curr_page = option_pages.len() - 1;
                            } else {
                                curr_page -= 1;
                            }
                            self.selected_option = 0;
                            options_len = option_pages[curr_page].len();
                            let mut tag_iter = option_pages[curr_page].iter();
                            for p_id in 0..self.options.len() {
                                if let Some((k, (v, a))) = &tag_iter.next() {
                                    let sel = self.selected_option == p_id;
                                    let activ = a;
                                    self.options[p_id].hide(mgr);
                                    self.options[p_id].update(*k as usize, &v, mgr);
                                    self.options[p_id].present(sel, *activ, mgr);
                                } else {
                                    self.options[p_id].hide(mgr);
                                }
                            }
                        }
                        Key::Up => {
                            if options_len == 0 {
                                continue;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(false, active, mgr);
                            if options_len >= options_in_row {
                                if self.selected_option >= options_in_row {
                                    self.selected_option -= options_in_row;
                                } else {
                                    let mut new_positions = Vec::with_capacity(options_in_row);
                                    for i in 0..options_in_row {
                                        new_positions.push(options_len - options_in_row + i);
                                    }
                                    new_positions.rotate_right(options_len % options_in_row);
                                    self.selected_option = new_positions[self.selected_option];
                                }
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(true, active, mgr);
                        }
                        Key::Down => {
                            if options_len == 0 {
                                continue;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(false, active, mgr);
                            self.selected_option += options_in_row;
                            if self.selected_option >= options_len {
                                self.selected_option = self.selected_option % options_in_row;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(true, active, mgr);
                        }
                        Key::Right => {
                            if options_len == 0 {
                                continue;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(false, active, mgr);
                            self.selected_option += 1;
                            if self.selected_option >= options_len {
                                self.selected_option = 0;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(true, active, mgr);
                        }
                        Key::Left => {
                            if options_len == 0 {
                                continue;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(false, active, mgr);
                            if self.selected_option == 0 {
                                self.selected_option = options_len - 1;
                            } else {
                                self.selected_option -= 1;
                            }
                            let (_index, (_name, active)) =
                                option_pages[curr_page][self.selected_option];
                            self.options[self.selected_option].present(true, active, mgr);
                        }
                        _other => {}
                    }
                } else
                //Backspace
                if char == '\u{7f}' {
                    // eprintln!("BS, filter len: {}", filter_len);
                    if filter_len > filter_header.len() {
                        filter.pop();
                        g.set_char(' ');
                        mgr.set_glyph(self.g_id, g, filter_len, 1);
                        filter_len = filter_len - 1;
                    }
                } else
                //Escape
                if char == '\u{1b}' {
                    self.selected_option = 0;
                    break;
                } else {
                    // let mut options_count = total_added;
                    filter.push(char);
                    filter_len += 1;
                    let filtered_options = options
                        .iter()
                        .filter(|x| x.contains(&filter))
                        .collect::<Vec<_>>();
                    if filtered_options.len() != options_count {
                        eprintln!("options count has changed");
                    }

                    g.set_char(char);
                    mgr.set_glyph(self.g_id, g, filter_len, 1);
                }
            }
        }
        for tag in &mut self.options {
            tag.hide(mgr);
        }
        mgr.move_graphic(self.g_id, 0, (0, 0));
        selected
    }
}
