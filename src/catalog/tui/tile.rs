use std::path::PathBuf;

use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Manager;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;

#[derive(Copy, Clone, Debug)]
pub enum TileType {
    Home(GnomeId),
    Neighbor(GnomeId),
    Field,
    Application,
    Content(DataType, ContentID),
}
impl TileType {
    pub fn is_content(&self, c_id: ContentID) -> bool {
        match self {
            Self::Content(_d_type, content_id) => *content_id == c_id,
            _other => false,
        }
    }
    pub fn is_home(&self) -> bool {
        match self {
            Self::Home(_id) => true,
            _other => false,
        }
    }
    pub fn is_field(&self) -> bool {
        match self {
            Self::Field => true,
            _other => false,
        }
    }
}

pub struct Tile {
    //TODO
    pub id: usize,
    pub text_id: usize,
    pub tile_type: TileType,
    pub select_frame: usize,
    pub deselect_frame: usize,
}
impl Tile {
    pub fn new(offset: (isize, isize), mgr: &mut Manager, asset_dir: &PathBuf) -> Self {
        let c_path = asset_dir.join("content.g");
        let c_graphic = Graphic::from_file(c_path).unwrap();
        let id = mgr.add_graphic(c_graphic, 2, offset).unwrap();
        mgr.set_graphic(id, 0, true);
        let t_path = asset_dir.join("content_text.g");
        let t_graphic = Graphic::from_file(t_path).unwrap();
        let text_id = mgr.add_graphic(t_graphic, 3, offset).unwrap();
        mgr.set_graphic(text_id, 0, true);
        Tile {
            id,
            text_id,
            tile_type: TileType::Field,
            select_frame: 1,
            deselect_frame: 0,
        }
    }

    fn update_tile_text(&self, optional_text: Option<String>, mgr: &mut Manager) {
        mgr.set_graphic(self.text_id, 0, false);
        if let Some(text) = optional_text {
            let words = text.split_whitespace();
            let g = Glyph::transparent();
            let mut gt = Glyph::char(' ');
            let mut test_frame = vec![g; 84];
            let mut curr_line = 1;
            for word in words {
                for (i, char) in word.chars().enumerate() {
                    if i >= 10 {
                        continue;
                    }
                    gt.set_char(char);
                    test_frame[curr_line * 12 + 1 + i] = gt.clone();
                }
                curr_line += 1;
                if curr_line >= 6 {
                    break;
                }
            }
            // for (i, char) in text.chars().enumerate() {
            //     if i >= 84 {
            //         break;
            //     }
            //     gt.set_char(char);
            //     test_frame[i] = gt.clone();
            // }
            if let Some(_old_frame) = mgr.swap_frame(self.text_id, 1, test_frame) {
                mgr.set_graphic(self.text_id, 1, false);
            }
        }
    }

    pub fn set_to_home(
        &mut self,
        owner_id: GnomeId,
        selected: bool,
        is_my: bool,
        // optional_text: Option<String>,
        mgr: &mut Manager,
    ) {
        if is_my {
            self.select_frame = 9;
            self.deselect_frame = 8;
        } else {
            self.select_frame = 3;
            self.deselect_frame = 2;
        }

        self.update_tile_text(Some(format!("{}", owner_id)), mgr);
        self.tile_type = TileType::Home(owner_id);
        if selected {
            mgr.set_graphic(self.id, self.select_frame, false);
        } else {
            mgr.set_graphic(self.id, self.deselect_frame, false);
        }
    }
    pub fn set_to_neighbor(&mut self, n_id: GnomeId, selected: bool, mgr: &mut Manager) {
        // TODO: make use of n_id
        self.update_tile_text(Some(format!("Nejbor {:x}", n_id.0)), mgr);
        self.select_frame = 7;
        self.deselect_frame = 6;
        self.tile_type = TileType::Neighbor(n_id);
        if selected {
            mgr.set_graphic(self.id, self.select_frame, false);
        } else {
            mgr.set_graphic(self.id, self.deselect_frame, false);
        }
    }
    pub fn set_to_content(
        &mut self,
        description: Option<String>,
        d_type: DataType,
        c_id: ContentID,
        selected: bool,
        mgr: &mut Manager,
    ) {
        self.update_tile_text(description, mgr);
        self.select_frame = 5;
        self.deselect_frame = 4;
        self.tile_type = TileType::Content(d_type, c_id);
        if selected {
            mgr.set_graphic(self.id, self.select_frame, false);
        } else {
            mgr.set_graphic(self.id, self.deselect_frame, false);
        }
    }
    pub fn set_to_field(&mut self, mgr: &mut Manager) {
        self.update_tile_text(None, mgr);
        self.select_frame = 1;
        self.deselect_frame = 0;
        self.tile_type = TileType::Field;
        mgr.set_graphic(self.id, self.deselect_frame, false);
    }
}
