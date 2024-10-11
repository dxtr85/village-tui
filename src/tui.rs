use crate::input::Input;
use animaterm::prelude::*;
use animaterm::utilities::message_box;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::SwarmName;
use std::collections::HashMap;
use std::fmt::format;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

#[derive(Copy, Clone)]
pub enum TileType {
    Home,
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
    pub fn set_to_home(&mut self, selected: bool, mgr: &mut Manager) {
        self.select_frame = 3;
        self.deselect_frame = 2;
        self.tile_type = TileType::Home;
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
        if let Some(tile) = self.tiles.get_mut(&self.selected_tile) {
            tile.set_to_home(true, mgr);
        }
    }

    pub fn select(&mut self, index: &(u8, u8), mgr: &mut Manager) {
        if let Some(tile) = self.tiles.get(index) {
            if let Some(old_tile) = self.tiles.get(&self.selected_tile) {
                mgr.set_graphic(old_tile.id, old_tile.deselect_frame, false);
            }
            mgr.set_graphic(tile.id, tile.select_frame, false);
            self.selected_tile = *index;
        }
    }
    pub fn get_selection(&self) -> TileType {
        self.tiles.get(&self.selected_tile).unwrap().tile_type
    }
    pub fn add_new_neighbor(&mut self, n_id: GnomeId, mgr: &mut Manager) -> (u8, u8) {
        let tile_id = self.next_field_tile();
        let tile = self.tiles.get_mut(&tile_id).unwrap();
        tile.set_to_neighbor(n_id, false, mgr);
        tile_id
    }

    pub fn add_new_content(
        &mut self,
        d_type: DataType,
        c_id: ContentID,
        mgr: &mut Manager,
    ) -> (u8, u8) {
        let tile_id = self.next_field_tile();
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
}
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

pub enum ToTui {
    Neighbors(SwarmName, Vec<GnomeId>),
    // MoveSelection(Direction),
    AddContent(ContentID, DataType),
    Contents(ContentID, String),
}

pub enum FromTui {
    //TODO: we don't want to send Keys out of TUI, rather instructions depending on
    // context where given key was pressed
    NewUserEntry(String),
    ContentInquiry(ContentID),
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
pub fn serve_tui_mgr(mut mgr: Manager, to_app: Sender<FromTui>, to_tui_recv: Receiver<ToTui>) {
    let s_size = mgr.screen_size();
    let mut village = VillageLayout::new(s_size);
    village.initialize(&mut mgr);

    // let mut tiles_mapping = HashMap::<(u8, u8), TileType>::new();
    // eprintln!("Serving TUI Manager scr size: {}x{}", s_size.0, s_size.1);
    let main_display = 0;
    let input_display = mgr.new_display(true);
    let mut input = Input::new(&mut mgr);
    input.show(&mut mgr);
    mgr.restore_display(main_display, true);
    loop {
        if let Some(key) = mgr.read_key() {
            let terminate = key == Key::Q || key == Key::ShiftQ;
            match key {
                Key::Left | Key::H | Key::CtrlB => village.select_next(Direction::Left, &mut mgr),
                Key::Right | Key::L | Key::CtrlF => village.select_next(Direction::Right, &mut mgr),
                Key::Up | Key::K | Key::CtrlP => village.select_next(Direction::Up, &mut mgr),
                Key::Down | Key::J | Key::CtrlN => village.select_next(Direction::Down, &mut mgr),
                Key::Enter => {
                    //TODO: we need to be aware of what context we are in to determine what
                    match village.get_selection() {
                        TileType::Home => {
                            let typed_text =
                                serve_input(input_display, main_display, &mut input, &mut mgr);
                            eprintln!("Got: {}", typed_text);
                            to_app.send(FromTui::NewUserEntry(typed_text));
                        }
                        TileType::Field => {
                            println!("What?");
                        }
                        TileType::Content(_d_type, c_id) => {
                            // if let Some((c_id, d_type)) = tiles_mapping.get(&village.selected_tile)
                            // {
                            // println!("Something: {:?}", c_data);
                            to_app.send(FromTui::ContentInquiry(c_id));
                            // }
                        }
                        _ => {
                            //TODO
                        }
                    }
                }
                other => {
                    eprintln!("Send to app: {} terminate: {}", other, terminate);
                    let res = to_app.send(FromTui::KeyPress(other));
                    if res.is_err() || terminate {
                        break;
                    }
                }
            }
        }
        if let Ok(to_tui) = to_tui_recv.try_recv() {
            match to_tui {
                ToTui::Neighbors(s_name, neighbors) => {
                    //TODO: first make sure neighbor is not placed on screen
                    for neighbor in neighbors.into_iter() {
                        let _tile_id = village.add_new_neighbor(neighbor, &mut mgr);
                    }
                }
                ToTui::AddContent(c_id, d_type) => {
                    let tile_id = village.add_new_content(d_type, c_id, &mut mgr);
                    // tiles_mapping.insert(tile_id, TileType::Content(d_type, c_id));
                }
                ToTui::Contents(c_id, text) => {
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
