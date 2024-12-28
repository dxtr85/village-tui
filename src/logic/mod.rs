use animaterm::prelude::*;
use async_std::task::sleep;
use dapp_lib::prelude::SwarmID;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
mod manifest;
use crate::tui::{CreatorResult, FromPresentation, TileType, ToPresentation};
pub use manifest::Manifest;
pub use manifest::Tag;

#[derive(Debug, Clone)]
enum TuiState {
    Village,
    AddTag,
    AddDType,
    RemovePage(ContentID),
    AppendData(ContentID),
    ContextMenuOn(TileType),
    Creator(Option<ContentID>, bool, DataType, String, Vec<u8>),
    CreatorSelectTags(Option<ContentID>, bool, DataType, String, Vec<u8>),
    CreatorDisplayDescription(Option<ContentID>, bool, DataType, String, Vec<u8>),
    CreatorSelectDtype(Option<ContentID>, bool, DataType, String, Vec<u8>),
    ReadRequestForIndexer(ContentID),
    Indexing(
        ContentID,
        Option<u16>,
        DataType,
        Vec<u8>,
        String,
        Vec<String>,
        Vec<String>,
    ),
}

struct SwarmShell {
    swarm_id: SwarmID,
    founder_id: GnomeId,
    manifest: Manifest,
}

pub struct ApplicationLogic {
    my_id: GnomeId,
    state: TuiState,
    active_swarm: SwarmShell,
    to_app_mgr_send: Sender<ToAppMgr>,
    to_tui: Sender<ToPresentation>,
    from_tui_send: Sender<FromPresentation>,
    from_tui_recv: Receiver<FromPresentation>,
    to_user_send: Sender<ToApp>,
    to_app: Receiver<ToApp>,
    pending_notifications: HashMap<SwarmID, Vec<ToApp>>,
}
impl ApplicationLogic {
    pub fn new(
        my_id: GnomeId,
        to_app_mgr_send: Sender<ToAppMgr>,
        to_tui_send: Sender<ToPresentation>,
        from_tui_send: Sender<FromPresentation>,
        from_tui_recv: Receiver<FromPresentation>,
        to_user_send: Sender<ToApp>,
        to_user_recv: Receiver<ToApp>,
    ) -> Self {
        ApplicationLogic {
            my_id,
            state: TuiState::Village,
            active_swarm: SwarmShell {
                swarm_id: SwarmID(0),
                founder_id: my_id,
                manifest: Manifest::new(AppType::Catalog, HashMap::new()),
            },
            to_app_mgr_send,
            to_tui: to_tui_send,
            from_tui_send,
            from_tui_recv,
            to_user_send,
            to_app: to_user_recv,
            pending_notifications: HashMap::new(),
        }
    }
    pub async fn run(&mut self) {
        let dur = Duration::from_millis(32);
        let mut home_swarm_enforced = false;

        let mut buffered_from_tui = vec![];
        'outer: loop {
            sleep(dur).await;
            if let Ok(from_tui) = self.from_tui_recv.try_recv() {
                match from_tui {
                    FromPresentation::TileSelected(tile) => {
                        match tile {
                            TileType::Home(g_id) => {
                                if g_id == self.my_id {
                                    self.run_creator();
                                }
                            }
                            TileType::Neighbor(g_id) => {
                                let _ = self.to_app_mgr_send.send(ToAppMgr::SetActiveApp(g_id));
                            }
                            TileType::Field => {
                                // TODO: do anything?
                            }
                            TileType::Application => {
                                //TODO
                            }
                            TileType::Content(dtype, c_id) => {
                                self.query_content_for_indexer(c_id);
                                eprintln!("About to show CID {} dtype: {:?}", c_id, dtype);
                            }
                        }
                    }
                    FromPresentation::CreateContent(d_type, data) => {
                        if !home_swarm_enforced {
                            eprintln!("Push back AppendContent");
                            buffered_from_tui.push(FromPresentation::CreateContent(d_type, data));
                            continue;
                        }
                        //TODO: we need to sync this data with Swarm
                        //TODO: we need to create logic that converts user data like String
                        //      into SyncData|CastData before we can send it to Swarm
                        eprintln!("Requesting AppendContent {}", data.get_hash());
                        let _ = self.to_app_mgr_send.send(ToAppMgr::AppendContent(
                            self.active_swarm.swarm_id,
                            d_type,
                            data,
                        ));
                    }
                    FromPresentation::UpdateContent(c_id, d_type, d_id, data) => {
                        if !home_swarm_enforced {
                            eprintln!("Push back AppendContent");
                            buffered_from_tui
                                .push(FromPresentation::UpdateContent(c_id, d_type, d_id, data));
                            continue;
                        }
                        eprintln!("Requesting ChangeContent");
                        let _ = self.to_app_mgr_send.send(ToAppMgr::ChangeContent(
                            self.active_swarm.swarm_id,
                            c_id,
                            d_type,
                            data,
                        ));
                    }
                    FromPresentation::AddTags(tags) => {
                        eprintln!("Received FromPresentation::AddTags({:?})", tags);
                        // TODO: first check if we can add a tag for given swarm
                        // and also check if given tag is not already added
                        //TODO: we need to temporarily add given tag to manifest,
                        // calculate hashes, and request app manager to modify or add Data
                        // blocks to datastore for given swarm
                        if self.active_swarm.manifest.add_tags(tags) {
                            // Now our manifest is out of sync with swarm
                            // we need to sync it with swarm.
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0));
                            // TODO: Probably it is better to hide this functionality from user
                            // and instead send a list of Data objects to update/create
                            let data_vec = self.active_swarm.manifest.to_data();
                            // TODO: this is not the data we want to send!
                            eprintln!("app logic received add tags request,\nsending change content to app mgr…");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ChangeContent(
                                self.active_swarm.swarm_id,
                                0,
                                DataType::Data(0),
                                data_vec,
                            ));
                        }
                    }
                    FromPresentation::AddDataType(tag) => {
                        eprintln!("Received FromPresentation::AddDataType({})", tag.0);
                        // TODO: first check if we can add a tag for given swarm
                        // and also check if given tag is not already added
                        //TODO: we need to temporarily add given tag to manifest,
                        // calculate hashes, and request app manager to modify or add Data
                        // blocks to datastore for given swarm
                        // if let Some(mut manifest) = self.active_swarm.manifest.take() {
                        if self.active_swarm.manifest.add_data_type(tag) {
                            // Now our manifest is out of sync with swarm
                            // we need to sync it with swarm.
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0));
                            // TODO: Probably it is better to hide this functionality from user
                            // and instead send a list of Data objects to update/create
                            let data_vec = self.active_swarm.manifest.to_data();
                            // TODO: this is not the data we want to send!
                            eprintln!("app logic received add data type request,\nsending change content to app mgr…");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ChangeContent(
                                self.active_swarm.swarm_id,
                                0,
                                DataType::Data(0),
                                data_vec,
                            ));
                        }
                    }
                    FromPresentation::NeighborSelected(gnome_id) => {
                        eprintln!("Selected neighbor: {:?}", gnome_id);
                        let _ = self.to_app_mgr_send.send(ToAppMgr::SetActiveApp(gnome_id));
                    }
                    FromPresentation::ShowContextMenu(ttype) => {
                        self.state = TuiState::ContextMenuOn(ttype);
                        match ttype {
                            TileType::Home(_g_id) => {
                                //TODO: select proper set_id depending on g_id value
                                let _ = self.to_tui.send(ToPresentation::DisplayCMenu(1));
                            }
                            TileType::Neighbor(g_id) => {
                                //TODO
                            }
                            TileType::Content(dtype, c_id) => {
                                //TODO
                                let _ = self.to_tui.send(ToPresentation::DisplayCMenu(2));
                            }
                            TileType::Field => {
                                //TODO
                            }
                            TileType::Application => {
                                //TODO
                            }
                        }
                    }
                    FromPresentation::SelectedIndices(indices) => {
                        eprintln!(
                            "FromPresentation::SelectedIndices({:?}), {:?}",
                            indices, self.state
                        );
                        let mut new_state = TuiState::Village;
                        match &self.state {
                            TuiState::CreatorSelectTags(
                                c_id_opt,
                                read_only,
                                dtype,
                                descr,
                                prev_indices,
                            ) => {
                                if !read_only {
                                    let mut indices_u8 = Vec::with_capacity(indices.len());
                                    for idx in &indices {
                                        indices_u8.push(*idx as u8);
                                    }
                                    new_state = TuiState::Creator(
                                        *c_id_opt,
                                        false,
                                        *dtype,
                                        descr.clone(),
                                        indices_u8.clone(),
                                    );
                                    let read_only = false;
                                    let tag_names =
                                        self.active_swarm.manifest.tags_string(&indices_u8);
                                    let dtype_name =
                                        self.active_swarm.manifest.dtype_string(dtype.byte());

                                    let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                        read_only,
                                        dtype_name,
                                        descr.clone(),
                                        tag_names,
                                    ));
                                } else {
                                    new_state = TuiState::Creator(
                                        *c_id_opt,
                                        *read_only,
                                        *dtype,
                                        descr.clone(),
                                        prev_indices.clone(),
                                    );
                                    if !indices.is_empty() {
                                        eprintln!(
                                            "We should show street tagged with: {:?}",
                                            self.active_swarm
                                                .manifest
                                                .tag_names(Some(vec![prev_indices[indices[0]]])) // Above has to be doubly de-indexed!
                                        );
                                    }
                                    let tag_names =
                                        self.active_swarm.manifest.tags_string(&prev_indices);
                                    let dtype_name =
                                        self.active_swarm.manifest.dtype_string(dtype.byte());

                                    let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                        true,
                                        dtype_name,
                                        descr.clone(),
                                        tag_names,
                                    ));
                                }
                            }
                            TuiState::CreatorSelectDtype(
                                c_id_opt,
                                read_only,
                                dtype,
                                descr,
                                tags,
                            ) => {
                                if indices.len() != 1 {
                                    eprintln!("Incorrect indices size for DType change");
                                    continue;
                                }

                                if c_id_opt.is_some() {
                                    eprintln!("cid is some");
                                    new_state = TuiState::Creator(
                                        *c_id_opt,
                                        *read_only,
                                        *dtype,
                                        descr.clone(),
                                        tags.clone(),
                                    );
                                    let tag_names = self.active_swarm.manifest.tags_string(&tags);
                                    let dtype_name =
                                        self.active_swarm.manifest.dtype_string(dtype.byte());
                                    eprintln!("{} new dtype_name: {}", dtype.byte(), dtype_name);

                                    let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                        *read_only,
                                        dtype_name,
                                        descr.clone(),
                                        tag_names,
                                    ));
                                } else {
                                    eprintln!("cid is none");
                                    new_state = TuiState::Creator(
                                        *c_id_opt,
                                        *read_only,
                                        DataType::from(indices[0] as u8),
                                        descr.clone(),
                                        tags.clone(),
                                    );
                                    let read_only = false;
                                    let tag_names = self.active_swarm.manifest.tags_string(tags);
                                    let dtype_name =
                                        self.active_swarm.manifest.dtype_string(indices[0] as u8);
                                    eprintln!("DType name: '{}'", dtype_name);

                                    let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                        read_only,
                                        dtype_name,
                                        descr.clone(),
                                        tag_names,
                                    ));
                                }
                            }
                            other => {
                                eprintln!("{:?} got Selected indices", other);
                            }
                        }
                        eprintln!("Setting state to: {:?}", new_state);
                        self.state = new_state;
                    }

                    FromPresentation::EditResult(e_result) => {
                        let mut new_state = TuiState::Village;
                        let prev_state = std::mem::replace(&mut self.state, TuiState::Village);
                        match prev_state {
                            TuiState::AddTag => {
                                if let Some(text) = e_result {
                                    let _ =
                                        self.from_tui_send.send(FromPresentation::AddTags(vec![
                                            Tag::new(text).unwrap(),
                                        ]));
                                }
                            }
                            TuiState::AddDType => {
                                if let Some(text) = e_result {
                                    let _ = self.from_tui_send.send(FromPresentation::AddDataType(
                                        Tag::new(text).unwrap(),
                                    ));
                                }
                            }
                            TuiState::AppendData(c_id) => {
                                if let Some(text) = e_result {
                                    let data = Data::new(text.bytes().collect()).unwrap();
                                    let _ = self.to_app_mgr_send.send(ToAppMgr::AppendData(
                                        self.active_swarm.swarm_id,
                                        c_id,
                                        data,
                                    ));
                                }
                            }
                            TuiState::CreatorDisplayDescription(
                                c_id_opt,
                                read_only,
                                dtype,
                                prev_descr,
                                tag_ids,
                            ) => {
                                if let Some(text) = e_result {
                                    eprintln!(
                                        "Got some text after Edit from Creator {}",
                                        read_only
                                    );
                                    let tag_names =
                                        self.active_swarm.manifest.tags_string(&tag_ids);
                                    let dtype_name =
                                        self.active_swarm.manifest.dtype_string(dtype.byte());
                                    if !read_only {
                                        new_state = TuiState::Creator(
                                            c_id_opt,
                                            read_only,
                                            dtype,
                                            text.clone(),
                                            tag_ids.clone(),
                                        );
                                        eprintln!("Requesting display creator again");
                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            read_only, dtype_name, text, tag_names,
                                        ));
                                    } else {
                                        new_state = TuiState::Creator(
                                            c_id_opt,
                                            read_only,
                                            dtype,
                                            prev_descr.clone(),
                                            tag_ids.clone(),
                                        );
                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            read_only,
                                            dtype_name,
                                            prev_descr.clone(),
                                            tag_names,
                                        ));
                                    }
                                }
                            }
                            TuiState::Indexing(
                                c_id,
                                d_id_opt,
                                d_type,
                                tag_ids,
                                descr,
                                all_headers,
                                notes,
                            ) => {
                                if let Some(text) = e_result {
                                    eprintln!("Got edit result while indexing");
                                    if let Some(d_id) = d_id_opt {
                                        let data = Data::new(text.bytes().collect()).unwrap();
                                        let _ = self.to_app_mgr_send.send(ToAppMgr::UpdateData(
                                            self.active_swarm.swarm_id,
                                            c_id,
                                            d_id,
                                            data,
                                        ));
                                    }
                                } else {
                                    new_state = TuiState::Indexing(
                                        c_id,
                                        d_id_opt,
                                        d_type,
                                        tag_ids,
                                        descr,
                                        all_headers.clone(),
                                        notes,
                                    );
                                    let _ = self
                                        .to_tui
                                        .send(ToPresentation::DisplayIndexer(all_headers));
                                }
                            }
                            _other => {
                                eprintln!("Unexpected EditResult");
                            }
                        }
                        self.state = new_state;
                    }
                    FromPresentation::CMenuAction(action) => {
                        let prev_state = std::mem::replace(&mut self.state, TuiState::Village);
                        match prev_state {
                            TuiState::ContextMenuOn(ttype) => match ttype {
                                TileType::Home(g_id) => {
                                    self.run_cmenu_action_on_home(action);
                                }
                                TileType::Content(d_type, c_id) => {
                                    self.run_cmenu_action_on_content(c_id, d_type, action);
                                }
                                other => {
                                    eprintln!(
                                        "{:?} tile does not support Context Menu action",
                                        other
                                    );
                                    self.state = TuiState::ContextMenuOn(other);
                                }
                            },
                            other => {
                                eprintln!("{:?} state does not support Context Menu action", other);
                                self.state = other;
                            }
                        }
                    }
                    FromPresentation::CreatorResult(c_result) => {
                        //TODO
                        match c_result {
                            CreatorResult::SelectDType => {
                                // TODO send request to presentation to show selector
                                let new_state;
                                match &self.state {
                                    TuiState::Creator(
                                        c_id_opt,
                                        read_only,
                                        dtype,
                                        descr,
                                        tag_ids,
                                    ) => {
                                        new_state = TuiState::CreatorSelectDtype(
                                            *c_id_opt,
                                            *read_only,
                                            *dtype,
                                            descr.clone(),
                                            tag_ids.clone(),
                                        );
                                        let _ = self.to_tui.send(ToPresentation::DisplaySelector(
                                            true,
                                            "Catalog Application's Data Types".to_string(),
                                            self.active_swarm.manifest.dtype_names(),
                                            vec![dtype.byte() as usize],
                                        ));
                                    }
                                    other => {
                                        eprintln!(
                                            "{:?}: Unexpected state for CreatorResult::SelectDType",
                                            self.state
                                        );
                                        new_state = other.clone();
                                    }
                                }
                                self.state = new_state;
                            }
                            CreatorResult::SelectTags => {
                                // TODO send request to presentation to show selector
                                let new_state;
                                match &self.state {
                                    TuiState::Creator(
                                        c_id_opt,
                                        read_only,
                                        dtype,
                                        descr,
                                        tag_ids,
                                    ) => {
                                        new_state = TuiState::CreatorSelectTags(
                                            *c_id_opt,
                                            *read_only,
                                            *dtype,
                                            descr.clone(),
                                            tag_ids.clone(),
                                        );
                                        let mut long_ids = Vec::with_capacity(tag_ids.len());
                                        let mut quit_on_first_select = false;
                                        let filter = if *read_only {
                                            quit_on_first_select = true;
                                            Some(tag_ids.clone())
                                        } else {
                                            for t_id in tag_ids {
                                                long_ids.push(*t_id as usize);
                                            }
                                            None
                                        };

                                        let _ = self.to_tui.send(ToPresentation::DisplaySelector(
                                            quit_on_first_select,
                                            "Catalog Application's Tags".to_string(),
                                            self.active_swarm.manifest.tag_names(filter),
                                            long_ids,
                                        ));
                                    }
                                    other => {
                                        eprintln!(
                                            "Unexpected state for CreatorResult::SelectDType"
                                        );
                                        new_state = other.clone();
                                    }
                                }
                                self.state = new_state;
                            }
                            CreatorResult::SelectDescription => {
                                // TODO send request to presentation to show editor
                                let new_state;
                                match &self.state {
                                    TuiState::Creator(
                                        c_id_opt,
                                        read_only,
                                        dtype,
                                        descr,
                                        tag_ids,
                                    ) => {
                                        new_state = TuiState::CreatorDisplayDescription(
                                            *c_id_opt,
                                            *read_only,
                                            *dtype,
                                            descr.clone(),
                                            tag_ids.clone(),
                                        );
                                        let _ = self.to_tui.send(ToPresentation::DisplayEditor(
                                            (*read_only,self.my_id ==self.active_swarm.founder_id),
                                    " Max size: 764  Multiline  Content Description    (TAB to finish)".to_string(),
                                    Some(descr.clone()),
                                    true,
                                    Some(764),
                                    ));
                                    }
                                    other => {
                                        eprintln!(
                                            "Unexpected state for CreatorResult::SelectDType"
                                        );
                                        new_state = other.clone();
                                    }
                                }
                                self.state = new_state;
                            }
                            CreatorResult::Cancel => {
                                self.state = TuiState::Village;
                                // TODO send request to presentation to show village
                                eprintln!("Cancel ");
                            }
                            CreatorResult::Create => {
                                if let TuiState::Creator(
                                    c_id_opt,
                                    read_only,
                                    d_type,
                                    descr,
                                    tag_indices,
                                ) = &self.state
                                {
                                    //TODO: send request to app_mgr
                                    // eprintln!("CreatorResult::Create(d_type, data)");
                                    if !home_swarm_enforced {
                                        eprintln!("Push back AppendContent");
                                        buffered_from_tui.push(FromPresentation::CreatorResult(
                                            CreatorResult::Create,
                                        ));
                                        continue;
                                    }
                                    //TODO: we need to sync this data with Swarm
                                    //TODO: we need to create logic that converts user data like String
                                    //      into SyncData|CastData before we can send it to Swarm
                                    if let Some(c_id) = c_id_opt {
                                        //TODO: here we update existing Content
                                        let mut bytes = Vec::with_capacity(1024);
                                        bytes.push(tag_indices.len() as u8);
                                        for tag in tag_indices {
                                            bytes.push(*tag as u8);
                                        }
                                        for byte in descr.bytes() {
                                            bytes.push(byte as u8);
                                        }
                                        let _ = self.to_app_mgr_send.send(ToAppMgr::UpdateData(
                                            self.active_swarm.swarm_id,
                                            *c_id,
                                            0,
                                            Data::new(bytes).unwrap(),
                                        ));
                                    } else {
                                        let mut bytes = Vec::with_capacity(1024);
                                        bytes.push(tag_indices.len() as u8);
                                        for tag in tag_indices {
                                            bytes.push(*tag as u8);
                                        }
                                        for byte in descr.bytes() {
                                            bytes.push(byte as u8);
                                        }
                                        let data = Data::new(bytes).unwrap();
                                        eprintln!("Requesting AppendContent {}", data.get_hash());
                                        let _ = self.to_app_mgr_send.send(ToAppMgr::AppendContent(
                                            self.active_swarm.swarm_id,
                                            *d_type,
                                            data,
                                        ));
                                    }
                                } else {
                                    eprintln!("Got TUI Create request when in {:?}", self.state);
                                }
                                self.state = TuiState::Village;
                            }
                        }
                    }
                    FromPresentation::KeyPress(key) => {
                        if self.handle_key(key) {
                            // eprintln!("Sending ToAppMgr::Quit");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::Quit);
                        }
                    }
                    FromPresentation::ContentInquiry(c_id) => {
                        if !home_swarm_enforced {
                            buffered_from_tui.push(FromPresentation::ContentInquiry(c_id));
                            continue;
                        }
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id));
                    }
                    FromPresentation::IndexResult(i_result) => {
                        let mut new_state = None;
                        match &self.state {
                            TuiState::RemovePage(c_id) => {
                                if let Some(page_id) = i_result {
                                    eprintln!(
                                        "We should request Page #{} removal from CID-{}",
                                        page_id, c_id
                                    );
                                    if page_id > 0 {
                                        let _ = self.to_app_mgr_send.send(ToAppMgr::RemoveData(
                                            self.active_swarm.swarm_id,
                                            *c_id,
                                            page_id as u16,
                                        ));
                                    } else {
                                        // TODO: make this logic built-into dapp-lib
                                        eprintln!("Unable to remove Page #0");
                                    }
                                }
                                new_state = Some(TuiState::Village);
                            }
                            TuiState::Indexing(
                                c_id,
                                d_id_opt,
                                d_type,
                                tag_ids,
                                descr,
                                headers,
                                notes,
                            ) => {
                                if let Some(idx) = i_result {
                                    match idx {
                                        0 => {
                                            let read_only =
                                                self.my_id != self.active_swarm.founder_id;
                                            eprintln!(
                                                "Should show Content header RO: {}",
                                                read_only
                                            );
                                            let (data_type, tags) = (
                                                self.active_swarm
                                                    .manifest
                                                    .dtype_string(d_type.byte()),
                                                self.active_swarm.manifest.tags_string(&tag_ids),
                                            );
                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplayCreator(
                                                    read_only,
                                                    data_type,
                                                    descr.clone(),
                                                    tags,
                                                ));
                                            new_state = Some(TuiState::Creator(
                                                Some(*c_id),
                                                read_only,
                                                *d_type,
                                                descr.clone(),
                                                tag_ids.clone(),
                                            ));
                                        }
                                        other => {
                                            eprintln!("Should show Note #{}", other);
                                            new_state = Some(TuiState::Indexing(
                                                *c_id,
                                                Some(other as u16),
                                                *d_type,
                                                tag_ids.clone(),
                                                descr.clone(),
                                                headers.clone(),
                                                notes.clone(),
                                            ));
                                            let read_only = true;
                                            let header = format!("Page #{}", other);
                                            let contents_opt =
                                                if let Some(text) = notes.clone().get(other) {
                                                    Some(text.clone())
                                                } else {
                                                    None
                                                };
                                            let allow_newlines = true;
                                            let byte_limit = Some(1024);
                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplayEditor(
                                                    (
                                                        read_only,
                                                        self.my_id == self.active_swarm.founder_id,
                                                    ),
                                                    header,
                                                    contents_opt.clone(),
                                                    allow_newlines,
                                                    byte_limit,
                                                ));
                                        }
                                    }
                                } else {
                                    self.state = TuiState::Village;
                                    eprintln!("Going back to Village");
                                }
                            }
                            other => {
                                eprintln!("Got Indexing response when in state {:?}", other);
                            }
                        }
                        if let Some(new_state) = new_state {
                            self.state = new_state;
                        }
                    }
                }
            }
            while let Ok(msg) = self.to_app.try_recv() {
                match msg {
                    ToApp::ActiveSwarm(f_id, s_id) => {
                        if !home_swarm_enforced {
                            home_swarm_enforced = true;
                            eprintln!("Requesting Manifest");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, 0));
                            for msg in buffered_from_tui.iter_mut() {
                                let _ = self.from_tui_send.send(msg.clone());
                            }
                        }
                        eprintln!("Set active {:?}", s_id);
                        self.active_swarm = SwarmShell {
                            swarm_id: s_id,
                            founder_id: f_id,
                            manifest: Manifest::new(AppType::Catalog, HashMap::new()),
                        };
                        if let Some(pending_notifications) = self
                            .pending_notifications
                            .remove(&self.active_swarm.swarm_id)
                        {
                            for note in pending_notifications.into_iter() {
                                let _ = self.to_user_send.send(note);
                            }
                        }
                        let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors);
                    }
                    ToApp::GetCIDsForTags(s_id, n_id, tags, all_first_pages) => {
                        eprintln!("App received request for tags: {:?}", tags);
                        for (c_id, data) in all_first_pages {
                            let mut tags_iter = data.ref_bytes().iter();
                            let tags_count = tags_iter.next().unwrap();
                            if *tags_count == 0 {
                                continue;
                            }
                            for _i in 0..*tags_count {
                                let tag = tags_iter.next().unwrap();
                                if tags.contains(tag) {
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::CIDsForTag(s_id, n_id, *tag, c_id, data));
                                    break;
                                }
                            }
                        }
                    }
                    ToApp::Neighbors(s_id, neighbors) => {
                        if !home_swarm_enforced {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::SetActiveApp(self.my_id));
                        }
                        if s_id == self.active_swarm.swarm_id {
                            if self.my_id == self.active_swarm.founder_id {
                                let _ = self.to_tui.send(ToPresentation::Neighbors(neighbors));
                            } else {
                                //TODO: here we need to insert our id as a Neighbor, and remove Swarm's founder from Neighbor list
                                let mut new_neighbors = vec![self.my_id];
                                for n in neighbors {
                                    if n == self.active_swarm.founder_id {
                                        eprintln!("Removing founder from neighbors list");
                                        continue;
                                    }
                                    new_neighbors.push(n);
                                }
                                let _ = self.to_tui.send(ToPresentation::Neighbors(new_neighbors));
                            }
                        } else {
                            eprintln!(
                                "Not sending neighbors, because my {:?} != {:?}",
                                self.active_swarm.swarm_id, s_id
                            );
                            self.pending_notifications
                                .entry(s_id)
                                .or_insert(vec![])
                                .push(ToApp::Neighbors(s_id, neighbors));
                        }
                    }
                    ToApp::NewContent(s_id, c_id, d_type) => {
                        if s_id == self.active_swarm.swarm_id && home_swarm_enforced {
                            eprintln!("ToApp::NewContent({:?},{:?})", c_id, d_type);
                            let _ = self
                                .to_tui
                                .send(ToPresentation::AppendContent(c_id, d_type));
                        } else {
                            eprintln!(
                                "Not sending new content, because my {:?} != {:?}",
                                self.active_swarm.swarm_id, s_id
                            );
                            self.pending_notifications
                                .entry(s_id)
                                .or_insert(vec![])
                                .push(ToApp::NewContent(s_id, c_id, d_type));
                        }
                    }
                    ToApp::ContentChanged(s_id, c_id) => {
                        if c_id == 0 {
                            eprintln!("Requesting ReadData for CID-0");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, c_id));
                        }
                        if s_id == self.active_swarm.swarm_id && home_swarm_enforced {
                            eprintln!("recv ToApp::ContentChanged({:?})", c_id);
                        } else {
                            eprintln!(
                                "Not sending changed content, because my {:?} != {:?}",
                                self.active_swarm.swarm_id, s_id
                            );
                        }
                    }
                    ToApp::ReadSuccess(s_id, c_id, d_type, d_vec) => {
                        eprintln!("Received ReadSuccess {:?} {}", s_id, c_id);
                        if s_id == self.active_swarm.swarm_id {
                            self.process_data(c_id, d_type, d_vec);
                        } else {
                            self.pending_notifications
                                .entry(s_id)
                                .or_insert(vec![])
                                .push(ToApp::ReadSuccess(s_id, c_id, d_type, d_vec));
                        }
                    }
                    ToApp::ReadError(s_id, c_id, error) => {
                        eprintln!("Received ReadError for {:?} {} {}", s_id, c_id, error);
                        if matches!(error, AppError::AppDataNotSynced) && c_id == 0 {
                            //TODO: some delay would be nice
                            eprintln!("Requesting {} again…", c_id);
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, c_id));
                        }
                        if s_id == self.active_swarm.swarm_id {
                            if c_id == 0 && self.my_id == self.active_swarm.founder_id {
                            } else {
                                let _ = self.to_tui.send(ToPresentation::ReadError(c_id, error));
                            }
                        }
                    }
                    ToApp::Disconnected => {
                        eprintln!("Done serving ApplicationLogic");
                        break 'outer;
                    }
                }
            }
        }
    }

    fn process_data(&mut self, c_id: ContentID, d_type: DataType, mut d_vec: Vec<Data>) {
        if c_id == 0 {
            eprintln!(
                "Sending Manifest d_vec.len: {} to Presentation",
                d_vec.len()
            );
            let manifest = Manifest::from(d_vec);
            self.active_swarm.manifest = manifest;
        } else {
            match &self.state {
                TuiState::ReadRequestForIndexer(rc_id) | TuiState::RemovePage(rc_id) => {
                    if *rc_id == c_id {
                        // eprintln!("Run ReadRequestForIndexer on {} data items", d_vec.len());
                        //TODO: get first line of every data for Indexer to present
                        // first data is a header,
                        // for i in 0..d_vec.len() {
                        //     eprintln!("Data #{}:\n{:?}", i, d_vec[i]);
                        // }
                        let mut all_notes = Vec::with_capacity(d_vec.len());
                        all_notes.push(String::new());
                        let mut all_headers = Vec::with_capacity(d_vec.len());
                        let mut description = String::new();
                        let first_data = d_vec.remove(0);
                        let mut bytes = first_data.bytes();
                        let how_many_tags = bytes.remove(0);
                        let mut tag_ids = Vec::with_capacity(how_many_tags as usize);
                        for _i in 0..how_many_tags {
                            if !bytes.is_empty() {
                                tag_ids.push(bytes.remove(0));
                            } else {
                                eprintln!("This should not happen!");
                            }
                        }
                        description.push_str(&String::from_utf8(bytes).unwrap());
                        let mut header = String::new();
                        if !description.is_empty() {
                            for c in description.lines().next().unwrap().chars() {
                                header.push(c);
                            }
                        }
                        if header.is_empty() {
                            header = "No description".to_string();
                        }
                        // eprintln!("Pushing '{}' to all_headers", header);
                        all_headers.push(header);
                        // then each data is a note
                        for data in d_vec {
                            let text = String::from_utf8(data.bytes()).unwrap();
                            let mut header = String::new();
                            if !text.is_empty() {
                                for c in text.lines().next().unwrap().chars() {
                                    header.push(c);
                                }
                            }
                            if header.is_empty() {
                                header = "Nothing to show".to_string();
                            }
                            // eprintln!("Pushing '{}' to all_headers", header);
                            all_headers.push(header);
                            all_notes.push(text);
                        }
                        // when done, request TUI to DisplayIndexer with given options
                        // eprintln!("Done pushing to all_headers");
                        let _ = self
                            .to_tui
                            .send(ToPresentation::DisplayIndexer(all_headers.clone()));
                        // and update state to store all data that was read
                        if !matches!(self.state, TuiState::RemovePage(_c_id)) {
                            self.state = TuiState::Indexing(
                                c_id,
                                None,
                                d_type,
                                tag_ids,
                                description,
                                all_headers,
                                all_notes,
                            );
                        }
                    } else {
                        eprintln!("ReadSuccess on {}, was expecting: {}", c_id, rc_id);
                    }
                }
                other => {
                    eprintln!("ReadSuccess when in state: {:?}", other);
                }
            }
        }
    }

    fn run_creator(&mut self) {
        self.state = TuiState::Creator(None, false, DataType::Data(0), String::new(), vec![]);
        let read_only = false;
        let d_type = self.active_swarm.manifest.dtype_string(0);
        let tags = String::new();
        let description = String::new();
        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
            read_only,
            d_type,
            tags,
            description,
        ));
    }

    fn run_cmenu_action_on_content(&mut self, c_id: ContentID, d_type: DataType, action: usize) {
        match action {
            1 => {
                //TODO
                self.state = TuiState::AppendData(c_id);
                let _ = self.to_tui.send(ToPresentation::DisplayEditor(
                    (false, true),
                    "Add Note".to_string(),
                    None,
                    true,
                    Some(1024),
                ));
            }
            2 => {
                // eprintln!("Setting state to RemovePage({})", c_id);
                self.query_content_for_indexer(c_id);
                self.state = TuiState::RemovePage(c_id);
            }
            other => {
                //TODO
                eprintln!("{} Context Menu action on Content", other);
            }
        }
    }
    fn run_cmenu_action_on_home(&mut self, action: usize) {
        //TODO
        eprintln!(
            "We should perform action: {} when in {:?}",
            action, self.state
        );
        match action {
            1 => {
                let _ = self.to_tui.send(ToPresentation::DisplaySelector(
                    false,
                    "Catalog Application's Tags".to_string(),
                    self.active_swarm.manifest.tag_names(None),
                    vec![],
                ));
            }
            2 => {
                let _ = self.to_tui.send(ToPresentation::DisplaySelector(
                    true,
                    "Catalog Application's Data Types".to_string(),
                    self.active_swarm.manifest.dtype_names(),
                    vec![],
                ));
            }
            3 => {
                self.state = TuiState::AddTag;
                let _ = self.to_tui.send(ToPresentation::DisplayEditor(
                    (false, true),
                    " Max size: 32  Oneline  Define new Tag name    (TAB to finish)".to_string(),
                    None,
                    false,
                    Some(32),
                ));
            }
            4 => {
                self.state = TuiState::AddDType;
                let _ = self.to_tui.send(ToPresentation::DisplayEditor(
                    (false, true),
                    " Max size: 32  Oneline  Define new Data Type    (TAB to finish)".to_string(),
                    None,
                    false,
                    Some(32),
                ));
            }
            5 => {
                self.run_creator();
            }
            _other => {
                //TODO
            }
        }
    }

    fn query_content_for_indexer(&mut self, c_id: ContentID) {
        eprintln!("In query_content_for_indexer");
        //TODO: first we need to retrieve Pages in order to have something to present
        if matches!(self.state, TuiState::Village) {
            self.state = TuiState::ReadRequestForIndexer(c_id);
            let _ = self
                .to_app_mgr_send
                .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id));
        } else {
            eprintln!("Can not run query when in state: {:?}", self.state);
        }
    }

    fn handle_key(&self, key: Key) -> bool {
        match key {
            Key::U => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::UploadData);
            }
            Key::J => {
                // TODO: indirect send via AppMgr
                // let _ = gmgr_send.send(ManagerRequest::JoinSwarm("trzat".to_string()));
            }
            Key::Q | Key::ShiftQ => {
                // TODO: indirect send via AppMgr
                // let _ = gmgr_send.send(ManagerRequest::Disconnect);
                // keep_running = false;
                return true;
            }
            // TODO: this should be served separately by sending to user_req
            Key::B => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::StartBroadcast);
                // let res = service_request.send(Request::StartBroadcast);
                // b_req_sent = res.is_ok();
            }
            Key::ShiftB => {
                eprintln!("ShiftB");
                let _ = self.to_app_mgr_send.send(ToAppMgr::EndBroadcast);
                // let res = service_request.send(Request::StartBroadcast);
                // b_req_sent = res.is_ok();
            }
            Key::CtrlB => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::UnsubscribeBroadcast);
            }
            Key::CtrlU => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::TransformLinkRequest(Box::new(
                        SyncData::new(vec![27; 1024]).unwrap(),
                    )));
            }
            Key::M => {
                // Here we send request to read manifest data
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0));
            }
            Key::N => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors);
            }
            Key::C => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::ChangeContent(
                    self.active_swarm.swarm_id,
                    1,
                    DataType::from(0),
                    vec![Data::empty(0)],
                ));
            }
            Key::S => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::AppendContent(
                    self.active_swarm.swarm_id,
                    DataType::Data(0),
                    Data::empty(0),
                ));
            }
            Key::ShiftS => {
                // TODO: extend this message with actual content
                let _ = self.to_app_mgr_send.send(ToAppMgr::AppendContent(
                    self.active_swarm.swarm_id,
                    DataType::Data(0),
                    Data::empty(0),
                ));
            }
            Key::ShiftU => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::StartUnicast);
            }
            _ => eprintln!(),
        }
        false
    }
}
fn text_to_data(text: String) -> Data {
    Data::new(text.try_into().unwrap()).unwrap()
}
