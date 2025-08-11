use crate::catalog::logic::Manifest;
use animaterm::prelude::Glyph;
use animaterm::prelude::Graphic;
use animaterm::prelude::Manager;
use dapp_lib::prelude::DataType;
use dapp_lib::Data;
use std::collections::HashMap;

// TODO: Use Adder for adding new DataType or Tag
pub struct Viewer {
    g_id: usize,
    width: usize,
    height: usize,
}

impl Viewer {
    pub fn new(mgr: &mut Manager) -> Self {
        let (width, height) = mgr.screen_size();
        let (mut desired_width, mut desired_height) = (60, 8);
        if width < desired_width {
            desired_width = width;
        }
        if height < desired_height {
            desired_height = height;
        }
        let g = Glyph::char(' ');
        let frame = vec![g; desired_width * desired_height];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let g_id = mgr
            .add_graphic(
                Graphic::new(desired_width, desired_height, 0, library, None),
                0,
                (
                    (width - desired_width) as isize / 2,
                    (height - desired_height) as isize / 2,
                ),
            )
            .unwrap();
        Viewer {
            g_id,
            width: desired_width,
            height: desired_height,
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
            &format!(
                "Define new DataType                       bytes left: {}",
                30
            )
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
        let t_text = "Descpription: Plaintext file";
        let mut char_iter = t_text.chars();
        for x in 2..self.width {
            if let Some(c) = char_iter.next() {
                g.set_char(c);
                mgr.set_glyph(self.g_id, g, x, 3);
            } else {
                mgr.set_glyph(self.g_id, p, x, 3);
            }
        }
        let b_text = "                    APPLY    CANCEL";
        let mut char_iter = b_text.chars();
        let y = self.height - 2;
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
