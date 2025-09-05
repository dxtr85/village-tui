use animaterm::prelude::*;
use async_std::channel::Sender as ASender;
use async_std::task::sleep;
// use async_std::channel;
// use animaterm::utilities::message_box;
pub use content_creator::Creator;
pub use content_creator::CreatorResult;
use dapp_lib::prelude::AppError;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::SwarmID;
use dapp_lib::prelude::SwarmName;
use dapp_lib::Data;
pub use notifier::Notifier;
use std::collections::HashMap;
use std::collections::HashSet;
// use std::fmt::format;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
mod ask;
pub mod button;
mod content_creator;
mod context_menu;
mod editor;
mod indexer;
mod notifier;
mod option;
mod selector;
mod tile;
mod viewer;
use crate::catalog::logic::Tag;
use crate::config::Configuration;
use crate::InternalMsg;
use crate::Toolset;
use ask::Question;
use context_menu::CMenu;
pub use editor::Editor;
pub use indexer::Indexer;
pub use selector::Selector;
use tile::Tile;
pub use tile::TileType;
// use viewer::Viewer;

pub struct VillageLayout {
    my_name: SwarmName,
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
            my_name: SwarmName {
                founder: GnomeId::any(),
                name: format!("/"),
            },
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
        self.my_name.founder = owner_id;
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
            tile.set_to_home(
                owner_id,
                true,
                owner_id == self.my_name.founder,
                // Some(format!("Testowy pirszy")),
                mgr,
            );
        }
        self.visible_rows >> 1
    }
    pub fn reset_tiles(&mut self, owner_id: GnomeId, keep_neighbors: bool, mgr: &mut Manager) {
        let visible_streets = self.visible_rows >> 1;
        let mut str_names = Vec::with_capacity(visible_streets as usize);
        for _i in 0..visible_streets {
            str_names.push((Tag::empty(), HashSet::new()));
        }
        self.set_street_names(str_names, mgr);
        if keep_neighbors {
            for c in 2..self.tiles_in_row {
                for r in 0..self.visible_rows {
                    if let Some(tile) = self.tiles.get_mut(&(r, c)) {
                        if tile.tile_type.is_field() {
                            continue;
                        }
                        tile.set_to_field(mgr);
                    }
                }
            }
        } else {
            self.neighbors = HashMap::new();
            for tile in self.tiles.values_mut() {
                tile.set_to_field(mgr);
            }
            if let Some(tile) = self.tiles.get_mut(&self.home_tile) {
                tile.set_to_home(
                    owner_id,
                    false,
                    owner_id == self.my_name.founder,
                    // Some(format!("{}", owner_id)),
                    mgr,
                );
            }
            self.selected_tile = self.home_tile;
        }
    }

    fn clear_street_names(&mut self, street_count: usize, mgr: &mut Manager) {
        let g = Glyph::plain();
        for s_idx in 0..street_count {
            for c in 0..32 {
                mgr.set_glyph(0, g, 32 + c, (14 * s_idx as usize) + 7);
            }
        }
    }
    fn set_street_names(
        &mut self,
        str_names: Vec<(Tag, HashSet<(DataType, ContentID, String)>)>,
        mgr: &mut Manager,
    ) {
        eprintln!("set_street_names, resetting street_to_rows");
        // let visible_streets = self.visible_rows >> 1;
        let mut g = Glyph::plain();
        self.street_to_rows = HashMap::new();
        // for i in 0..visible_streets {
        let mut i = 0;
        for (str_name, contents) in str_names {
            // if let Some(str_name) = str_names.get(i as usize) {
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
            let restricted_rows = if 1 + (2 * i) < self.visible_rows {
                vec![2 * i, 1 + (2 * i)]
            } else {
                vec![2 * i]
            };
            self.street_to_rows
                .insert(str_name.clone(), restricted_rows.clone());
            // for (d_type, c_id) in contents{
            //TODO: display contents
            // let pos = self.next_field_tile(&restricted_rows);
            self.add_contents_to_rows(contents, restricted_rows, mgr);
            // }
            // } else {
            //     g.set_char(' ');
            //     for cii in 0..32 {
            //         mgr.set_glyph(0, g, 32 + cii, (14 * i as usize) + 7);
            //     }
            // }
            i += 1;
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
            // if let TileType::Home(curr_owner) = tile.tile_type {
            // if curr_owner.is_any() {
            tile.set_to_home(
                owner_id,
                false,
                owner_id == self.my_name.founder,
                // Some(format!("Testowy trzeci")),
                mgr,
            );
            // } else {
            //     eprintln!(
            //         "Attempt to change owner from: {} to {}",
            //         curr_owner, owner_id
            //     );
            // }
            // }
        } else {
            eprintln!("Unable to find home tile in order to set owner");
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
        self.tiles
            .get(&self.selected_tile)
            .unwrap()
            .tile_type
            .clone()
    }
    pub fn add_new_neighbor(&mut self, n_id: GnomeId, mgr: &mut Manager) -> (u8, u8) {
        if let Some(tile_location) = self.neighbors.get(&n_id) {
            eprintln!("Have this neighbor already under {:?}", tile_location);
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
    pub fn remove_neighbor(&mut self, n_id: GnomeId, mgr: &mut Manager) -> (u8, u8) {
        if let Some(tile_location) = self.neighbors.get(&n_id) {
            if let Some(tile) = self.tiles.get_mut(tile_location) {
                tile.set_to_field(mgr);
            }
            return *tile_location;
        }
        return (255, 255);
    }

    pub fn add_new_content(
        &mut self,
        d_type: DataType,
        c_id: ContentID,
        tags: HashSet<Tag>,
        description: String,
        mgr: &mut Manager,
        // ) -> (u8, u8) {
    ) {
        eprintln!("add_new_content CID-{} tags: {:?}", c_id, tags);
        eprintln!("street to rows: {:?}", self.street_to_rows);
        for tag in tags {
            eprintln!("Tag: {:?}", tag);
            if let Some(restricted_rows) = self.street_to_rows.get(&tag) {
                eprintln!("rest rows: {:?}", restricted_rows);
                let tile_id = self.next_field_tile(restricted_rows);
                eprintln!("Next id: {:?}", tile_id);
                if let Some(tile) = self.tiles.get_mut(&tile_id) {
                    tile.set_to_content(Some(description.clone()), d_type, c_id, false, mgr);
                } else {
                    eprintln!("No tile");
                }
            } else {
                eprintln!("No restricted rows");
            }
        }
        // tile_id
    }
    pub fn add_contents_to_rows(
        &mut self,
        contents: HashSet<(DataType, ContentID, String)>,
        restricted_rows: Vec<u8>,
        // d_type: DataType,
        // c_id: ContentID,
        // tags: Vec<Tag>,
        // street_to_rows: &HashMap<Tag, Vec<u8>>,
        mgr: &mut Manager,
        // ) -> (u8, u8) {
    ) {
        // eprintln!("add_new_content CID-{} tags: {:?}", c_id, tags);
        // eprintln!("street to rows: {:?}", self.street_to_rows);
        // for tag in tags {
        // eprintln!("Tag: {:?}", tag);
        // if let Some(restricted_rows) = self.street_to_rows.get(&tag) {
        eprintln!("rest rows: {:?}", restricted_rows);
        for (d_type, c_id, header) in contents {
            let tile_id = self.next_field_tile(&restricted_rows);
            eprintln!("Next id: {:?}", tile_id);
            if let Some(tile) = self.tiles.get_mut(&tile_id) {
                tile.set_to_content(Some(header), d_type, c_id, false, mgr);
            } else {
                eprintln!("No tile");
            }
        }
        // } else {
        //     eprintln!("No restricted rows");
        // }
        // }
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
    fn filter_visible_tags(&self, tags: Vec<Tag>) -> HashSet<Tag> {
        self.street_to_rows
            .keys()
            .cloned()
            .filter(|t| tags.contains(t))
            .collect()
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
        for x in 2..self.tiles_in_row {
            for y in restricted_rows {
                if let Some(tile) = self.tiles.get(&(*y, x)) {
                    if matches!(tile.tile_type, TileType::Field) {
                        return (*y, x);
                    }
                }
            }
        }
        return (255, 255);
    }
    pub fn select_next(&mut self, direction: Direction, mgr: &mut Manager) -> bool {
        let mut out_of_screen = false;
        let new_index = match direction {
            Direction::Left => {
                let new_col = if self.selected_tile.1 == 0 {
                    out_of_screen = true;
                    self.tiles_in_row - 1
                } else {
                    self.selected_tile.1 - 1
                };
                (self.selected_tile.0, new_col)
            }
            Direction::Right => {
                let new_col = if self.selected_tile.1 + 1 == self.tiles_in_row {
                    out_of_screen = true;
                    0
                } else {
                    self.selected_tile.1 + 1
                };
                (self.selected_tile.0, new_col)
            }
            Direction::Up => {
                let new_row = if self.selected_tile.0 == 0 {
                    out_of_screen = true;
                    self.visible_rows - 1
                } else {
                    self.selected_tile.0 - 1
                };
                (new_row, self.selected_tile.1)
            }
            Direction::Down => {
                let new_row = if self.selected_tile.0 + 1 == self.visible_rows {
                    out_of_screen = true;
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
        out_of_screen
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
#[derive(Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub enum ToCatalogView {
    Neighbors(Vec<GnomeId>),
    NeighborLeft(GnomeId),
    AppendContent(ContentID, DataType, Vec<Tag>, String),
    HideContent(ContentID, Vec<Tag>),
    ContentHeader(ContentID, Data),
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
    StreetNames(Vec<(Tag, HashSet<(DataType, ContentID, String)>)>),
    SetNotification(usize, Vec<Glyph>),
    MoveNotification(usize, (isize, isize)),
    // SwitchApp(AppType, MessagePipes),
}

#[derive(Clone)]
pub enum FromCatalogView {
    //TODO: we don't want to send Keys out of TUI, rather instructions depending on
    // context where given key was pressed
    AddDataType(Tag),
    AddTags(Vec<Tag>),
    ChangeTag(u8, Tag),
    CreateContent(DataType, Data),
    UpdateContent(ContentID, DataType, u16, Vec<Data>),
    ContentInquiry(ContentID),
    NeighborSelected(SwarmName),
    KeyPress(Key),
    SwitchToApp(AppType, SwarmID, SwarmName),
    ShowContextMenu(TileType),
    TileSelected(TileType),
    CMenuAction(usize),
    SelectedIndices(Vec<usize>),
    EditResult(Option<String>),
    IndexResult(Option<usize>),
    CreatorResult(CreatorResult),
    VisibleStreetsCount(u8),
    CursorOutOfScreen(Direction),
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
    // let (cols, rows) = mgr.screen_size();
    // let frame = vec![Glyph::plain(); cols * rows];
    // let mut library = HashMap::new();
    // library.insert(0, frame);
    // let bg = Graphic::new(cols, rows, 0, library, None);
    // let res = mgr.add_graphic(bg, 1, (0, 0));
    // eprintln!("TUI background index: {:?}", res);
    mgr
}

/// This function is for sending requests to application and displaying user interface.
pub fn serve_catalog_tui(
    main_display: usize,
    my_id: GnomeId,
    mut mgr: Manager,
    // message_pipes: MessagePipes,
    to_app: Sender<FromCatalogView>,
    // to_tui_send: Sender<ToPresentation>,
    to_tui_recv: Receiver<ToCatalogView>,
    config: Configuration,
    mut editor: Editor,
    mut creator: Creator,
    mut selector: Selector,
    mut indexer: Indexer,
) -> Toolset {
    // let to_app = message_pipes.sender();
    // let to_tui_recv = message_pipes.reveiver();
    let s_size = mgr.screen_size();
    let mut village = VillageLayout::new(s_size);
    let mut neighboring_villages = HashMap::new();
    let visible_streets = village.initialize(my_id, &mut mgr, config.clone());
    // let mut street_to_rows = HashMap::new();
    let _ = to_app.send(FromCatalogView::VisibleStreetsCount(visible_streets));

    // let mut tiles_mapping = HashMap::<(u8, u8), TileType>::new();
    // eprintln!("Serving TUI Manager scr size: {}x{}", s_size.0, s_size.1);
    // let main_display = 0;
    // let mut indexer = Indexer::new(&mut mgr);
    // let mut creator = Creator::new(&mut mgr);
    // let mut selector = Selector::new(AppType::Catalog, &mut mgr);
    // let mut editor = Editor::new(&mut mgr);
    // mgr.restore_display(main_display, true);
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
            " Add Search".to_string(),
        ],
    );
    let _set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " Nowa Notatka".to_string(),
            " Usuń Notatkę".to_string(),
            " Kopiuj Odnośnik".to_string(),
            " Nie".to_string(),
            " Było".to_string(),
            " Będzie".to_string(),
            " Bardzo".to_string(),
            " Miło".to_string(),
        ],
    );
    eprintln!("Added CMenu set: {}", _set_id);
    let _set_id = c_menu.add_set(
        &mut mgr,
        vec![
            " Wklej Odnośnik".to_string(),
            " Na".to_string(),
            " Razie".to_string(),
            " Nic".to_string(),
            " Ciekawego".to_string(),
            " Tu".to_string(),
            " Nie".to_string(),
            " List Searches".to_string(),
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
                    let _ = to_app.send(FromCatalogView::ShowContextMenu(village.get_selection()));
                }
                Key::Left | Key::H | Key::CtrlB => {
                    if village.select_next(Direction::Left, &mut mgr) {
                        //TODO: we went out of screen
                    }
                }
                Key::Right | Key::L | Key::CtrlF => {
                    if village.select_next(Direction::Right, &mut mgr) {
                        //TODO
                    }
                }
                Key::Up | Key::K | Key::CtrlP => {
                    if village.select_next(Direction::Up, &mut mgr) {
                        let _ = to_app.send(FromCatalogView::CursorOutOfScreen(Direction::Up));
                    }
                }
                Key::Down | Key::J | Key::CtrlN => {
                    if village.select_next(Direction::Down, &mut mgr) {
                        let _ = to_app.send(FromCatalogView::CursorOutOfScreen(Direction::Down));
                    }
                }
                Key::AltCtrlH => {
                    let _ = to_app.send(FromCatalogView::NeighborSelected(village.my_name.clone()));
                    // TODO: remove swap_tiles logic from presentation, it should not be here
                    eprintln!("AltCtrlH");
                    swap_tiles(
                        village.my_name.founder,
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
                    // if let TileType::Neighbor(n_id) = &tile {
                    //     // eprintln!("Enter");
                    //     swap_tiles(*n_id, &mut village, &mut neighboring_villages, &mut mgr);
                    // }
                    let _ = to_app.send(FromCatalogView::TileSelected(tile));
                }
                Key::F => {
                    // TODO: make sure we are attached to target swarm
                    eprintln!("F: SwitchToApp");
                    let _ = to_app.send(FromCatalogView::SwitchToApp(
                        AppType::Forum,
                        SwarmID(2),
                        SwarmName {
                            founder: my_id,
                            name: "F".to_string(),
                        },
                    ));
                    break;
                }
                other => {
                    // eprintln!("Send to app: {} terminate: {}", other, terminate);
                    let res = to_app.send(FromCatalogView::KeyPress(other));
                    if res.is_err() || terminate {
                        break;
                    }
                }
            }
        }
        if let Ok(to_tui) = to_tui_recv.try_recv() {
            match to_tui {
                ToCatalogView::StreetNames(str_names) => {
                    village.reset_tiles(village.my_name.founder, true, &mut mgr);
                    village.set_street_names(str_names, &mut mgr);
                }
                ToCatalogView::DisplayCMenu(set_id) => {
                    let action = c_menu.show(&mut mgr, set_id, village.cm_position());
                    let _ = to_app.send(FromCatalogView::CMenuAction(action));
                }
                ToCatalogView::Neighbors(neighbors) => {
                    //TODO: first make sure neighbor is not placed on screen
                    for neighbor in neighbors.into_iter() {
                        let _tile_id = village.add_new_neighbor(neighbor, &mut mgr);
                        eprintln!("Showing Neighbor {}: {:?}", neighbor, _tile_id);
                    }
                }
                ToCatalogView::NeighborLeft(n_id) => {
                    let _tile_id = village.remove_neighbor(n_id, &mut mgr);
                }
                ToCatalogView::AppendContent(c_id, d_type, tags, description) => {
                    eprintln!(
                        "ToPresentation::AppendContent({:?}, {:?})\nTags: {:?}",
                        c_id, d_type, tags
                    );
                    //TODO: First we check if this content should be displayed on screen
                    let mut filtered_tags = village.filter_visible_tags(tags);
                    // if so, we get all it's current instances as Tile
                    let mut t_headers = village.get_tile_headers();
                    let tile_locations = t_headers
                        .iter()
                        .filter(|(_loc, ttype)| matches!(ttype, TileType::Content(d_type, c_id)));
                    eprintln!("tlocs: {:?}", tile_locations);
                    for (loc, _t) in tile_locations {
                        eprintln!("tlocs: {:?}", _t);
                        for (tag, rows) in &village.street_to_rows {
                            eprintln!("tag: {:?}, rows: {:?}", tag, rows);
                            if rows.contains(&loc.0) {
                                // and subscract Tags that correspond to them
                                filtered_tags.remove(&tag);
                                eprintln!("Rem: {:?}", tag);
                            }
                        }
                    }
                    // if there are any tags left we add Tiles for them.
                    village.add_new_content(d_type, c_id, filtered_tags, description, &mut mgr);
                    // tiles_mapping.insert(tile_id, TileType::Content(d_type, c_id));
                }
                ToCatalogView::HideContent(c_id, tags) => {
                    village.hide_content(c_id, tags, &mut mgr);
                }
                ToCatalogView::ContentHeader(c_id, data) => {
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
                ToCatalogView::DisplaySelector(
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
                    let _ = to_app.send(FromCatalogView::SelectedIndices(_selected));
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
                ToCatalogView::DisplayEditor(
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
                    let _ = to_app.send(FromCatalogView::EditResult(edit_result));
                }
                ToCatalogView::DisplayIndexer(headers) => {
                    let index_result =
                        indexer.serve(main_display, "This is an Indexer", headers, &mut mgr);
                    let _ = to_app.send(FromCatalogView::IndexResult(index_result));
                }
                ToCatalogView::DisplayCreator(read_only, d_type, description, tags) => {
                    //TODO
                    let c_result =
                        creator.show(main_display, &mut mgr, read_only, d_type, tags, description);
                    let _ = to_app.send(FromCatalogView::CreatorResult(c_result));
                }
                ToCatalogView::SwapTiles(g_id) => {
                    eprintln!("ToPresentation::SwapTiles");
                    swap_tiles(g_id, &mut village, &mut neighboring_villages, &mut mgr);
                }
                ToCatalogView::ReadError(c_id, error) => {
                    if question.ask(
                        &format!("Error reading CID {}:\n {}", c_id, error),
                        &mut mgr,
                    ) {
                        // let manifest = create_manifest(&mut mgr);
                        // TODO: send Data to Swarm
                    }
                }
                ToCatalogView::SetNotification(g_id, new_frame) => {
                    // eprintln!("Got SetNotification");
                    let _old_frame = mgr.swap_frame(g_id, 0, new_frame);
                    // eprintln!("Swap frame res: {:?}", _old_frame);
                }
                ToCatalogView::MoveNotification(g_id, offset) => {
                    mgr.move_graphic(g_id, 4, offset);
                } // ToPresentation::SwitchApp(app_type, pipes) => {
                  //     //TODO
                  //     eprintln!("Should switch to {app_type:?}");
                  // }
            }
        }
    }
    // editor.cleanup(main_display, &mut mgr);
    // creator.cleanup(main_display, &mut mgr);
    // selector.cleanup(main_display, &mut mgr);
    // indexer.cleanup(main_display, &mut mgr);
    eprintln!("Done serving TUI");
    // mgr.terminate();
    // (mgr, config)
    Toolset::fold(
        mgr,
        config,
        Some(editor),
        Some(creator),
        Some(selector),
        Some(indexer),
        None,
    )
}

fn swap_tiles(
    n_id: GnomeId,
    village: &mut VillageLayout,
    neighboring_villages: &mut HashMap<GnomeId, Vec<((u8, u8), TileType)>>,
    mgr: &mut Manager,
) {
    if n_id.is_any() {
        eprintln!("We should clear all tiles");
        village.neighbors = HashMap::new();
        village.clear_street_names(2, mgr);
        for tile in village.tiles.values_mut() {
            if tile.tile_type.is_field() || tile.tile_type.is_home() {
                continue;
            }
            tile.set_to_field(mgr);
        }
        return;
    }
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
                            tile.set_to_home(
                                g_id,
                                false,
                                g_id == village.my_name.founder,
                                // Some(format!("Testowy czwarty")),
                                mgr,
                            );
                        }
                    }
                    TileType::Neighbor(n_id) => {
                        if let Some(tile) = village.tiles.get_mut(&slot) {
                            // We should get updated Neighbor list from internal mechanism
                            // tile.set_to_field(mgr);
                            village.neighbors.insert(n_id, slot);
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
                        // TODO: need to define a better algorithm
                        //
                        // if let Some(tile) = village.tiles.get_mut(&slot) {
                        //     tile.set_to_content(None, d_type, c_id, false, mgr);
                        // }
                    }
                }
            }
        } else {
            eprintln!("Reseting tiles");
            village.reset_tiles(n_id, false, mgr);
        };
        let sel = village.selected_tile;
        village.select(&sel, mgr);
    } else {
        //TODO
        eprintln!("No owner defined for village");
    }
}

pub async fn from_catalog_tui_adapter(
    from_presentation: Receiver<FromCatalogView>,
    wrapped_sender: ASender<InternalMsg>,
) {
    let timeout = Duration::from_millis(16);
    loop {
        let recv_res = from_presentation.recv_timeout(timeout);
        match recv_res {
            Ok(from_tui) => {
                let _ = wrapped_sender.send(InternalMsg::Catalog(from_tui)).await;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => sleep(timeout).await,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
    eprintln!("from_tui_adapter is done");
}
