use crate::tui::tag;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Key;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use std::collections::HashMap;
use tag::TagTui;

use crate::logic::Manifest;

pub struct ManifestTui {
    g_id: usize,
    width: usize,
    height: usize,
    tags: Vec<TagTui>,
    selected_tag: usize,
    last_updated_row: usize,
}

impl ManifestTui {
    pub fn new(_app_type: AppType, mgr: &mut Manager) -> Self {
        let (cols, rows) = mgr.screen_size();
        let width = cols;
        let height = rows;
        let tags_in_row = (width >> 5) as isize;
        let g = Glyph::char(' ');
        let frame = vec![g; width * height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(Graphic::new(width, height, 0, library, None), 0, (0, 0))
            .unwrap();
        let mut tags = Vec::new();
        for y in (2..height).step_by(2) {
            for x in 0..tags_in_row {
                tags.push(TagTui::new((x * 33, y as isize), mgr));
            }
        }
        ManifestTui {
            g_id,
            width,
            height,
            tags,
            // selected_tag: usize::MAX,
            selected_tag: 0,
            last_updated_row: 1,
        }
    }

    pub fn present(&mut self, manifest: Manifest, mgr: &mut Manager) {
        mgr.move_graphic(self.g_id, 3, (0, 0));
        let gp = Glyph::plain();
        let mut g = Glyph::plain();
        let text = format!(
            "APPLICATION: {:?}, {} Tags   ",
            manifest.app_type,
            manifest.tags.len(),
        );
        let mut iter = text.chars();
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
        let mut tags_len = manifest.tags.len();
        let tags_in_row = (self.width >> 5);
        let tag_rows_count = ((self.height >> 1) - 1);
        let tags_per_page = tags_in_row * tag_rows_count;
        let mut tag_pages = vec![];
        let mut page = Vec::with_capacity(tags_per_page);
        let mut total_added = 0;
        for i in 0..=255 {
            if let Some(tag) = manifest.tags.get(&i) {
                page.push((i, tag));
                total_added += 1;
                if total_added % tags_per_page == 0 {
                    tag_pages.push(page);
                    page = Vec::with_capacity(tags_per_page);
                }
                if total_added >= tags_len {
                    if !page.is_empty() {
                        tag_pages.push(page);
                    }
                    break;
                }
            }
        }
        if tag_pages.is_empty() {
            tag_pages.push(vec![]);
        }
        let mut curr_page = 0;
        tags_len = tag_pages[curr_page].len();
        let mut tag_iter = tag_pages[curr_page].iter();
        for p_id in 0..self.tags.len() {
            if let Some((k, v)) = &tag_iter.next() {
                let sel = self.selected_tag == p_id;
                let activ = false;
                self.tags[p_id].update(*k, &v.0, mgr);
                self.tags[p_id].present(sel, activ, mgr);
            } else {
                self.tags[p_id].hide(mgr);
            }
        }
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    Key::Escape => {
                        self.selected_tag = 0;
                        break;
                    }
                    Key::PgDn => {
                        if tag_pages.len() <= 1 {
                            continue;
                        }
                        self.selected_tag = 0;
                        curr_page += 1;
                        if curr_page >= tag_pages.len() {
                            curr_page = 0;
                        }
                        tags_len = tag_pages[curr_page].len();
                        let mut tag_iter = tag_pages[curr_page].iter();
                        for p_id in 0..self.tags.len() {
                            if let Some((k, v)) = &tag_iter.next() {
                                let sel = self.selected_tag == p_id;
                                let activ = false;
                                self.tags[p_id].hide(mgr);
                                self.tags[p_id].update(*k, &v.0, mgr);
                                self.tags[p_id].present(sel, activ, mgr);
                            } else {
                                self.tags[p_id].hide(mgr);
                            }
                        }
                    }
                    Key::PgUp => {
                        if tag_pages.len() <= 1 {
                            continue;
                        }
                        if curr_page == 0 {
                            curr_page = tag_pages.len() - 1;
                        } else {
                            curr_page -= 1;
                        }
                        self.selected_tag = 0;
                        tags_len = tag_pages[curr_page].len();
                        let mut tag_iter = tag_pages[curr_page].iter();
                        for p_id in 0..self.tags.len() {
                            if let Some((k, v)) = &tag_iter.next() {
                                let sel = self.selected_tag == p_id;
                                let activ = false;
                                self.tags[p_id].hide(mgr);
                                self.tags[p_id].update(*k, &v.0, mgr);
                                self.tags[p_id].present(sel, activ, mgr);
                            } else {
                                self.tags[p_id].hide(mgr);
                            }
                        }
                    }
                    Key::Up => {
                        if tags_len == 0 {
                            continue;
                        }
                        self.tags[self.selected_tag].present(false, false, mgr);
                        if tags_len >= tags_in_row {
                            if self.selected_tag >= tags_in_row {
                                self.selected_tag -= tags_in_row;
                            } else {
                                let mut new_positions = Vec::with_capacity(tags_in_row);
                                for i in 0..tags_in_row {
                                    new_positions.push(tags_len - tags_in_row + i);
                                }
                                new_positions.rotate_right(tags_len % tags_in_row);
                                self.selected_tag = new_positions[self.selected_tag];
                            }
                        }
                        self.tags[self.selected_tag].present(true, false, mgr);
                    }
                    Key::Down => {
                        if tags_len == 0 {
                            continue;
                        }
                        self.tags[self.selected_tag].present(false, false, mgr);
                        self.selected_tag += tags_in_row;
                        if self.selected_tag >= tags_len {
                            self.selected_tag = self.selected_tag %   
                            tags_in_row;
                    }
                        self.tags[self.selected_tag].present(true, false, mgr);
                    }
                    Key::Right => {
                        if tags_len == 0 {
                            continue;
                        }
                        self.tags[self.selected_tag].present(false, false, mgr);
                        self.selected_tag += 1;
                        if self.selected_tag >= tags_len {
                            self.selected_tag = 0;
                        }
                        self.tags[self.selected_tag].present(true, false, mgr);
                    }
                    Key::Left => {
                        if tags_len == 0 {
                            continue;
                        }
                        self.tags[self.selected_tag].present(false, false, mgr);
                        if self.selected_tag == 0 {
                            self.selected_tag = tags_len - 1;
                        } else {
                            self.selected_tag -= 1;
                        }
                        self.tags[self.selected_tag].present(true, false, mgr);
                    }
                    _other => {}
                }
            }
        }
        for tag in &mut self.tags {
            tag.hide(mgr);
        }
        mgr.move_graphic(self.g_id, 0, (0, 0));
    }
}
