use crate::catalog::tui::option;
use animaterm::prelude::map_private_char_to_key;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Key;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use option::Option;
use std::collections::HashMap;

#[derive(Debug)]
enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    NextPage,
    PrevPage,
    Select,
    AddToFilter(char),
    DelFromFilter,
    ClearFilter,
    Finish,
    None,
}

pub struct Selector {
    g_id: usize,
    display_id: usize,
    width: usize,
    height: usize,
    options: Vec<Option>,
    selected_option: usize,
    last_updated_row: usize,
}

impl Selector {
    pub fn new(_app_type: AppType, mgr: &mut Manager) -> Self {
        let display_id = mgr.new_display(true);
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
            display_id,
            width,
            height,
            options: tags,
            selected_option: 0,
            last_updated_row: 1,
        }
    }

    pub fn select(
        &mut self,
        header: &str,
        options: &Vec<String>,
        mut selected: Vec<usize>,
        mgr: &mut Manager,
        quit_on_first_select: bool,
    ) -> Vec<usize> {
        mgr.restore_display(self.display_id, true);
        eprintln!("Options to present: {:?}", options);
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let gp = Glyph::plain();
        let mut g = Glyph::plain();
        let filter_header = format!("Filter: ",);
        let mut filter = String::with_capacity(64);
        let mut filter_len = filter_header.len();
        let mut iter = header.chars();
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
        let buttons_len = self.options.len();
        let options_in_row = self.width >> 5;
        let opt_rows_count = (self.height >> 1) - 1;
        let opts_per_page = options_in_row * opt_rows_count;
        let mut option_pages = vec![];
        let mut page = Vec::with_capacity(opts_per_page);
        let mut total_added = 0;
        for i in 0..options_len {
            page.push(i);
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
        if option_pages.is_empty() {
            option_pages.push(vec![]);
        }
        let mut curr_page = 0;
        let mut options_count = total_added;
        let mut filter_updated = false;
        options_len = option_pages[curr_page].len();
        let mut opt_page = &option_pages[curr_page];
        let mut last_updated = 0;
        if let Some(opt_id) = opt_page.get(last_updated) {
            let sel = self.selected_option == last_updated;
            self.options[last_updated].update(*opt_id, options.get(*opt_id).unwrap(), mgr);
            self.options[last_updated].present(sel, selected.contains(opt_id), mgr);
        } else {
            self.options[last_updated].hide(mgr);
        }

        loop {
            if let Some(char) = mgr.read_char() {
                let mut action = Action::None;
                // eprintln!("A char read {:?}", char);
                if let Some(key) = map_private_char_to_key(char) {
                    // Now that we added filter and read char instead of Key
                    // there will be some control characters that we would like to use
                    // for navigation/selection instead of pushing them into filter string
                    // So we might define an Action enum and map a char or key into that
                    action = match key {
                        Key::AltTab => Action::Select,
                        Key::PgUp => Action::PrevPage,
                        Key::PgDn => Action::NextPage,
                        Key::Up => Action::MoveUp,
                        Key::Down => Action::MoveDown,
                        Key::Left => Action::MoveLeft,
                        Key::Right => Action::MoveRight,
                        _other => {
                            // eprintln!("Key {} not defined", key);
                            Action::None
                        }
                    };
                } else if char.is_control() {
                    action = match char {
                        //CtrlB
                        '\u{2}' => Action::MoveLeft,
                        //CtrlD
                        '\u{4}' => Action::NextPage,
                        //CtrlF
                        '\u{6}' => Action::MoveRight,
                        //CtrlJ or Enter
                        '\u{a}' => Action::Select,
                        //CtrlK
                        '\u{b}' => Action::ClearFilter,
                        //CtrlN
                        '\u{e}' => Action::MoveDown,
                        //CtrlP
                        '\u{10}' => Action::MoveUp,
                        //CtrlU
                        '\u{15}' => Action::PrevPage,
                        //Esc
                        '\u{1b}' => Action::Finish,
                        //Backspace
                        '\u{7f}' => Action::DelFromFilter,
                        other => {
                            eprintln!("Undefined control char: {:?}", other);
                            Action::None
                        }
                    }
                } else {
                    action = Action::AddToFilter(char);
                }

                // eprintln!("Selected action: {:?}", action);
                match action {
                    Action::Finish => {
                        self.selected_option = 0;
                        break;
                    }
                    Action::Select => {
                        let idx = self.options[self.selected_option].index;
                        if let Some(position) = selected.iter().position(|&x| x == idx) {
                            selected.remove(position);
                            self.options[self.selected_option].present(true, false, mgr);
                            // let index = option_pages[curr_page][self.selected_option];
                        } else if quit_on_first_select {
                            selected = vec![idx];
                            self.selected_option = 0;
                            break;
                        } else {
                            selected.push(idx);
                            self.options[self.selected_option].present(true, true, mgr);
                        }
                    }
                    Action::NextPage => {
                        if option_pages.len() <= 1 {
                            continue;
                        }
                        self.selected_option = 0;
                        curr_page += 1;
                        if curr_page >= option_pages.len() {
                            curr_page = 0;
                        }
                        options_len = option_pages[curr_page].len();
                        opt_page = &option_pages[curr_page];
                        last_updated = 0;
                        if let Some(opt_id) = opt_page.get(last_updated) {
                            let sel = self.selected_option == last_updated;
                            self.options[last_updated].update(
                                *opt_id,
                                options.get(*opt_id).unwrap(),
                                mgr,
                            );
                            self.options[last_updated].present(sel, selected.contains(opt_id), mgr);
                        } else {
                            self.options[last_updated].hide(mgr);
                        }
                    }
                    Action::PrevPage => {
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
                        opt_page = &option_pages[curr_page];
                        last_updated = 0;
                        if let Some(opt_id) = opt_page.get(last_updated) {
                            let sel = self.selected_option == last_updated;
                            self.options[last_updated].update(
                                *opt_id,
                                options.get(*opt_id).unwrap(),
                                mgr,
                            );
                            self.options[last_updated].present(sel, selected.contains(opt_id), mgr);
                        } else {
                            self.options[last_updated].hide(mgr);
                        }
                    }
                    Action::MoveUp => {
                        if options_len == 0 {
                            continue;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            false,
                            selected.contains(&index),
                            mgr,
                        );
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
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            true,
                            selected.contains(&index),
                            mgr,
                        );
                    }
                    Action::MoveDown => {
                        if options_len == 0 {
                            continue;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            false,
                            selected.contains(&index),
                            mgr,
                        );
                        self.selected_option += options_in_row;
                        if self.selected_option >= options_len {
                            self.selected_option = self.selected_option % options_in_row;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            true,
                            selected.contains(&index),
                            mgr,
                        );
                    }
                    Action::MoveRight => {
                        if options_len == 0 {
                            continue;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            false,
                            selected.contains(&index),
                            mgr,
                        );
                        self.selected_option += 1;
                        if self.selected_option >= options_len {
                            self.selected_option = 0;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            true,
                            selected.contains(&index),
                            mgr,
                        );
                    }
                    Action::MoveLeft => {
                        if options_len == 0 {
                            continue;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            false,
                            selected.contains(&index),
                            mgr,
                        );
                        if self.selected_option == 0 {
                            self.selected_option = options_len - 1;
                        } else {
                            self.selected_option -= 1;
                        }
                        let index = option_pages[curr_page][self.selected_option];
                        self.options[self.selected_option].present(
                            true,
                            selected.contains(&index),
                            mgr,
                        );
                    }
                    Action::AddToFilter(char) => {
                        eprintln!(
                            "updating filter with: '{:?}',alfa_num:{}, ctrl:{}",
                            char,
                            char.is_alphanumeric(),
                            char.is_control()
                        );
                        filter_updated = true;
                        // TODO: every time we modify filter's contents we need to recalculate
                        //       what options are going to be displayed on screen
                        //       if option count has changed we need to update displayed options
                        //       when necessary
                        //       Sometimes the number of options does change, but all the options
                        //       that are currently displayed should stay unchanged.
                        //       Sometimes first n options does not change, so we only need to
                        //       update following options
                        // TODO: in order to implement above we need to compare every Option's
                        //       current id with updated id, and act only if those are different.
                        filter.push(char);
                        filter_len += 1;

                        // TODO: and then compare currently displayed page with new corresponding
                        //       page and update options starting from first difference onwards
                        //       if ids match we skip given option and move forward
                        //       if we run out of new options, we hide remaining option buttons

                        g.set_char(char);
                        mgr.set_glyph(self.g_id, g, filter_len, 1);
                    }
                    Action::DelFromFilter => {
                        if filter_len > filter_header.len() {
                            filter_updated = true;
                            filter.pop();
                            g.set_char(' ');
                            mgr.set_glyph(self.g_id, g, filter_len, 1);
                            filter_len = filter_len - 1;
                        }
                    }
                    Action::ClearFilter => {
                        while filter_len > filter_header.len() {
                            filter_updated = true;
                            filter.pop();
                            g.set_char(' ');
                            mgr.set_glyph(self.g_id, g, filter_len, 1);
                            filter_len = filter_len - 1;
                        }
                    }

                    Action::None => {
                        // eprintln!("Other key pressed: {:?}", other);
                    }
                }
            }
            if filter_updated {
                let filter_words: Vec<_> = filter.split_whitespace().collect();
                // eprintln!("Filter words:");
                // for word in filter_words.iter() {
                //     eprintln!("W: {}", word);
                // }
                let filtered_options = if filter_words.is_empty() {
                    let mut f_o = Vec::with_capacity(options_count);
                    for i in 0..options_count {
                        f_o.push(i);
                    }
                    f_o
                } else {
                    options
                        .iter()
                        .enumerate()
                        .filter(|(_i, x)| {
                            let mut contains = false;
                            for word in &filter_words {
                                if x.contains(word) {
                                    contains = true;
                                    break;
                                }
                            }
                            contains
                        })
                        .map(|(i, _x)| i)
                        .collect::<Vec<_>>()
                };
                eprintln!("options count has changed: {}", filtered_options.len());
                option_pages = vec![];
                let pages_iter = filtered_options.chunks_exact(buttons_len);
                let leftover = pages_iter.remainder();
                for option_page in pages_iter {
                    option_pages.push(Vec::from(option_page));
                }
                if !leftover.is_empty() {
                    let mut more = Vec::with_capacity(leftover.len());
                    for id in leftover {
                        more.push(*id);
                    }
                    option_pages.push(more);
                }
                if option_pages.is_empty() {
                    eprintln!("no opts!");
                    option_pages.push(vec![]);
                }
                curr_page = 0;
                self.selected_option = 0;
                opt_page = &option_pages[curr_page];
                options_len = opt_page.len();
                last_updated = 0;
                if let Some(opt_id) = opt_page.get(last_updated) {
                    let sel = self.selected_option == last_updated;
                    self.options[last_updated].update(*opt_id, options.get(*opt_id).unwrap(), mgr);
                    self.options[last_updated].present(sel, selected.contains(opt_id), mgr);
                } else {
                    self.options[last_updated].hide(mgr);
                }
                filter_updated = false;
            }
            if last_updated + 1 < buttons_len {
                last_updated = last_updated + 1;
                if let Some(opt_idx) = opt_page.get(last_updated) {
                    let sel = self.selected_option == last_updated;
                    // if self.options[last_updated].index != *opt_idx {
                    self.options[last_updated].update(
                        *opt_idx,
                        options.get(*opt_idx).unwrap(),
                        mgr,
                    );
                    // }
                    self.options[last_updated].present(sel, selected.contains(opt_idx), mgr);
                } else {
                    self.options[last_updated].hide(mgr);
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
