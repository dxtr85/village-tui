use animaterm::prelude::*;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::ToUser;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub struct Tile {
    //TODO
    pub id: usize,
}
impl Tile {
    pub fn new(offset: (isize, isize), mgr: &mut Manager) -> Self {
        let c_graphic =
            Graphic::from_file("/home/dxtr/projects/village-tui/assets/content.g").unwrap();
        let id = mgr.add_graphic(c_graphic, 1, offset).unwrap();
        mgr.set_graphic(id, 0, true);
        Tile { id }
    }
}

pub struct VillageLayout {
    tiles_in_row: u8,
    visible_rows: u8,
    selected_tile: (u8, u8),
    tiles: HashMap<(u8, u8), Tile>,
}

impl VillageLayout {
    pub fn new((x_size, y_size): (usize, usize)) -> Self {
        let tiles_in_row: u8 = (x_size / 12) as u8;
        let visible_rows: u8 = (y_size / 8) as u8 + 1;
        VillageLayout {
            tiles_in_row,
            visible_rows,
            selected_tile: (2, 1),
            tiles: HashMap::new(),
        }
    }

    pub fn initialize(&mut self, mgr: &mut Manager) {
        //TODO
        for col in 0..self.tiles_in_row {
            for row in 0..self.visible_rows {
                let off_x: isize = if col > 1 {
                    4 + (col as isize) * 12
                } else {
                    col as isize * 12
                };
                let off_y: isize = if row % 2 == 0 {
                    row as isize * 7
                } else if row > 1 {
                    row as isize * 8 - 2
                } else {
                    row as isize * 8
                };
                let tile = Tile::new((off_x, off_y), mgr);
                self.tiles.insert((row, col), tile);
            }
        }
    }

    pub fn select(&mut self, index: &(u8, u8), mgr: &mut Manager) {
        if let Some(tile) = self.tiles.get(index) {
            if let Some(old_tile) = self.tiles.get(&self.selected_tile) {
                mgr.set_graphic(old_tile.id, 0, false);
            }
            mgr.set_graphic(tile.id, 1, false);
            self.selected_tile = *index;
        }
    }
    pub fn select_next(&mut self, direction: Direction, mgr: &mut Manager) {
        let new_index = match direction {
            Direction::Left => {
                let new_col = if self.selected_tile.1 == 0 {
                    self.tiles_in_row - 1
                } else {
                    self.selected_tile.1 - 1
                };
                (self.selected_tile.0, new_col)
            }
            Direction::Right => {
                let new_col = if self.selected_tile.1 + 1 == self.tiles_in_row {
                    0
                } else {
                    self.selected_tile.1 + 1
                };
                (self.selected_tile.0, new_col)
            }
            Direction::Up => {
                let new_row = if self.selected_tile.0 == 0 {
                    self.visible_rows - 1
                } else {
                    self.selected_tile.0 - 1
                };
                (new_row, self.selected_tile.1)
            }
            Direction::Down => {
                let new_row = if self.selected_tile.0 + 1 == self.visible_rows {
                    0
                } else {
                    self.selected_tile.0 + 1
                };
                (new_row, self.selected_tile.1)
            }
        };
        if let Some(tile) = self.tiles.get(&new_index) {
            if let Some(old_tile) = self.tiles.get(&self.selected_tile) {
                mgr.set_graphic(old_tile.id, 0, false);
            }
            mgr.set_graphic(tile.id, 1, false);
            self.selected_tile = new_index;
        }
    }
}
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub enum ToTui {
    Neighbors(String, Vec<GnomeId>),
    MoveSelection(Direction),
    AddContent(ContentID, DataType),
}

pub enum FromTui {
    KeyPress(Key),
}

pub fn instantiate_tui_mgr() -> Manager {
    let capture_keyboard = true;
    let cols = None;
    let rows = None; // use all rows available
    let glyph = Some(Glyph::default());
    let refresh_timeout = Some(Duration::from_millis(10));
    let mut mgr = Manager::new(
        capture_keyboard,
        cols,
        rows,
        glyph,
        refresh_timeout,
        Some(vec![(Key::AltM, MacroSequence::empty())]),
    );
    mgr
}

/// This function is responsible for sending keys to application and displaying user interface.
/// It is application's responsibility to interpret key presses and in response send
/// ToTui messages informing what action should be performed by presentation layer.
pub fn serve_tui_mgr(mut mgr: Manager, to_app: Sender<FromTui>, to_tui_recv: Receiver<ToTui>) {
    let s_size = mgr.screen_size();
    let mut village = VillageLayout::new(s_size);
    village.initialize(&mut mgr);
    village.select(&village.selected_tile.clone(), &mut mgr);

    let mut content_mapping = HashMap::<usize, (ContentID, DataType)>::new();
    eprintln!("Serving TUI Manager scr size: {}x{}", s_size.0, s_size.1);
    // let frame = Graphic::from_file("/home/dxtr/projects/village-tui/assets/frame.g").unwrap();
    // let frame_id = mgr.add_graphic(frame, 1, (12, 14)).unwrap();
    // mgr.set_graphic(frame_id, 0, true);
    // let graphic = Graphic::from_file("/home/dxtr/projects/village-tui/assets/house.g").unwrap();
    // let house_id = mgr.add_graphic(graphic, 0, (12, 14)).unwrap();
    // mgr.set_graphic(house_id, 0, true);

    loop {
        if let Some(key) = mgr.read_key() {
            let terminate = key == Key::Q || key == Key::ShiftQ;
            let res = to_app.send(FromTui::KeyPress(key));
            if res.is_err() || terminate {
                break;
            }
        }
        if let Ok(to_tui) = to_tui_recv.try_recv() {
            match to_tui {
                ToTui::Neighbors(s_name, neighbors) => {
                    //TODO build a graphic for every neighbor
                    for (i, neighbor) in neighbors.into_iter().enumerate() {
                        let graphic =
                            Graphic::from_file("/home/dxtr/projects/village-tui/assets/house.g")
                                .unwrap();
                        let x_offset = 31 + (i * 12) as isize;
                        let house_id = mgr.add_graphic(graphic, 0, (x_offset, 19)).unwrap();
                        mgr.set_graphic(house_id, 0, true);
                    }
                }
                ToTui::AddContent(c_id, d_type) => {
                    //TODO: we need a mapping between graphic id and ContentID
                    // When application asks for item currently selected we return ContentID
                    // or GnomeId if a Neighbor is selected
                    let c_graphic =
                        Graphic::from_file("/home/dxtr/projects/village-tui/assets/content.g")
                            .unwrap();
                    let c_gr_id = mgr.add_graphic(c_graphic, 1, (19, 12)).unwrap();
                    mgr.set_graphic(c_gr_id, 0, true);
                    content_mapping.insert(c_gr_id, (c_id, d_type));
                }
                ToTui::MoveSelection(dir) => village.select_next(dir, &mut mgr),
            }
        }
    }
    mgr.terminate();
}
