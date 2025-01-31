// use crate::logic::Manifest;
use animaterm::prelude::*;
// use animaterm::utilities::message_box;
pub use content_creator::CreatorResult;
use dapp_lib::prelude::AppError;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::Data;
use std::collections::HashMap;
// use std::fmt::format;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
mod ask;
mod button;
mod content_creator;
mod context_menu;
mod editor;
mod indexer;
mod option;
mod selector;
mod tile;
mod viewer;
use crate::config::Configuration;
use crate::logic::Tag;
use ask::Question;
use content_creator::Creator;
use context_menu::CMenu;
use editor::Editor;
use indexer::Indexer;
use selector::Selector;
use tile::Tile;
pub use tile::TileType;
// use viewer::Viewer;

pub struct VillageLayout {
    my_id: GnomeId,
    tiles_in_row: u8,
    visible_rows: u8,
    home_tile: (u8, u8),
    selected_tile: (u8, u8),
    tiles: HashMap<(u8, u8), Tile>,
    neighbors: HashMap<GnomeId, (u8, u8)>,
    street_to_rows: HashMap<Tag, Vec<u8>>,
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
            street_to_rows: HashMap::new(),
        }
    }

    pub fn initialize(
        &mut self,
        owner_id: GnomeId,
        mgr: &mut Manager,
        config: Configuration,
    ) -> u8 {
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
                let tile = Tile::new((off_x, off_y), mgr, &config.asset_dir);
                self.tiles.insert((row, col), tile);
            }
        }
        if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
            tile.set_to_home(owner_id, true, owner_id == self.my_id, mgr);
        }
        self.visible_rows >> 1
    }
    pub fn reset_tiles(&mut self, owner_id: GnomeId, mgr: &mut Manager) {
        let visible_streets = self.visible_rows >> 1;
        let mut str_names = Vec::with_capacity(visible_streets as usize);
        for _i in 0..visible_streets {
            str_names.push(Tag::empty());
        }
        self.set_street_names(str_names, mgr);
        for tile in self.tiles.values_mut() {
            tile.set_to_field(mgr);
        }
        if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
            tile.set_to_home(owner_id, false, owner_id == self.my_id, mgr);
        }
        self.selected_tile = self.home_tile;
    }

    fn set_street_names(&mut self, str_names: Vec<Tag>, mgr: &mut Manager) {
        let visible_streets = (self.visible_rows >> 1);
        let mut g = Glyph::plain();
        self.street_to_rows = HashMap::new();
        for i in 0..visible_streets {
            if let Some(str_name) = str_names.get(i as usize) {
                //TODO: print this name on screen
                let mut ci = 0;
                for c in str_name.0.chars() {
                    g.set_char(c);
                    mgr.set_glyph(0, g, 32 + ci, (14 * i as usize) + 7);
                    ci += 1;
                }
                g.set_char(' ');
                for cii in ci..32 {
                    mgr.set_glyph(0, g, 32 + cii, (14 * i as usize) + 7);
                }
                self.street_to_rows
                    .insert(str_name.clone(), vec![2 * i, 1 + (2 * i)]);
            } else {
                g.set_char(' ');
                for cii in 0..32 {
                    mgr.set_glyph(0, g, 32 + cii, (14 * i as usize) + 7);
                }
            }
        }
        // eprintln!("Presentation got street names:\n{:?}", str_names);
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
            let tile_id = self.next_neighbor_field_tile();
            self.neighbors.insert(n_id, tile_id);
            if let Some(tile) = self.tiles.get_mut(&tile_id) {
                tile.set_to_neighbor(n_id, false, mgr);
            }
            tile_id
        }
    }

    pub fn add_new_content(
        &mut self,
        d_type: DataType,
        c_id: ContentID,
        tags: Vec<Tag>,
        // street_to_rows: &HashMap<Tag, Vec<u8>>,
        mgr: &mut Manager,
        // ) -> (u8, u8) {
    ) {
        eprintln!("add_new_content tags: {:?}", tags);
        eprintln!("street to rows: {:?}", self.street_to_rows);
        for tag in tags {
            if let Some(restricted_rows) = self.street_to_rows.get(&tag) {
                eprintln!("rest rows: {:?}", restricted_rows);
                let tile_id = self.next_field_tile(restricted_rows);
                eprintln!("Next id: {:?}", tile_id);
                if let Some(tile) = self.tiles.get_mut(&tile_id) {
                    tile.set_to_content(d_type, c_id, false, mgr);
                }
            }
        }
        // tile_id
    }

    fn hide_content(&mut self, c_id: ContentID, tags: Vec<Tag>, mgr: &mut Manager) {
        //TODO: find tiles and change them to field
        eprintln!("We should hide CID-{} from: {:?}", c_id, tags);
        for tag in tags {
            eprintln!("hiding tag {:?}", tag);
            if let Some(rows) = self.street_to_rows.get(&tag) {
                eprintln!("searching among rows: {:?}", rows);
                'outer: for row in rows {
                    for column in 2..self.tiles_in_row {
                        if let Some(tile) = self.tiles.get_mut(&(*row, column)) {
                            eprintln!("Tile: {:?}", tile.tile_type);
                            if tile.tile_type.is_content(c_id) {
                                eprintln!("found it");
                                tile.set_to_field(mgr);
                                break 'outer;
                            }
                        }
                    }
                }
            } else {
                eprintln!("no rows to search in");
            }
        }
    }
    fn next_neighbor_field_tile(&self) -> (u8, u8) {
        for y in 0..self.visible_rows {
            if let Some(tile) = self.tiles.get(&(0, y)) {
                if matches!(tile.tile_type, TileType::Field) {
                    // return (0, y);
                    return (y, 0);
                }
            } else if let Some(tile) = self.tiles.get(&(1, y)) {
                if matches!(tile.tile_type, TileType::Field) {
                    // return (1, y);
                    return (y, 1);
                }
            }
        }
        (255, 255)
    }

    fn next_field_tile(&self, restricted_rows: &Vec<u8>) -> (u8, u8) {
        // if is_neighbor {
        //     for y in 0..self.visible_rows {
        //         if let Some(tile) = self.tiles.get(&(0, y)) {
        //             if matches!(tile.tile_type, TileType::Field) {
        //                 // return (0, y);
        //                 return (y, 0);
        //             }
        //         } else if let Some(tile) = self.tiles.get(&(1, y)) {
        //             if matches!(tile.tile_type, TileType::Field) {
        //                 // return (1, y);
        //                 return (y, 1);
        //             }
        //         }
        //     }
        // }
        // for y in 0..self.visible_rows {
        for y in restricted_rows {
            for x in 2..self.tiles_in_row {
                if let Some(tile) = self.tiles.get(&(x, *y)) {
                    if matches!(tile.tile_type, TileType::Field) {
                        return (*y, x);
                    }
                }
            }
        }
        return (255, 255);
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
        // eprintln!("sel: {:?}, (x,y): {:?}", self.selected_tile, (x, y));
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
    Neighbors(Vec<GnomeId>),
    AppendContent(ContentID, DataType, Vec<Tag>),
    HideContent(ContentID, Vec<Tag>),
    Contents(ContentID, DataType, Vec<u8>, String, Vec<Data>),
    ReadError(ContentID, AppError),
    DisplaySelector(bool, String, Vec<String>, Vec<usize>), //bool indicates if we are founder of active swarm
    DisplayCMenu(usize),
    DisplayEditor(
        (bool, bool), // (read_only, can_edit)
        String,
        Option<String>,
        bool,        // allow_newlines
        Option<u16>, // byte_limit
    ),
    DisplayCreator(
        bool,   // read_only
        String, // DataType,
        String, //Description
        String, //Tags
    ),
    DisplayIndexer(Vec<String>),
    SwapTiles(GnomeId),
    StreetNames(Vec<Tag>),
}

#[derive(Clone)]
pub enum FromPresentation {
    //TODO: we don't want to send Keys out of TUI, rather instructions depending on
    // context where given key was pressed
    AddDataType(Tag),
    AddTags(Vec<Tag>),
    CreateContent(DataType, Data),
    UpdateContent(ContentID, DataType, u16, Vec<Data>),
    ContentInquiry(ContentID),
    NeighborSelected(GnomeId),
    KeyPress(Key),
    ShowContextMenu(TileType),
    TileSelected(TileType),
    CMenuAction(usize),
    SelectedIndices(Vec<usize>),
    EditResult(Option<String>),
    IndexResult(Option<usize>),
    CreatorResult(CreatorResult),
    VisibleStreetsCount(u8),
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
    let res = mgr.add_graphic(bg, 1, (0, 0));
    eprintln!("TUI background index: {:?}", res);
    mgr
}

/// This function is for sending requests to application and displaying user interface.
pub fn serve_tui_mgr(
    my_id: GnomeId,
    mut mgr: Manager,
    to_app: Sender<FromPresentation>,
    to_tui_recv: Receiver<ToPresentation>,
    config: Configuration,
) {
    let s_size = mgr.screen_size();
    let mut village = VillageLayout::new(s_size);
    let mut neighboring_villages = HashMap::new();
    let visible_streets = village.initialize(my_id, &mut mgr, config);
    // let mut street_to_rows = HashMap::new();
    let _ = to_app.send(FromPresentation::VisibleStreetsCount(visible_streets));

    // let mut tiles_mapping = HashMap::<(u8, u8), TileType>::new();
    // eprintln!("Serving TUI Manager scr size: {}x{}", s_size.0, s_size.1);
    let main_display = 0;
    let mut editor = Editor::new(&mut mgr);
    let mut indexer = Indexer::new(&mut mgr);
    let mut creator = Creator::new(&mut mgr);
    let mut selector = Selector::new(AppType::Catalog, &mut mgr);
    mgr.restore_display(main_display, true);
    let mut c_menu = CMenu::new(&mut mgr);
    let mut d_type_map = HashMap::new();
    d_type_map.insert(DataType::Data(0), "Text".to_string());
    d_type_map.insert(DataType::Data(1), "Text file".to_string());
    d_type_map.insert(DataType::Data(2), "Binary file".to_string());
    let question = Question::new(&mut mgr);
    // let mut am_i_founder = false;
    // let mut manifest = Manifest::new(AppType::Catalog, HashMap::new());
    let _set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " TAGs".to_string(),
            " Data types".to_string(),
            " Add TAG".to_string(),
            " Add data type".to_string(),
            " Create new…".to_string(),
            " Public IPs".to_string(),
            " Active Swarms".to_string(),
            " Known Swarms".to_string(),
        ],
    );
    let _set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " Nowa Notatka".to_string(),
            " Usuń Notatkę".to_string(),
            " By".to_string(),
            " Nie".to_string(),
            " Było".to_string(),
            " Będzie".to_string(),
            " Bardzo".to_string(),
            " Miło".to_string(),
        ],
    );
    eprintln!("Added CMenu set: {}", _set_id);
    // let mut manifest_req: u8 = 0;
    loop {
        if let Some(key) = mgr.read_key() {
            let terminate = key == Key::Q || key == Key::ShiftQ;
            match key {
                Key::AltEnter | Key::Space => {
                    // TODO: minimize logic in tui - simply send a Selected message to logic
                    //       and wait for instructions
                    let _ = to_app.send(FromPresentation::ShowContextMenu(village.get_selection()));
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
                    let tile = village.get_selection();
                    if let TileType::Neighbor(g_id) = &tile {
                        swap_tiles(*g_id, &mut village, &mut neighboring_villages, &mut mgr);
                    }
                    let _ = to_app.send(FromPresentation::TileSelected(tile));
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
                ToPresentation::StreetNames(str_names) => {
                    village.set_street_names(str_names, &mut mgr);
                }
                ToPresentation::DisplayCMenu(set_id) => {
                    let action = c_menu.show(&mut mgr, set_id, village.cm_position());
                    let _ = to_app.send(FromPresentation::CMenuAction(action));
                }
                ToPresentation::Neighbors(neighbors) => {
                    //TODO: first make sure neighbor is not placed on screen
                    for neighbor in neighbors.into_iter() {
                        let _tile_id = village.add_new_neighbor(neighbor, &mut mgr);
                    }
                }
                ToPresentation::AppendContent(c_id, d_type, tags) => {
                    eprintln!(
                        "ToPresentation::AppendContent({:?}, {:?})\nTags: {:?}",
                        c_id, d_type, tags
                    );
                    village.add_new_content(d_type, c_id, tags, &mut mgr);
                    // tiles_mapping.insert(tile_id, TileType::Content(d_type, c_id));
                }
                ToPresentation::HideContent(c_id, tags) => {
                    village.hide_content(c_id, tags, &mut mgr);
                }
                ToPresentation::Contents(c_id, d_type, tags, text, mut data_vec) => {
                    eprintln!("Showing Contents of {}", c_id,);
                    // let read_only = !am_i_founder;

                    // if let Some((d_type, data)) = creator.show(
                    //     &mut mgr,
                    //     &manifest,
                    //     &mut selector,
                    //     read_only,
                    //     Some(d_type),
                    //     tags,
                    //     text,
                    //     main_display,
                    //     input_display,
                    //     &mut editor,
                    // ) {
                    //     //TODO
                    //     eprintln!("We have some work to do {:?} {}", d_type, data.len());
                    //     let mut d_vec = vec![data];
                    //     d_vec.append(&mut data_vec);
                    //     let _ =
                    //         to_app.send(FromPresentation::UpdateContent(c_id, d_type, 0, d_vec));
                    // } else {
                    //     eprintln!("Nothing to do from creator");
                    // };
                }
                ToPresentation::DisplaySelector(
                    quit_on_first_select,
                    header,
                    tag_names,
                    selections,
                ) => {
                    // eprintln!("TUI got Manifest {}", mani.tags.len());
                    // am_i_founder = me_founder;
                    // manifest = mani.clone();
                    // if manifest_req == 1 {
                    // let tag_names = mani.tag_names();
                    eprintln!("All tag names: {:?}", tag_names);
                    let _selected = selector.select(
                        &header,
                        &tag_names,
                        selections,
                        &mut mgr,
                        quit_on_first_select,
                    );
                    mgr.restore_display(main_display, true);
                    // eprintln!("Selected tags: ");
                    // for index in _selected {
                    //     if let Some(value) = tag_names.get(index) {
                    //         eprintln!("{} - {}", index, value);
                    //     }
                    // }
                    let _ = to_app.send(FromPresentation::SelectedIndices(_selected));
                    // } else if manifest_req == 2 {
                    //     let tag_names = mani.dtype_names();
                    //     eprintln!("All dtype names: {:?}", tag_names);
                    //     let _selected = selector.select(
                    //         "Catalog Application's Data types",
                    //         &tag_names,
                    //         vec![],
                    //         &mut mgr,
                    //         true,
                    //     );
                    //     eprintln!("Selected data types: ");
                    //     for index in _selected {
                    //         eprintln!("{} - {}", index, tag_names.get(index).unwrap());
                    //     }
                    // }
                    // manifest_req = 0;
                }
                ToPresentation::DisplayEditor(
                    read_only,
                    header,
                    initial_text,
                    allow_new_lines,
                    limit,
                ) => {
                    editor.set_mode(read_only);
                    let edit_result = editor.serve(
                        // input_display,
                        main_display,
                        // &mut editor,
                        &header,
                        initial_text,
                        allow_new_lines,
                        limit,
                        &mut mgr,
                    );
                    let _ = to_app.send(FromPresentation::EditResult(edit_result));
                }
                ToPresentation::DisplayIndexer(headers) => {
                    let index_result =
                        indexer.serve(main_display, "This is an Indexer", headers, &mut mgr);
                    let _ = to_app.send(FromPresentation::IndexResult(index_result));
                }
                ToPresentation::DisplayCreator(read_only, d_type, description, tags) => {
                    //TODO
                    let c_result = creator.show(&mut mgr, read_only, d_type, tags, description);
                    let _ = to_app.send(FromPresentation::CreatorResult(c_result));
                }
                ToPresentation::SwapTiles(g_id) => {
                    swap_tiles(g_id, &mut village, &mut neighboring_villages, &mut mgr);
                }
                ToPresentation::ReadError(c_id, error) => {
                    if question.ask(
                        &format!("Error reading CID {}:\n {}", c_id, error),
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
