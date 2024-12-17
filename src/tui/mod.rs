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
use std::fmt::format;
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
        // eprintln!("Next id: {:?}", tile_id);
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
    AppendContent(ContentID, DataType),
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
            " Create new...".to_string(),
            " Pięć".to_string(),
            " Sześć".to_string(),
            " Konstantynopol".to_string(),
        ],
    );
    let _set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " New Note".to_string(),
            " Jak".to_string(),
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

                    // TODO: move following code elsewhere
                    // let action = c_menu.show(&mut mgr, set_id, village.cm_position());
                    // eprintln!("Sel: {}", action);
                    let action = 0;
                    match action {
                        1 => {
                            // eprintln!("Requesting manifest");
                            // manifest_req = 1;
                            let _ = to_app.send(FromPresentation::ContentInquiry(0));
                        }
                        2 => {
                            // manifest_req = 2;
                            // eprintln!("Requesting manifest");
                            let _ = to_app.send(FromPresentation::ContentInquiry(0));
                        }
                        3 => {
                            // eprintln!("Adding new TAG");
                            let edit_result = editor.serve(
                                // input_display,
                                main_display,
                                " Max size: 32  Oneline  Define new Tag name    (TAB to finish)",
                                None,
                                // true,
                                false,
                                // None,
                                Some(32),
                                &mut mgr,
                            );
                            if let Some(text) = edit_result {
                                if !text.is_empty() {
                                    eprintln!("Got: '{}'", text);
                                    let tags = vec![Tag::new(text).unwrap()];
                                    let _ = to_app.send(FromPresentation::AddTags(tags));
                                }
                            }
                        }
                        4 => {
                            // eprintln!("Adding new DataType");
                            let edit_result = editor.serve(
                                // input_display,
                                main_display,
                                // &mut editor,
                                " Max size: 32  Oneline  Define new DataType   (TAB to finish)",
                                None,
                                // true,
                                false,
                                // None,
                                Some(32),
                                &mut mgr,
                            );
                            if let Some(text) = edit_result {
                                if !text.is_empty() {
                                    eprintln!("Got: '{}'", text);
                                    let _ = to_app.send(FromPresentation::AddDataType(
                                        Tag::new(text).unwrap(),
                                    ));
                                }
                            }
                        }
                        5 => {
                            // let read_only = my_id != manifest.;
                            // let read_only = !am_i_founder;
                            // let d_type = None;
                            // if let Some((d_type, data)) = creator.show(
                            //     &mut mgr,
                            //     &manifest,
                            //     &mut selector,
                            //     read_only,
                            //     d_type,
                            //     vec![],
                            //     String::new(),
                            //     main_display,
                            //     input_display,
                            //     &mut editor,
                            // ) {
                            //     //TODO
                            //     eprintln!("We have some work to do {:?} {}", d_type, data.len());
                            //     let _ = to_app.send(FromPresentation::CreateContent(d_type, data));
                            // } else {
                            //     eprintln!("Nothing to do from creator");
                            // };
                        }
                        other => {
                            eprintln!("A: {}", other);
                        }
                    }
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
                    //         let _ = to_app.send(FromPresentation::NeighborSelected(n_id));
                    let _ = to_app.send(FromPresentation::TileSelected(tile));
                    //TODO: we need to be aware of what context we are in to determine what
                    // match{
                    //     TileType::Home(owner) => {
                    //         if owner != village.my_id {
                    //             let _action = c_menu.show(&mut mgr, 0, village.cm_position());
                    //             continue;
                    //         }
                    //         // if let Some((d_type, data)) = creator.show(
                    //         //     &mut mgr,
                    //         //     &manifest,
                    //         //     &mut selector,
                    //         //     false,
                    //         //     None,
                    //         //     vec![],
                    //         //     String::new(),
                    //         //     main_display,
                    //         //     input_display,
                    //         //     &mut editor,
                    //         // ) {
                    //         //     //TODO
                    //         //     eprintln!("We have some work to do {:?} {}", d_type, data.len());
                    //         //     let _ = to_app.send(FromPresentation::CreateContent(d_type, data));
                    //         // } else {
                    //         //     eprintln!("Nothing to do from creator");
                    //         // };
                    //     }
                    //     TileType::Neighbor(n_id) => {
                    //         let _ = to_app.send(FromPresentation::NeighborSelected(n_id));
                    //         swap_tiles(n_id, &mut village, &mut neighboring_villages, &mut mgr);
                    //     }
                    //     TileType::Field => {
                    //         print!("What?");
                    //     }
                    //     TileType::Content(_d_type, c_id) => {
                    //         // if let Some((c_id, d_type)) = tiles_mapping.get(&village.selected_tile)
                    //         // {
                    //         // println!("Something: {:?}", c_data);
                    //         let _ = to_app.send(FromPresentation::ContentInquiry(c_id));
                    //         // }
                    //     }
                    //     _ => {
                    //         //TODO
                    //     }
                    // }
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
                ToPresentation::AppendContent(c_id, d_type) => {
                    eprintln!("ToPresentation::AppendContent({:?}, {:?})", c_id, d_type);
                    let tile_id = village.add_new_content(d_type, c_id, &mut mgr);
                    // tiles_mapping.insert(tile_id, TileType::Content(d_type, c_id));
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
