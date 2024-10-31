use crate::logic::Manifest;
use crate::logic::Tag;
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
        Creator {
            g_id,
            width,
            height,
        }
    }

    // TODO: we need a mapping from DataType -> String so that we can present it properly
    pub fn show(
        &mut self,
        mgr: &mut Manager,
        manifest: &Manifest,
        read_only: bool,
        d_type: DataType,
        tags: Vec<u8>,
        description: String,
        d_type_map: &HashMap<DataType, String>,
    ) -> Option<(DataType, Data)> {
        let d_text = if let Some(text) = d_type_map.get(&d_type) {
            &format!("EDIT    DataType: {}", text)
        } else {
            "EDIT    DataType: Unknown"
        };
        let p = Glyph::plain();
        let mut g = Glyph::plain();
        let mut char_iter = d_text.chars();
        for x in 2..self.width {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 1);
            } else {
                mgr.set_glyph(self.g_id, p, x, 1);
            }
        }
        let t_text = "EDIT    Tags: Main Games Entertainment G:Kraków";
        let mut char_iter = t_text.chars();
        for x in 2..self.width {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 3);
            } else {
                mgr.set_glyph(self.g_id, p, x, 3);
            }
        }
        let s_text = "EDIT    Description:  This is an example of a description.";
        let mut char_iter = s_text.chars();
        for x in 2..self.width {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 10);
            } else {
                mgr.set_glyph(self.g_id, p, x, 10);
            }
        }
        let b_text = "APPLY    CANCEL";
        let mut char_iter = b_text.chars();
        let y = self.height - 3;
        for x in 2..self.width {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, y);
            } else {
                mgr.set_glyph(self.g_id, p, x, y);
            }
        }

        mgr.move_graphic(self.g_id, 3, (0, 0));
        loop {
            if let Some(_key) = mgr.read_key() {
                break;
            }
        }
        mgr.move_graphic(self.g_id, 0, (0, 0));
        if read_only {
            None
        } else {
            Some((DataType::Data(0), Data::empty(0)))
        }
    }
}
