use animaterm::Graphic;
use animaterm::Manager;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;

#[derive(Copy, Clone)]
pub enum TileType {
    Home(GnomeId),
    Neighbor(GnomeId),
    Field,
    Application,
    Content(DataType, ContentID),
}

pub struct Tile {
    //TODO
    pub id: usize,
    pub tile_type: TileType,
    pub select_frame: usize,
    pub deselect_frame: usize,
}
impl Tile {
    pub fn new(offset: (isize, isize), mgr: &mut Manager) -> Self {
        let c_graphic =
            Graphic::from_file("/home/dxtr/projects/village-tui/assets/content.g").unwrap();
        let id = mgr.add_graphic(c_graphic, 2, offset).unwrap();
        mgr.set_graphic(id, 0, true);
        Tile {
            id,
            tile_type: TileType::Field,
            select_frame: 1,
            deselect_frame: 0,
        }
    }

    pub fn set_to_home(
        &mut self,
        owner_id: GnomeId,
        selected: bool,
        is_my: bool,
        mgr: &mut Manager,
    ) {
        if is_my {
            self.select_frame = 9;
            self.deselect_frame = 8;
        } else {
            self.select_frame = 3;
            self.deselect_frame = 2;
        }
        self.tile_type = TileType::Home(owner_id);
        if selected {
            mgr.set_graphic(self.id, self.select_frame, false);
        } else {
            mgr.set_graphic(self.id, self.deselect_frame, false);
        }
    }
    pub fn set_to_neighbor(&mut self, n_id: GnomeId, selected: bool, mgr: &mut Manager) {
        // TODO: make use of n_id
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
        d_type: DataType,
        c_id: ContentID,
        selected: bool,
        mgr: &mut Manager,
    ) {
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
        self.select_frame = 1;
        self.deselect_frame = 0;
        self.tile_type = TileType::Field;
        mgr.set_graphic(self.id, self.deselect_frame, false);
    }
}
