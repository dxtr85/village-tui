use crate::input::Input;
use crate::logic::Manifest;
use animaterm::prelude::*;
use animaterm::utilities::message_box;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::Data;
use std::collections::HashMap;
use std::fmt::format;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
mod ask;
mod content_creator;
mod context_menu;
mod manifest;
mod tag;
mod tile;
use crate::logic::Tag;
use ask::Question;
use content_creator::Creator;
use context_menu::CMenu;
use manifest::ManifestTui;
use tile::Tile;
use tile::TileType;

pub struct VillageLayout {
    my_id: GnomeId,
    tiles_in_row: u8,
    visible_rows: u8,
    home_tile: (u8, u8),
    selected_tile: (u8, u8),
    tiles: HashMap<(u8, u8), Tile>,
    neighbors: HashMap<GnomeId, (u8, u8)>,
}

impl VillageLayout {
    pub fn new((x_size, y_size): (usize, usize)) -> Self {
        let tiles_in_row: u8 = (x_size / 12) as u8;
        let visible_rows: u8 = (y_size / 8) as u8 + 1;
        VillageLayout {
            my_id: GnomeId::any(),
            tiles_in_row,
            visible_rows,
            home_tile: (2, 1),
            selected_tile: (2, 1),
            tiles: HashMap::new(),
            neighbors: HashMap::new(),
        }
    }

    pub fn initialize(&mut self, owner_id: GnomeId, mgr: &mut Manager) {
        self.my_id = owner_id;
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
        if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
            tile.set_to_home(owner_id, true, owner_id == self.my_id, mgr);
        }
    }
    pub fn reset_tiles(&mut self, owner_id: GnomeId, mgr: &mut Manager) {
        for tile in self.tiles.values_mut() {
            tile.set_to_field(mgr);
        }
        if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
            tile.set_to_home(owner_id, false, owner_id == self.my_id, mgr);
        }
        self.selected_tile = self.home_tile;
    }

    pub fn get_tile_headers(&self) -> Vec<((u8, u8), TileType)> {
        let mut result =
            Vec::with_capacity(self.tiles_in_row as usize * self.visible_rows as usize);
        for (slot, tile) in self.tiles.iter() {
            result.push((*slot, tile.tile_type.clone()));
        }
        result
    }
    pub fn get_owner(&self) -> Option<GnomeId> {
        if let Some(tile) = self.tiles.get(&self.home_tile) {
            if let TileType::Home(owner) = tile.tile_type {
                return Some(owner);
            }
        }
        None
    }

    pub fn set_owner(&mut self, owner_id: GnomeId, mgr: &mut Manager) {
        if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
            if let TileType::Home(curr_owner) = tile.tile_type {
                if curr_owner.is_any() {
                    tile.set_to_home(owner_id, false, owner_id == self.my_id, mgr);
                } else {
                    eprintln!(
                        "Attempt to change owner from: {} to {}",
                        curr_owner, owner_id
                    );
                }
            }
        }
    }

    pub fn select(&mut self, idx: &(u8, u8), mgr: &mut Manager) {
        if let Some(tile) = self.tiles.get(idx) {
            if let Some(old_tile) = self.tiles.get(&self.selected_tile) {
                mgr.set_graphic(old_tile.id, old_tile.deselect_frame, false);
            }
            mgr.set_graphic(tile.id, tile.select_frame, false);
            self.selected_tile = *idx;
        }
    }
    pub fn get_selection(&self) -> TileType {
        self.tiles.get(&self.selected_tile).unwrap().tile_type
    }
    pub fn add_new_neighbor(&mut self, n_id: GnomeId, mgr: &mut Manager) -> (u8, u8) {
        if let Some(tile_location) = self.neighbors.get(&n_id) {
            *tile_location
        } else {
            let tile_id = self.next_field_tile();
            self.neighbors.insert(n_id, tile_id);
            let tile = self.tiles.get_mut(&tile_id).unwrap();
            tile.set_to_neighbor(n_id, false, mgr);
            tile_id
        }
    }

    pub fn add_new_content(
        &mut self,
        d_type: DataType,
        c_id: ContentID,
        mgr: &mut Manager,
    ) -> (u8, u8) {
        let tile_id = self.next_field_tile();
        eprintln!("Next id: {:?}", tile_id);
        let tile = self.tiles.get_mut(&tile_id).unwrap();
        tile.set_to_content(d_type, c_id, false, mgr);
        tile_id
    }
    fn next_field_tile(&self) -> (u8, u8) {
        for (k, v) in &self.tiles {
            if matches!(v.tile_type, TileType::Field) {
                return *k;
            }
        }
        return (0, 0);
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
                mgr.set_graphic(old_tile.id, old_tile.deselect_frame, false);
            }
            mgr.set_graphic(tile.id, tile.select_frame, false);
            self.selected_tile = new_index;
        }
    }

    fn cm_position(&self) -> (isize, isize) {
        let x = if self.selected_tile.1 >= self.tiles_in_row - 1 {
            (self.tiles_in_row as isize - 1) * 12
        } else {
            (self.selected_tile.1 as isize + 1) * 12
        };
        let y = if self.selected_tile.0 + 1 >= self.visible_rows {
            (self.selected_tile.0 as isize - 1) * 8
        } else {
            (self.selected_tile.0 as isize) * 8
        };
        eprintln!("sel: {:?}, (x,y): {:?}", self.selected_tile, (x, y));
        (x, y)
    }
}
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub enum ToPresentation {
    // Owner(GnomeId),
    Neighbors(Vec<GnomeId>),
    // MoveSelection(Direction),
    AppendContent(ContentID, DataType),
    Contents(ContentID, String),
    ContentsNotExist(ContentID),
    Manifest(Manifest),
}

#[derive(Clone)]
pub enum FromPresentation {
    //TODO: we don't want to send Keys out of TUI, rather instructions depending on
    // context where given key was pressed
    AddTags(Vec<Tag>),
    NewUserEntry(String),
    ContentInquiry(ContentID),
    NeighborSelected(GnomeId),
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
    mgr.set_key_receive_timeout(Duration::from_millis(16));
    let (cols, rows) = mgr.screen_size();
    let frame = vec![Glyph::plain(); cols * rows];
    let mut library = HashMap::new();
    library.insert(0, frame);
    let bg = Graphic::new(cols, rows, 0, library, None);
    mgr.add_graphic(bg, 1, (0, 0)).unwrap();
    mgr
}

/// This function is for sending requests to application and displaying user interface.
pub fn serve_tui_mgr(
    my_id: GnomeId,
    mut mgr: Manager,
    to_app: Sender<FromPresentation>,
    to_tui_recv: Receiver<ToPresentation>,
) {
    let s_size = mgr.screen_size();
    let mut village = VillageLayout::new(s_size);
    let mut neighboring_villages = HashMap::new();
    village.initialize(my_id, &mut mgr);

    // let mut tiles_mapping = HashMap::<(u8, u8), TileType>::new();
    // eprintln!("Serving TUI Manager scr size: {}x{}", s_size.0, s_size.1);
    let main_display = 0;
    let input_display = mgr.new_display(true);
    let mut input = Input::new(&mut mgr);
    input.show(&mut mgr);
    mgr.restore_display(main_display, true);
    let mut c_menu = CMenu::new(&mut mgr);
    let mut creator = Creator::new(&mut mgr);
    let mut d_type_map = HashMap::new();
    d_type_map.insert(DataType::Data(0), "Text".to_string());
    d_type_map.insert(DataType::Data(1), "Text file".to_string());
    d_type_map.insert(DataType::Data(2), "Binary file".to_string());
    let question = Question::new(&mut mgr);
    let mut manifest = Manifest::new(AppType::Catalog, HashMap::new());
    let mut manifest_tui = ManifestTui::new(AppType::Catalog, &mut mgr);
    let set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " Manifest".to_string(),
            " Dwa".to_string(),
            " Trzy".to_string(),
            " Cztery".to_string(),
            " Pięć".to_string(),
            " Sześć".to_string(),
            " Siedem".to_string(),
            " Konstantynopol".to_string(),
        ],
    );
    loop {
        if let Some(key) = mgr.read_key() {
            let terminate = key == Key::Q || key == Key::ShiftQ;
            match key {
                Key::AltEnter | Key::Space => {
                    let action = c_menu.show(&mut mgr, set_id, village.cm_position());
                    eprintln!("Sel: {}", action);
                    match action {
                        1 => {
                            eprintln!("Requesting manifest");
                            let _ = to_app.send(FromPresentation::ContentInquiry(0));
                        }
                        other => {
                            print!("A: {}", other);
                        }
                    }
                }
                Key::C => {
                    let read_only = false;
                    let d_type = DataType::Data(0);
                    if let Some((d_type, data)) = creator.show(
                        &mut mgr,
                        &manifest,
                        read_only,
                        d_type,
                        vec![],
                        String::new(),
                        &d_type_map,
                    ) {
                        //TODO
                        eprintln!("We have some work to do…");
                    };
                }
                Key::T => {
                    //TODO
                    eprintln!("Sending add tag request from presentation layer");
                    let mut tags = Vec::with_capacity(256);
                    for i in 0..=255 {
                        tags.push(Tag::new(format!("Tag #{}", i)).unwrap());
                    }
                    let _ = to_app.send(FromPresentation::AddTags(tags));
                }
                Key::Left | Key::H | Key::CtrlB => village.select_next(Direction::Left, &mut mgr),
                Key::Right | Key::L | Key::CtrlF => village.select_next(Direction::Right, &mut mgr),
                Key::Up | Key::K | Key::CtrlP => village.select_next(Direction::Up, &mut mgr),

                Key::Down | Key::J | Key::CtrlN => village.select_next(Direction::Down, &mut mgr),
                Key::AltCtrlH => {
                    let _ = to_app.send(FromPresentation::NeighborSelected(village.my_id));
                    swap_tiles(
                        village.my_id,
                        &mut village,
                        &mut neighboring_villages,
                        &mut mgr,
                    );
                }
                Key::Home => {
                    let sel = village.home_tile;
                    village.select(&sel, &mut mgr);
                }
                Key::End => {
                    let sel = (village.selected_tile.0, village.tiles_in_row - 1);
                    village.select(&sel, &mut mgr);
                }
                Key::Enter => {
                    //TODO: we need to be aware of what context we are in to determine what
                    match village.get_selection() {
                        TileType::Home(owner) => {
                            if owner != village.my_id {
                                let action = c_menu.show(&mut mgr, 0, village.cm_position());
                                continue;
                            }
                            let typed_text =
                                serve_input(input_display, main_display, &mut input, &mut mgr);
                            eprintln!("Got: {}", typed_text);
                            let _ = to_app.send(FromPresentation::NewUserEntry(typed_text));
                        }
                        TileType::Neighbor(n_id) => {
                            let _ = to_app.send(FromPresentation::NeighborSelected(n_id));
                            swap_tiles(n_id, &mut village, &mut neighboring_villages, &mut mgr);
                        }
                        TileType::Field => {
                            print!("What?");
                        }
                        TileType::Content(_d_type, c_id) => {
                            // if let Some((c_id, d_type)) = tiles_mapping.get(&village.selected_tile)
                            // {
                            // println!("Something: {:?}", c_data);
                            let _ = to_app.send(FromPresentation::ContentInquiry(c_id));
                            // }
                        }
                        _ => {
                            //TODO
                        }
                    }
                }
                other => {
                    // eprintln!("Send to app: {} terminate: {}", other, terminate);
                    let res = to_app.send(FromPresentation::KeyPress(other));
                    if res.is_err() || terminate {
                        break;
                    }
                }
            }
        }
        if let Ok(to_tui) = to_tui_recv.try_recv() {
            match to_tui {
                ToPresentation::Neighbors(neighbors) => {
                    //TODO: first make sure neighbor is not placed on screen
                    for neighbor in neighbors.into_iter() {
                        let _tile_id = village.add_new_neighbor(neighbor, &mut mgr);
                    }
                }
                ToPresentation::AppendContent(c_id, d_type) => {
                    let tile_id = village.add_new_content(d_type, c_id, &mut mgr);
                    // tiles_mapping.insert(tile_id, TileType::Content(d_type, c_id));
                }
                ToPresentation::Contents(c_id, text) => {
                    //TODO: display text
                    let title = format!(" Content ID: {} ", c_id);
                    let m_box = message_box(Some(title), text, Glyph::plain(), 80, 24);
                    if let Some(g_id) = mgr.add_graphic(m_box, 3, (1, 1)) {
                        // println!("mbox added!");
                        mgr.set_graphic(g_id, 0, true);
                        let mut hide = false;
                        while !hide {
                            if let Some(key) = mgr.read_key() {
                                hide = true;
                            }
                        }
                        mgr.move_graphic(g_id, 0, (0, 0));
                        mgr.delete_graphic(g_id);
                    }
                }
                ToPresentation::Manifest(mani) => {
                    manifest = mani.clone();
                    manifest_tui.present(mani, &mut mgr);
                }
                ToPresentation::ContentsNotExist(c_id) => {
                    if question.ask(
                        &format!(
                            "Content {} was not created. Do you want to create one?",
                            c_id
                        ),
                        &mut mgr,
                    ) {
                        // let manifest = create_manifest(&mut mgr);
                        // TODO: send Data to Swarm
                    }
                }
            }
        }
    }
    eprintln!("Done serving TUI");
    mgr.terminate();
}

fn serve_input(
    input_display: usize,
    main_display: usize,
    input: &mut Input,
    mut mgr: &mut Manager,
) -> String {
    mgr.restore_display(input_display, true);
    eprint!("Type text in (press TAB to finish): ");
    // input.show(&mut mgr);
    loop {
        // TODO: here we need to build a text input window and add logic
        // for handling backspace, escape, delete...
        if let Some(ch) = mgr.read_char() {
            // eprintln!("Some ch: {}", ch);
            if let Some(key) = map_private_char_to_key(ch) {
                // eprintln!("Some key: {:?}", key);
                match key {
                    Key::Up => input.move_cursor(Direction::Up, &mut mgr),
                    Key::Down => input.move_cursor(Direction::Down, &mut mgr),
                    Key::Left => input.move_cursor(Direction::Left, &mut mgr),
                    Key::Right => input.move_cursor(Direction::Right, &mut mgr),
                    Key::Home => input.move_to_line_start(&mut mgr),
                    Key::End => input.move_to_line_end(&mut mgr),
                    Key::Delete => input.delete(&mut mgr),
                    Key::AltB => {
                        input.move_cursor(Direction::Left, &mut mgr);
                        input.move_cursor(Direction::Left, &mut mgr);
                        input.move_cursor(Direction::Left, &mut mgr);
                        input.move_cursor(Direction::Left, &mut mgr);
                    }
                    Key::AltF => {
                        input.move_cursor(Direction::Right, &mut mgr);
                        input.move_cursor(Direction::Right, &mut mgr);
                        input.move_cursor(Direction::Right, &mut mgr);
                        input.move_cursor(Direction::Right, &mut mgr);
                    }
                    other => eprint!("Other: {}", other),
                }
            } else if ch == '\t' {
                let taken = input.take_text(&mut mgr);
                mgr.restore_display(main_display, true);
                return taken;
            } else if ch == '\u{7f}' {
                input.backspace(&mut mgr);
            } else if ch == '\u{1}' {
                input.move_to_line_start(&mut mgr);
            } else if ch == '\u{5}' {
                input.move_to_line_end(&mut mgr);
            } else if ch == '\u{a}' {
                input.move_cursor(Direction::Down, &mut mgr);
                input.move_to_line_start(&mut mgr);
            } else if ch == '\u{b}' {
                input.remove_chars_from_cursor_to_end(&mut mgr);
            } else if ch == '\u{e}' {
                input.move_cursor(Direction::Down, &mut mgr);
            } else if ch == '\u{10}' {
                input.move_cursor(Direction::Up, &mut mgr);
            } else if ch == '\u{2}' {
                input.move_cursor(Direction::Left, &mut mgr);
            } else if ch == '\u{6}' {
                input.move_cursor(Direction::Right, &mut mgr);
            } else {
                // eprint!("code: {:?}", ch);
                input.insert(&mut mgr, ch);
            }
            // //Backspace
            // if ch == '\u{7f}' {
            //     eprint!(r"BS");
            //     // break;
            // }
            // //Escape
            // if ch == '\u{1b}' {
            //     eprint!("Esc");
            //     break;
            // }
        }
    }
}

fn swap_tiles(
    n_id: GnomeId,
    village: &mut VillageLayout,
    neighboring_villages: &mut HashMap<GnomeId, Vec<((u8, u8), TileType)>>,
    mgr: &mut Manager,
) {
    // TODO: we need to swap tiles for a new set
    if let Some(owner) = village.get_owner() {
        let existing_tile_types = village.get_tile_headers();
        neighboring_villages.insert(owner, existing_tile_types);
        village.set_owner(n_id, mgr);
        eprintln!(
            "NV keys: {:?}, searching for: {:?}",
            neighboring_villages.keys(),
            n_id
        );
        if let Some(new_tile_types) = neighboring_villages.remove(&n_id) {
            for (slot, tile_type) in new_tile_types.into_iter() {
                match tile_type {
                    TileType::Home(g_id) => {
                        if let Some(tile) = village.tiles.get_mut(&slot) {
                            tile.set_to_home(g_id, false, g_id == village.my_id, mgr);
                        }
                    }
                    TileType::Neighbor(n_id) => {
                        if let Some(tile) = village.tiles.get_mut(&slot) {
                            tile.set_to_neighbor(n_id, false, mgr);
                        }
                    }
                    TileType::Field => {
                        //TODO
                        if let Some(tile) = village.tiles.get_mut(&slot) {
                            tile.set_to_field(mgr);
                        }
                    }
                    TileType::Application => {
                        //TODO
                    }
                    TileType::Content(d_type, c_id) => {
                        if let Some(tile) = village.tiles.get_mut(&slot) {
                            tile.set_to_content(d_type, c_id, false, mgr);
                        }
                    }
                }
            }
        } else {
            eprintln!("Reseting tiles");
            village.reset_tiles(n_id, mgr);
        };
        let sel = village.selected_tile;
        village.select(&sel, mgr);
    } else {
        //TODO
        eprintln!("No owner defined for village");
    }
}
