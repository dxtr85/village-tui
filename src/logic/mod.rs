use animaterm::prelude::*;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use dapp_lib::prelude::Description;
use dapp_lib::prelude::SwarmID;
use dapp_lib::prelude::SwarmName;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::mpsc::Sender;
mod manifest;
use crate::tui::Direction;
use crate::tui::{CreatorResult, FromPresentation, TileType, ToPresentation};
use crate::InternalMsg;
pub use manifest::Manifest;
pub use manifest::Tag;

#[derive(Debug, Clone)]
pub enum CreatorContext {
    Data {
        c_id: Option<ContentID>,
        read_only: bool,
        d_type: DataType,
        description: Description,
        tags: Vec<u8>,
    },
    Link {
        c_id: Option<ContentID>,
        read_only: bool,
        s_name: SwarmName,
        target_id: ContentID,
        description: Description,
        tags: Vec<u8>,
        ti_opt: Option<TransformInfo>,
    },
}
impl CreatorContext {
    pub fn is_read_only(&self) -> bool {
        match self {
            Self::Data { read_only, .. } => *read_only,
            Self::Link { read_only, .. } => *read_only,
        }
    }
    pub fn set_tags(&mut self, new_tags: Vec<u8>) {
        match self {
            Self::Data { tags, .. } => *tags = new_tags,
            Self::Link { tags, .. } => *tags = new_tags,
        }
    }
    pub fn set_description(&mut self, new_description: Description) {
        match self {
            Self::Data { description, .. } => *description = new_description,
            Self::Link { description, .. } => *description = new_description,
        }
    }
    pub fn set_data_type(&mut self, new_type: DataType) {
        match self {
            Self::Data { d_type, .. } => *d_type = new_type,
            Self::Link {
                tags,
                c_id,
                read_only,
                s_name: _,
                target_id: _,
                description,
                ti_opt,
            } => {
                if new_type.is_link() {
                    return;
                }
                *self = Self::Data {
                    c_id: *c_id,
                    read_only: *read_only,
                    d_type: new_type,
                    description: description.clone(),
                    tags: tags.clone(),
                }
            }
        }
    }
    pub fn get_tags(&self) -> Vec<u8> {
        match self {
            Self::Data { tags, .. } => tags.clone(),
            Self::Link { tags, .. } => tags.clone(),
        }
    }
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Data { d_type, .. } => *d_type,
            Self::Link { .. } => DataType::Link,
        }
    }
    pub fn description(&self) -> Description {
        match self {
            Self::Data { description, .. } => description.clone(),
            Self::Link { description, .. } => description.clone(),
        }
    }
    pub fn content_id(&self) -> Option<ContentID> {
        match self {
            Self::Data { c_id, .. } => c_id.clone(),
            Self::Link { c_id, .. } => c_id.clone(),
        }
    }
    pub fn link_target(&self) -> Option<(SwarmName, ContentID, Data, Option<TransformInfo>)> {
        match self {
            Self::Data { .. } => None,
            Self::Link {
                s_name,
                target_id,
                tags,
                ti_opt,
                ..
            } => {
                let t_len = tags.len() as u8;
                let mut d_source = Vec::with_capacity(t_len as usize + 1);
                d_source.push(t_len);
                for t in tags {
                    d_source.push(*t);
                }
                let d = Data::new(d_source).unwrap();
                Some((s_name.clone(), *target_id, d, ti_opt.clone()))
            }
        }
    }
}
#[derive(Debug, Clone)]
enum TuiState {
    Village,
    AddTag,
    AddDType,
    RemovePage(ContentID),
    AppendData(ContentID),
    ContextMenuOn(TileType),
    Creator(CreatorContext),
    CreatorSelectTags(CreatorContext),
    CreatorDisplayDescription(CreatorContext),
    CreatorSelectDtype(CreatorContext),
    ReadRequestForIndexer(ContentID),
    ReadLinkToFollow(ContentID, Option<(SwarmName, ContentID)>),
    //TODO: Indexing should also have a context, or we should never index a Link
    Indexing(
        ContentID,
        Option<u16>,
        DataType,
        Vec<u8>,
        String,
        Vec<String>,
        Vec<String>,
    ),
    ShowActiveSwarms(Vec<(SwarmName, SwarmID)>),
}

struct SwarmShell {
    swarm_id: SwarmID,
    swarm_name: SwarmName,
    manifest: Manifest,
    tag_to_cid: HashMap<Tag, HashSet<(DataType, ContentID, String)>>,
    tag_ring: Vec<Vec<Tag>>,
}

impl SwarmShell {
    pub fn new(swarm_id: SwarmID, swarm_name: SwarmName, app_type: AppType) -> Self {
        SwarmShell {
            swarm_id,
            swarm_name,
            manifest: Manifest::new(app_type, HashMap::new()),
            tag_to_cid: HashMap::new(),
            tag_ring: Vec::new(),
        }
    }
    //TODO: we need to define logic for tag_to_cid
    // When joining a Swarm, after sync we read all main pages
    // For each CID main page we insert that CID for every entry
    //   matching Tag it has on his main page
    // When content has changed we update every entry in tag_to_cid,
    //   in order to match actual state.
    // If a Tag was added/deleted and if corresponding Street is visible
    //   we send proper notification ToPresentation
    pub fn update_tag_to_cid(
        &mut self,
        c_id: ContentID,
        d_type: DataType,
        tags: Vec<Tag>,
        header: String,
    ) -> (Vec<Tag>, Vec<Tag>) {
        eprintln!("update_tag_to_cid {}: {:?}", c_id, tags);
        let mut newly_added = vec![];
        let mut removed = vec![];
        let to_add = (d_type, c_id, header.clone());
        if tags.is_empty() {
            if let Some(set) = self.tag_to_cid.get_mut(&Tag::empty()) {
                if set.insert((d_type, c_id, header)) {
                    newly_added.push(Tag::empty());
                }
            } else {
                let mut set = HashSet::new();
                set.insert((d_type, c_id, header));
                self.tag_to_cid.insert(Tag::empty(), set);
                newly_added.push(Tag::empty());
            }
            return (newly_added, removed);
        }
        for new_tag in &tags {
            eprintln!("processing {:?}", new_tag);
            if let Some(cids) = self.tag_to_cid.get_mut(new_tag) {
                eprintln!("found");
                //todo
                if cids.contains(&to_add) {
                    eprintln!("contains");
                    // Do nothing
                } else {
                    eprintln!("inserting");
                    let _ = cids.insert(to_add.clone());
                    newly_added.push(new_tag.clone());
                }
            } else {
                let mut hsadd = HashSet::new();
                hsadd.insert(to_add.clone());
                eprintln!("hash set: {:?}", hsadd);
                let res = self.tag_to_cid.insert(new_tag.clone(), hsadd);
                eprintln!("not found,insert result: {:?}", res);
                newly_added.push(new_tag.clone());
            }
        }
        // eprintln!("before tag_to_cid: {:?}", self.tag_to_cid);
        for (e_tag, cids) in self.tag_to_cid.iter_mut() {
            if cids.contains(&to_add) {
                if tags.contains(e_tag) {
                    // Do nothing all is fine
                } else {
                    let _ = cids.remove(&to_add);
                    removed.push(e_tag.clone());
                }
                // } else {
                //     if tags.contains(e_tag) {
                //         let _ = cids.insert(to_add);
                //         newly_added.push(e_tag.clone());
                //     } else {
                //         // Do nothing all is fine
                //     }
            }
        }
        // eprintln!("after tag_to_cid: {:?}", self.tag_to_cid);
        (newly_added, removed)
    }

    pub fn get_cids_for_tag(&self, tag: Tag) -> HashSet<(DataType, ContentID, String)> {
        if let Some(contents) = self.tag_to_cid.get(&tag) {
            contents.clone()
        } else {
            HashSet::new()
        }
    }
    pub fn clear_tag_to_cid(&mut self) {
        for c_ids in self.tag_to_cid.values_mut() {
            *c_ids = HashSet::new();
        }
    }
}

pub struct ApplicationLogic {
    my_name: SwarmName,
    pub_ips: Vec<(IpAddr, u16, Nat, (PortAllocationRule, i8))>,
    state: TuiState,
    active_swarm: SwarmShell,
    to_app_mgr_send: ASender<ToAppMgr>,
    to_tui: Sender<ToPresentation>,
    // from_tui_send: Sender<FromPresentation>,
    // from_tui_recv: Receiver<FromPresentation>,
    to_user_send: ASender<InternalMsg>,
    to_app: AReceiver<InternalMsg>,
    notification_sender: ASender<Option<String>>,
    pending_notifications: HashMap<SwarmID, Vec<ToApp>>, //TODO: clear pending
    // when given SwarmID was disconnected
    visible_streets: (usize, Vec<Tag>), // (how many streets visible at once, visible street names),
    home_swarm_enforced: bool,
    buffered_from_tui: Vec<FromPresentation>,
    clipboard: Option<(SwarmName, ContentID)>,
}
impl ApplicationLogic {
    pub fn new(
        my_name: SwarmName,
        to_app_mgr_send: ASender<ToAppMgr>,
        to_tui_send: Sender<ToPresentation>,
        notification_sender: ASender<Option<String>>,
        // from_tui_send: Sender<FromPresentation>,
        // from_tui_recv: Receiver<FromPresentation>,
        // to_user_send: Sender<ToApp>,
        // to_user_recv: Receiver<ToApp>,
        to_user_send: ASender<InternalMsg>,
        to_user_recv: AReceiver<InternalMsg>,
    ) -> Self {
        ApplicationLogic {
            my_name,
            pub_ips: vec![],
            state: TuiState::Village,
            active_swarm: SwarmShell::new(
                SwarmID(0),
                SwarmName {
                    founder: GnomeId::any(),
                    name: String::new(),
                },
                AppType::Catalog,
            ),
            to_app_mgr_send,
            to_tui: to_tui_send,
            // from_tui_send,
            // from_tui_recv,
            to_user_send,
            to_app: to_user_recv,
            pending_notifications: HashMap::new(),
            visible_streets: (0, vec![]),
            home_swarm_enforced: false,
            buffered_from_tui: vec![],
            notification_sender,
            clipboard: None,
        }
    }
    pub async fn run(&mut self) {
        'outer: loop {
            while let Ok(internal_msg) = self.to_app.recv().await {
                match internal_msg {
                    InternalMsg::User(msg) => match msg {
                        ToApp::ActiveSwarm(s_name, s_id) => {
                            eprintln!("Requesting Manifest");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, 0)).await;
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ReadAllFirstPages(s_id))
                                .await;
                            let prev_state = std::mem::replace(&mut self.state, TuiState::Village);
                            if let TuiState::ReadLinkToFollow(
                                _c_id,
                                Some((target_name, target_cid)),
                            ) = prev_state
                            {
                                //TODO
                                // 4 open target content
                                if target_name == s_name {
                                    self.state = TuiState::ReadRequestForIndexer(target_cid);
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::ReadData(s_id, target_cid))
                                        .await;
                                } else {
                                    eprintln!(
                                        "Had to follow a link, but switched to a different swarm"
                                    );
                                }
                            }
                            if !self.home_swarm_enforced {
                                self.home_swarm_enforced = true;
                                for msg in self.buffered_from_tui.iter_mut() {
                                    // let _ = self.from_tui_send.send(msg.clone());
                                    let _ =
                                        self.to_user_send.send(InternalMsg::Tui(msg.clone())).await;
                                }
                            }
                            eprintln!("Set active {}", s_id);
                            self.active_swarm = SwarmShell::new(s_id, s_name, AppType::Catalog);
                            if let Some(pending_notifications) = self
                                .pending_notifications
                                .remove(&self.active_swarm.swarm_id)
                            {
                                for note in pending_notifications.into_iter() {
                                    let _ = self.to_user_send.send(InternalMsg::User(note)).await;
                                }
                            }
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors).await;
                        }
                        ToApp::MyPublicIPs(ip_list) => {
                            //TODO: push this to my swarm's Manifest after syncing
                            eprintln!("\nApplication got Pub IPs:\n{:?}\n", ip_list);
                            for ip_tuple in ip_list {
                                if !self.pub_ips.contains(&ip_tuple) {
                                    self.pub_ips.push(ip_tuple);
                                }
                            }
                        }
                        ToApp::NameToIDMapping(mapping) => {
                            // TODO: show after mgr responds with a list
                            if mapping.is_empty() {
                                //TODO: display a notification
                                // that there are no synced swarms available
                                self.state = TuiState::Village;
                                continue;
                            }
                            let mut map_vec = Vec::with_capacity(mapping.len());
                            let mut name_vec = Vec::with_capacity(mapping.len());
                            for (s_name, s_id) in mapping {
                                map_vec.push((s_name.clone(), s_id));
                                name_vec.push(format!("Swarm ID {}: {}", s_id.0, s_name));
                            }
                            self.state = TuiState::ShowActiveSwarms(map_vec);
                            self.show_active_swarms(name_vec);
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
                                            .send(ToAppMgr::CIDsForTag(
                                                s_id, n_id, *tag, c_id, data,
                                            ))
                                            .await;
                                        break;
                                    }
                                }
                            }
                        }
                        ToApp::Neighbors(s_id, neighbors) => {
                            if !self.home_swarm_enforced {
                                eprintln!("!home_swarm_enforced");
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::SetActiveApp(self.my_name.clone()))
                                    .await;
                                self.pending_notifications
                                    .entry(s_id)
                                    .or_insert(vec![])
                                    .push(ToApp::Neighbors(s_id, neighbors));
                                continue;
                            }
                            if s_id == self.active_swarm.swarm_id {
                                if self.my_name == self.active_swarm.swarm_name {
                                    eprintln!("App got neighbors: {:?}", neighbors);
                                    if !neighbors.is_empty() {
                                        // eprintln!("{} Sending Neighbors0: {:?}", s_id, neighbors);
                                        let _ =
                                            self.to_tui.send(ToPresentation::Neighbors(neighbors));
                                    }
                                } else {
                                    //TODO: here we need to insert our id as a Neighbor, and remove Swarm's founder from Neighbor list
                                    let mut new_neighbors = vec![self.my_name.founder];
                                    for n in neighbors {
                                        if n == self.active_swarm.swarm_name.founder {
                                            eprintln!("Removing {} from neighbors list", n);
                                            continue;
                                        }
                                        new_neighbors.push(n);
                                    }
                                    eprintln!("Presenting Neighbors: {:?}", new_neighbors);
                                    let _ =
                                        self.to_tui.send(ToPresentation::Neighbors(new_neighbors));
                                }
                            } else {
                                eprintln!(
                                    "Not sending neighbors, because my {} != {}",
                                    self.active_swarm.swarm_id, s_id
                                );
                                self.pending_notifications
                                    .entry(s_id)
                                    .or_insert(vec![])
                                    .push(ToApp::Neighbors(s_id, neighbors));
                            }
                        }
                        ToApp::NewContent(s_id, c_id, d_type, main_page) => {
                            //TODO: read tags from main page
                            if s_id == self.active_swarm.swarm_id && self.home_swarm_enforced {
                                self.update_active_content_tags(s_id, c_id, d_type, main_page);
                            } else {
                                eprintln!(
                                    "Not sending new content, because my {} != {}( home swarm enforced: {})",
                                    self.active_swarm.swarm_id, s_id,self.home_swarm_enforced
                                );
                                self.pending_notifications
                                    .entry(s_id)
                                    .or_insert(vec![])
                                    .push(ToApp::NewContent(s_id, c_id, d_type, main_page));
                            }
                        }
                        ToApp::ContentChanged(s_id, c_id, d_type, main_page_option) => {
                            if c_id == 0 {
                                eprintln!("Requesting ReadData for CID-0");
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ReadData(s_id, c_id))
                                    .await;
                            }
                            if s_id == self.active_swarm.swarm_id && self.home_swarm_enforced {
                                if let Some(main_page) = main_page_option {
                                    if main_page.is_empty() && main_page.get_hash() > 0 {
                                        eprintln!("We should check if Tags have changed");
                                        let _ = self
                                            .to_app_mgr_send
                                            .send(ToAppMgr::ReadData(s_id, c_id))
                                            .await;
                                    } else {
                                        self.update_active_content_tags(
                                            s_id, c_id, d_type, main_page,
                                        );
                                    }
                                }
                                eprintln!("recv ToApp::ContentChanged({:?})", c_id);
                            } else {
                                eprintln!(
                                    "Not sending changed content, because my {:?} != {:?}",
                                    self.active_swarm.swarm_id, s_id
                                );
                            }
                        }
                        ToApp::ReadSuccess(s_id, c_id, d_type, d_vec) => {
                            // eprintln!(
                            //     "Received ReadSuccess {} CID-{} (len: {})",
                            //     s_id,
                            //     c_id,
                            //     d_vec.len()
                            // );
                            if s_id == self.active_swarm.swarm_id {
                                // eprintln!("processing it");
                                self.process_data(c_id, d_type, d_vec).await;
                            } else {
                                // eprintln!(
                                //     "shelving it (active swarm: {:?})",
                                //     self.active_swarm.swarm_id
                                // );
                                self.pending_notifications
                                    .entry(s_id)
                                    .or_insert(vec![])
                                    .push(ToApp::ReadSuccess(s_id, c_id, d_type, d_vec));
                            }
                        }
                        ToApp::ReadError(s_id, c_id, error) => {
                            // eprintln!("Received ReadError for {} CID-{}: {}", s_id, c_id, error);
                            if matches!(error, AppError::AppDataNotSynced) && c_id == 0 {
                                //TODO: some delay would be nice
                                // eprintln!("Requesting CID-{} again…", c_id);
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ReadData(s_id, c_id))
                                    .await;
                            }
                            // if s_id == self.active_swarm.swarm_id {
                            //     if c_id == 0 && self.my_name == self.active_swarm.swarm_name {
                            //     } else {
                            //         let _ =
                            //             self.to_tui.send(ToPresentation::ReadError(c_id, error));
                            //     }
                            // }
                        }
                        ToApp::FirstPages(s_id, first_pages) => {
                            //TODO: we need to store this information in SwarmShell
                            // we also need to know what Contents are already displayed
                            // and we should only send ToPresentation those contents
                            // that are not already visible, but should be
                            if s_id == self.active_swarm.swarm_id {
                                eprintln!("App received first pages: {}", first_pages.len());
                                for (c_id, d_type, main_page) in first_pages {
                                    self.update_active_content_tags(s_id, c_id, d_type, main_page);
                                }
                            } else {
                                self.pending_notifications
                                    .entry(s_id)
                                    .or_insert(vec![])
                                    .push(ToApp::FirstPages(s_id, first_pages));
                            }
                        }
                        ToApp::Disconnected(is_reconnecting, s_id, s_name) => {
                            self.swarm_disconnected(is_reconnecting, s_id, s_name).await
                            // TODO: maybe later we can allow offline read-only mode
                            // but for now we need to implement something simple
                            //
                        }
                        ToApp::Quit => {
                            eprintln!("Done serving ApplicationLogic");
                            break 'outer;
                        }
                    },
                    InternalMsg::Tui(from_tui) => match from_tui {
                        FromPresentation::VisibleStreetsCount(v_streets) => {
                            //TODO: store this in order to know what information to send
                            eprintln!("OK we see {} streets at once", v_streets);
                            self.visible_streets.0 = v_streets as usize;
                        }
                        FromPresentation::TileSelected(tile) => {
                            match tile {
                                TileType::Home(g_id) => {
                                    if g_id == self.my_name.founder {
                                        self.run_creator();
                                    }
                                }
                                TileType::Neighbor(g_id) => {
                                    eprintln!("NeighborSelected");
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::SetActiveApp(SwarmName {
                                            founder: g_id,
                                            name: "/".to_string(),
                                        }))
                                        .await;
                                }
                                TileType::Field => {
                                    // TODO: do anything?
                                }
                                TileType::Application => {
                                    //TODO
                                }
                                TileType::Content(dtype, c_id) => {
                                    if dtype.is_link() {
                                        eprintln!("About to follow a link");
                                        self.follow_link(c_id).await;
                                    } else {
                                        self.query_content_for_indexer(c_id).await;
                                        eprintln!("About to show CID {} dtype: {:?}", c_id, dtype);
                                    }
                                }
                            }
                        }
                        FromPresentation::CursorOutOfScreen(direction) => {
                            match direction {
                                Direction::Up => {
                                    //TODO
                                    eprintln!("We should draw new streets on screen UP");
                                    if self.active_swarm.tag_ring.len() > 1 {
                                        let prev_names = self.active_swarm.tag_ring.remove(0);
                                        self.active_swarm.tag_ring.push(prev_names);
                                        let street_names = self.active_swarm.tag_ring[0].clone();
                                        self.visible_streets.1 = street_names.clone();
                                        eprintln!("Sending StreetNames: {:?}", street_names);
                                        let mut streets_with_contents = vec![];
                                        for street in street_names {
                                            let contents =
                                                self.active_swarm.get_cids_for_tag(street.clone());
                                            streets_with_contents.push((street, contents));
                                        }
                                        let _ = self.to_tui.send(ToPresentation::StreetNames(
                                            streets_with_contents,
                                        ));
                                    }
                                }
                                Direction::Down => {
                                    //TODO
                                    eprintln!("We should draw new streets on screen DOWN");
                                    if self.active_swarm.tag_ring.len() > 1 {
                                        let street_names =
                                            self.active_swarm.tag_ring.pop().unwrap();
                                        self.active_swarm.tag_ring.insert(0, street_names.clone());
                                        self.visible_streets.1 = street_names.clone();
                                        eprintln!("Sending StreetNames: {:?}", street_names);
                                        let mut streets_with_contents = vec![];
                                        for street in street_names {
                                            let contents =
                                                self.active_swarm.get_cids_for_tag(street.clone());
                                            streets_with_contents.push((street, contents));
                                        }
                                        let _ = self.to_tui.send(ToPresentation::StreetNames(
                                            streets_with_contents,
                                        ));
                                    }
                                }
                                Direction::Left => {
                                    //TODO
                                }
                                Direction::Right => {
                                    //TODO
                                }
                            }
                        }
                        FromPresentation::CreateContent(d_type, data) => {
                            if !self.home_swarm_enforced {
                                eprintln!("Push back AppendContent");
                                self.buffered_from_tui
                                    .push(FromPresentation::CreateContent(d_type, data));
                                continue;
                            }
                            //TODO: we need to sync this data with Swarm
                            //TODO: we need to create logic that converts user data like String
                            //      into SyncData|CastData before we can send it to Swarm
                            eprintln!("Requesting AppendContent {}", data.get_hash());
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::AppendContent(
                                    self.active_swarm.swarm_id,
                                    d_type,
                                    data,
                                ))
                                .await;
                        }
                        FromPresentation::UpdateContent(c_id, d_type, d_id, data) => {
                            if !self.home_swarm_enforced {
                                eprintln!("Push back AppendContent");
                                self.buffered_from_tui.push(FromPresentation::UpdateContent(
                                    c_id, d_type, d_id, data,
                                ));
                                continue;
                            }
                            eprintln!("Requesting ChangeContent");
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ChangeContent(
                                    self.active_swarm.swarm_id,
                                    c_id,
                                    d_type,
                                    data,
                                ))
                                .await;
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
                                    .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0))
                                    .await;
                                // TODO: Probably it is better to hide this functionality from user
                                // and instead send a list of Data objects to update/create
                                let data_vec = self.active_swarm.manifest.to_data();
                                // TODO: this is not the data we want to send!
                                eprintln!("app logic received add tags request,\nsending change content to app mgr…");
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ChangeContent(
                                        self.active_swarm.swarm_id,
                                        0,
                                        DataType::Data(0),
                                        data_vec,
                                    ))
                                    .await;
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
                                    .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0))
                                    .await;
                                // TODO: Probably it is better to hide this functionality from user
                                // and instead send a list of Data objects to update/create
                                let data_vec = self.active_swarm.manifest.to_data();
                                // TODO: this is not the data we want to send!
                                eprintln!("app logic received add data type request,\nsending change content to app mgr…");
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ChangeContent(
                                        self.active_swarm.swarm_id,
                                        0,
                                        DataType::Data(0),
                                        data_vec,
                                    ))
                                    .await;
                            }
                        }
                        FromPresentation::NeighborSelected(s_name) => {
                            eprintln!("Selected neighbor swarm: {}", s_name);
                            if self.active_swarm.swarm_name == s_name {
                                eprintln!("Already showing selected swarm");
                                continue;
                            }
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::SetActiveApp(s_name))
                                .await;
                        }
                        FromPresentation::ShowContextMenu(ttype) => {
                            match ttype {
                                TileType::Home(_g_id) => {
                                    self.state = TuiState::ContextMenuOn(ttype);
                                    //TODO: select proper set_id depending on g_id value
                                    let _ = self.to_tui.send(ToPresentation::DisplayCMenu(1));
                                }
                                TileType::Neighbor(_g_id) => {
                                    //TODO
                                }
                                TileType::Content(dtype, c_id) => {
                                    //TODO
                                    self.state = TuiState::ContextMenuOn(ttype);
                                    let _ = self.to_tui.send(ToPresentation::DisplayCMenu(2));
                                }
                                TileType::Field => {
                                    //TODO
                                    self.state = TuiState::ContextMenuOn(ttype);
                                    let _ = self.to_tui.send(ToPresentation::DisplayCMenu(3));
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
                                TuiState::CreatorSelectTags(creator_context) => {
                                    if !creator_context.is_read_only() {
                                        let mut indices_u8 = Vec::with_capacity(indices.len());
                                        for idx in &indices {
                                            indices_u8.push(*idx as u8);
                                        }
                                        let mut new_context = creator_context.clone();
                                        new_context.set_tags(indices_u8.clone());
                                        new_state = TuiState::Creator(new_context);
                                        let tag_names =
                                            self.active_swarm.manifest.tags_string(&indices_u8);
                                        let dtype_name = self
                                            .active_swarm
                                            .manifest
                                            .dtype_string(creator_context.data_type().byte());

                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            false,
                                            dtype_name,
                                            creator_context.description().text(),
                                            tag_names,
                                        ));
                                    } else {
                                        let new_context = creator_context.clone();
                                        let prev_indices = creator_context.get_tags();
                                        new_state = TuiState::Creator(new_context);
                                        if !indices.is_empty() {
                                            eprintln!(
                                                "We should show street tagged with: {:?}",
                                                self.active_swarm.manifest.tag_names(Some(vec![
                                                    prev_indices[indices[0]]
                                                ])) // Above has to be doubly de-indexed!
                                            );
                                        }
                                        let tag_names =
                                            self.active_swarm.manifest.tags_string(&prev_indices);
                                        let dtype_name = self
                                            .active_swarm
                                            .manifest
                                            .dtype_string(creator_context.data_type().byte());

                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            true,
                                            dtype_name,
                                            creator_context.description().text(),
                                            tag_names,
                                        ));
                                    }
                                }
                                // TuiState::CreatorSelectDtype(
                                //     c_id_opt,
                                //     read_only,
                                //     dtype,
                                //     descr,
                                //     tags,
                                TuiState::CreatorSelectDtype(c_context) => {
                                    if indices.len() != 1 {
                                        eprintln!("Incorrect indices size for DType change");
                                        continue;
                                    }

                                    if c_context.content_id().is_some() {
                                        eprintln!("cid is some");
                                        new_state = TuiState::Creator(c_context.clone());
                                        let tag_names = self
                                            .active_swarm
                                            .manifest
                                            .tags_string(&c_context.get_tags());
                                        let dtype_name = self
                                            .active_swarm
                                            .manifest
                                            .dtype_string(c_context.data_type().byte());
                                        eprintln!(
                                            "{} new dtype_name: {}",
                                            c_context.data_type().byte(),
                                            dtype_name
                                        );

                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            c_context.is_read_only(),
                                            dtype_name,
                                            c_context.description().text(),
                                            tag_names,
                                        ));
                                    } else {
                                        eprintln!("cid is none");
                                        let mut new_context = c_context.clone();
                                        new_context.set_data_type(DataType::from(indices[0] as u8));
                                        new_state = TuiState::Creator(new_context);
                                        let read_only = false;
                                        let tag_names = self
                                            .active_swarm
                                            .manifest
                                            .tags_string(&c_context.get_tags());
                                        let dtype_name = self
                                            .active_swarm
                                            .manifest
                                            .dtype_string(indices[0] as u8);
                                        eprintln!("DType name: '{}'", dtype_name);

                                        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
                                            read_only,
                                            dtype_name,
                                            c_context.description().text(),
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
                                        // let _ =
                                        //     self.from_tui_send.send(FromPresentation::AddTags(vec![
                                        //         Tag::new(text).unwrap(),
                                        //     ]));
                                        let _ = self
                                            .to_user_send
                                            .send(InternalMsg::Tui(FromPresentation::AddTags(
                                                vec![Tag::new(text).unwrap()],
                                            )))
                                            .await;
                                    }
                                }
                                TuiState::AddDType => {
                                    if let Some(text) = e_result {
                                        // let _ = self.from_tui_send.send(FromPresentation::AddDataType(
                                        //     Tag::new(text).unwrap(),
                                        // ));
                                        let _ = self
                                            .to_user_send
                                            .send(InternalMsg::Tui(FromPresentation::AddDataType(
                                                Tag::new(text).unwrap(),
                                            )))
                                            .await;
                                    }
                                }
                                TuiState::AppendData(c_id) => {
                                    if let Some(text) = e_result {
                                        let data = Data::new(text.bytes().collect()).unwrap();
                                        let _ = self
                                            .to_app_mgr_send
                                            .send(ToAppMgr::AppendData(
                                                self.active_swarm.swarm_id,
                                                c_id,
                                                data,
                                            ))
                                            .await;
                                    }
                                }
                                TuiState::CreatorDisplayDescription(
                                    c_context,
                                    // c_id_opt,
                                    // read_only,
                                    // dtype,
                                    // prev_descr,
                                    // tag_ids,
                                ) => {
                                    if let Some(text) = e_result {
                                        eprintln!(
                                            "Got some text after Edit from Creator {}",
                                            c_context.is_read_only()
                                        );
                                        let tag_names = self
                                            .active_swarm
                                            .manifest
                                            .tags_string(&c_context.get_tags());
                                        let dtype_name = self
                                            .active_swarm
                                            .manifest
                                            .dtype_string(c_context.data_type().byte());
                                        if !c_context.is_read_only() {
                                            let mut new_context = c_context.clone();
                                            new_context.set_description(
                                                Description::new(text.clone()).unwrap(),
                                            );
                                            new_state = TuiState::Creator(new_context);
                                            eprintln!("Requesting display creator again");
                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplayCreator(
                                                    false, dtype_name, text, tag_names,
                                                ));
                                        } else {
                                            let descr = c_context.description();
                                            new_state = TuiState::Creator(c_context);
                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplayCreator(
                                                    true,
                                                    dtype_name,
                                                    descr.text(),
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
                                            let _ = self
                                                .to_app_mgr_send
                                                .send(ToAppMgr::UpdateData(
                                                    self.active_swarm.swarm_id,
                                                    c_id,
                                                    d_id,
                                                    data,
                                                ))
                                                .await;
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
                                        self.run_cmenu_action_on_home(action).await;
                                    }
                                    TileType::Content(d_type, c_id) => {
                                        self.run_cmenu_action_on_content(c_id, d_type, action)
                                            .await;
                                    }
                                    TileType::Field => {
                                        self.run_cmenu_action_on_field(action).await;
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
                                    eprintln!(
                                        "{:?} state does not support Context Menu action",
                                        other
                                    );
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
                                            c_context, // c_id_opt,
                                                       // read_only,
                                                       // dtype,
                                                       // descr,
                                                       // tag_ids,
                                        ) => {
                                            new_state = TuiState::CreatorSelectDtype(
                                                c_context.clone(), // *c_id_opt,
                                                                   // *read_only,
                                                                   // *dtype,
                                                                   // descr.clone(),
                                                                   // tag_ids.clone(),
                                            );
                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplaySelector(
                                                    true,
                                                    "Catalog Application's Data Types".to_string(),
                                                    self.active_swarm.manifest.dtype_names(),
                                                    vec![c_context.data_type().byte() as usize],
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
                                            c_context, // c_id_opt,
                                                       // read_only,
                                                       // dtype,
                                                       // descr,
                                                       // tag_ids,
                                        ) => {
                                            new_state = TuiState::CreatorSelectTags(
                                                c_context.clone(), // *c_id_opt,
                                                                   // *read_only,
                                                                   // *dtype,
                                                                   // descr.clone(),
                                                                   // tag_ids.clone(),
                                            );
                                            let tag_ids = c_context.get_tags();
                                            let mut long_ids = Vec::with_capacity(tag_ids.len());
                                            let mut quit_on_first_select = false;
                                            let filter = if c_context.is_read_only() {
                                                quit_on_first_select = true;
                                                Some(tag_ids.clone())
                                            } else {
                                                for t_id in tag_ids {
                                                    long_ids.push(t_id as usize);
                                                }
                                                None
                                            };

                                            let _ =
                                                self.to_tui.send(ToPresentation::DisplaySelector(
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
                                            c_context, // c_id_opt,
                                                       // read_only,
                                                       // dtype,
                                                       // descr,
                                                       // tag_ids,
                                        ) => {
                                            new_state = TuiState::CreatorDisplayDescription(
                                                c_context.clone(), // *c_id_opt,
                                                                   // *read_only,
                                                                   // *dtype,
                                                                   // descr.clone(),
                                                                   // tag_ids.clone(),
                                            );
                                            let _ = self.to_tui.send(ToPresentation::DisplayEditor(
                                            (c_context.is_read_only(),self.my_name ==self.active_swarm.swarm_name),
                                    " Max size: 764  Multiline  Content Description    (TAB to finish)".to_string(),
                                    Some(c_context.description().text()),
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
                                        c_context, // c_id_opt,
                                                   // read_only,
                                                   // d_type,
                                                   // descr,
                                                   // tag_indices,
                                    ) = &self.state
                                    {
                                        //TODO: send request to app_mgr
                                        // eprintln!("CreatorResult::Create(d_type, data)");
                                        if !self.home_swarm_enforced {
                                            eprintln!("Push back AppendContent");
                                            self.buffered_from_tui.push(
                                                FromPresentation::CreatorResult(
                                                    CreatorResult::Create,
                                                ),
                                            );
                                            continue;
                                        }
                                        //TODO: we need to sync this data with Swarm
                                        //TODO: we need to create logic that converts user data like String
                                        //      into SyncData|CastData before we can send it to Swarm
                                        let tag_indices = c_context.get_tags();
                                        let descr = c_context.description();
                                        let mut bytes = Vec::with_capacity(1024);
                                        if c_context.data_type().is_link() {
                                            let (s_name, target_cid, data, ti_opt) =
                                                c_context.link_target().unwrap();
                                            // eprintln!("We've got to update a link, dunno how");
                                            let content = Content::Link(
                                                s_name, target_cid, descr, data, ti_opt,
                                            );
                                            bytes = content.to_data().unwrap().bytes();
                                            // let s_bytes = s_name.as_bytes();
                                            // for byte in s_bytes {
                                            //     bytes.push(byte);
                                            // }
                                            // let [t1, t2] = target_cid.to_be_bytes();
                                            // bytes.push(t1);
                                            // bytes.push(t2);
                                            // bytes.push(tag_indices.len() as u8);
                                            // for tag in tag_indices {
                                            //     bytes.push(tag as u8);
                                            // }
                                            // for byte in descr.bytes() {
                                            //     bytes.push(byte as u8);
                                            // }
                                        } else {
                                            // here we update existing Content
                                            bytes.push(tag_indices.len() as u8);
                                            for tag in tag_indices {
                                                bytes.push(tag as u8);
                                            }
                                            for byte in descr.text().bytes() {
                                                bytes.push(byte as u8);
                                            }
                                        }
                                        let data_res = Data::new(bytes);
                                        if let Ok(data) = data_res {
                                            if let Some(c_id) = c_context.content_id() {
                                                let _ = self
                                                    .to_app_mgr_send
                                                    .send(ToAppMgr::UpdateData(
                                                        self.active_swarm.swarm_id,
                                                        c_id,
                                                        0,
                                                        data,
                                                    ))
                                                    .await;
                                            } else {
                                                eprintln!(
                                                    "Requesting AppendContent {:?} {}",
                                                    data,
                                                    data.get_hash()
                                                );
                                                let _ = self
                                                    .to_app_mgr_send
                                                    .send(ToAppMgr::AppendContent(
                                                        self.active_swarm.swarm_id,
                                                        c_context.data_type(),
                                                        data,
                                                    ))
                                                    .await;
                                            }
                                        } else {
                                            eprintln!(
                                                "Failed to build Data from bytes (too many bytes?)"
                                            );
                                        }
                                    } else {
                                        eprintln!(
                                            "Got TUI Create request when in {:?}",
                                            self.state
                                        );
                                    }
                                    self.state = TuiState::Village;
                                }
                            }
                        }
                        FromPresentation::KeyPress(key) => {
                            if self.handle_key(key).await {
                                // eprintln!("Sending ToAppMgr::Quit");
                                let _ = self.to_app_mgr_send.send(ToAppMgr::Quit).await;
                            }
                        }
                        FromPresentation::ContentInquiry(c_id) => {
                            if !self.home_swarm_enforced {
                                self.buffered_from_tui
                                    .push(FromPresentation::ContentInquiry(c_id));
                                continue;
                            }
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id))
                                .await;
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
                                            let _ = self
                                                .to_app_mgr_send
                                                .send(ToAppMgr::RemoveData(
                                                    self.active_swarm.swarm_id,
                                                    *c_id,
                                                    page_id as u16,
                                                ))
                                                .await;
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
                                                    self.my_name != self.active_swarm.swarm_name;
                                                eprintln!(
                                                    "Should show Content header RO: {}",
                                                    read_only
                                                );
                                                let (data_type, tags) = (
                                                    self.active_swarm
                                                        .manifest
                                                        .dtype_string(d_type.byte()),
                                                    self.active_swarm
                                                        .manifest
                                                        .tags_string(&tag_ids),
                                                );
                                                let _ = self.to_tui.send(
                                                    ToPresentation::DisplayCreator(
                                                        read_only,
                                                        data_type,
                                                        descr.clone(),
                                                        tags,
                                                    ),
                                                );
                                                let c_context = CreatorContext::Data {
                                                    c_id: Some(*c_id),
                                                    read_only,
                                                    d_type: *d_type,
                                                    description: Description::new(descr.clone())
                                                        .unwrap(),
                                                    tags: tag_ids.clone(),
                                                };
                                                // if d_type.is_link(){
                                                //     let s_name = SwarmName::new(GnomeId::any(),format!("/"));
                                                //     CreatorContext::Link { c_id: Some(*c_id), read_only, s_name , target_id: 0, description: descr.clone(), tags: tag_ids.clone() }else{
                                                //         CreatorContext::Data { c_id: Some(*c_id), read_only, d_type: *d_type, description: descr.clone(), tags: tag_ids.clone() }
                                                //     }
                                                // };
                                                new_state = Some(TuiState::Creator(
                                                    c_context, // Some(*c_id),
                                                              // read_only,
                                                              // *d_type,
                                                              // descr.clone(),
                                                              // tag_ids.clone(),
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
                                                let _ = self.to_tui.send(
                                                    ToPresentation::DisplayEditor(
                                                        (
                                                            read_only,
                                                            self.my_name
                                                                == self.active_swarm.swarm_name,
                                                        ),
                                                        header,
                                                        contents_opt.clone(),
                                                        allow_newlines,
                                                        byte_limit,
                                                    ),
                                                );
                                            }
                                        }
                                    } else {
                                        self.state = TuiState::Village;
                                        eprintln!("Going back to Village");
                                    }
                                }
                                TuiState::ShowActiveSwarms(mapping) => {
                                    if let Some(idx) = i_result {
                                        eprintln!(
                                            "Now we should activate swarm with id: {:?}",
                                            idx
                                        );
                                        //TODO: we need a mapping from idx -> gnome_id
                                        if let Some((swarm_name, _s_id)) = mapping.get(idx) {
                                            new_state = Some(TuiState::Village);
                                            let _ = self
                                                .to_app_mgr_send
                                                .send(ToAppMgr::SetActiveApp(swarm_name.clone()))
                                                .await;
                                            let _ = self.to_tui.send(ToPresentation::SwapTiles(
                                                swarm_name.founder,
                                            ));
                                        }
                                    } else {
                                        eprintln!("Going back to Village");
                                        self.state = TuiState::Village;
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
                    },
                }
            }
        }
    }

    async fn process_data(&mut self, c_id: ContentID, d_type: DataType, mut d_vec: Vec<Data>) {
        if c_id == 0 {
            // eprintln!(
            //     "Sending Manifest d_vec.len: {} to Presentation",
            //     d_vec.len()
            // );
            let manifest = Manifest::from(d_vec);
            eprintln!("Process data manifest tags: {:?}", manifest.tags);
            // let mut streets_left_to_present = self.visible_streets.0;
            // let mut street_names = Vec::with_capacity(streets_left_to_present);
            // let mut street_idx = 0;
            // eprintln!("Streets left to present: {}", streets_left_to_present);
            // while streets_left_to_present > 0 {
            let mut tag_ring = vec![];
            let mut tag_set = vec![];
            let mut remaining_to_add = self.visible_streets.0;
            // for tag in manifest.tags.values(){

            // }
            for street_idx in 0..=255 {
                // for street_name in manifest.tags.values() {
                if let Some(street_name) = manifest.tags.get(&street_idx) {
                    if remaining_to_add > 0 {
                        remaining_to_add -= 1;
                        tag_set.push(street_name.clone());
                    } else {
                        tag_ring.push(tag_set);
                        tag_set = vec![street_name.clone()];
                        // remaining_to_add = self.visible_streets.0 - 1;
                    }
                    //     street_names.push(street_name.clone());
                    //     streets_left_to_present = streets_left_to_present - 1;
                    //     street_idx += 1;
                    // } else {
                    //     // eprintln!("Was expecting more streets!");
                    //     break;
                }
            }
            if remaining_to_add > 0 {
                tag_set.push(Tag::empty());
                tag_ring.push(tag_set);
            } else {
                tag_ring.push(tag_set);
                tag_ring.push(vec![Tag::empty()]);
            }
            let street_names = tag_ring[0].clone();
            // if street_names.is_empty() {
            //     street_names.push(Tag::empty());
            // }
            // for i in 0..streets_left_to_present {
            //     street_names.push(Tag(format!("Generic street #{}", i)));
            // }
            self.active_swarm.manifest = manifest;
            self.active_swarm.tag_ring = tag_ring;
            if street_names != self.visible_streets.1 {
                self.visible_streets.1 = street_names.clone();
                eprintln!("Sending StreetNames: {:?}", street_names);
                let mut streets_with_contents = vec![];
                for street in street_names {
                    let contents = self.active_swarm.get_cids_for_tag(street.clone());
                    streets_with_contents.push((street, contents));
                }
                let _ = self
                    .to_tui
                    .send(ToPresentation::StreetNames(streets_with_contents));
            }
            if let Some(pending_notifications) = self
                .pending_notifications
                .remove(&self.active_swarm.swarm_id)
            {
                for note in pending_notifications.into_iter() {
                    let _ = self.to_user_send.send(InternalMsg::User(note)).await;
                }
            }
            if self.my_name == self.active_swarm.swarm_name && !self.pub_ips.is_empty() {
                let ips = std::mem::replace(&mut self.pub_ips, vec![]);
                if self.active_swarm.manifest.update_pub_ips(ips) {
                    eprintln!("Now we should sync updated Manifest to Swarm");
                    let data_vec = self.active_swarm.manifest.to_data();
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::ChangeContent(
                            self.active_swarm.swarm_id,
                            0,
                            DataType::Data(0),
                            data_vec,
                        ))
                        .await;
                }
            }
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
                        let first_data = d_vec.remove(0);
                        let (tag_ids, description) =
                            read_tags_and_header(d_type, first_data.clone());
                        let mut bytes = first_data.bytes();
                        let _ = bytes.remove(0);
                        for _i in 0..tag_ids.len() {
                            let _ = bytes.remove(0);
                        }
                        // let how_many_tags = bytes.remove(0);
                        // let mut tag_ids = Vec::with_capacity(how_many_tags as usize);
                        // for _i in 0..how_many_tags {
                        //     if !bytes.is_empty() {
                        //         tag_ids.push(bytes.remove(0));
                        //     } else {
                        //         eprintln!("This should not happen!");
                        //     }
                        // }
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
                TuiState::ReadLinkToFollow(rc_id, _opt) => {
                    if *rc_id == c_id {
                        // 2 convert data to link
                        let first_data = d_vec.remove(0);
                        let link = data_to_link(first_data).unwrap();
                        if let Some((s_name, target_c_id, descr, data, ti_opt)) = link.link_params()
                        {
                            // 3 set target swarm active
                            eprintln!("ReadLinkToFollow");
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::SetActiveApp(s_name.clone()))
                                .await;
                            let _ = self.to_tui.send(ToPresentation::SwapTiles(s_name.founder));
                            self.state =
                                TuiState::ReadLinkToFollow(c_id, Some((s_name, target_c_id)));
                        } else {
                            eprintln!("Unable to read link params");
                        }
                    } else {
                        eprintln!("ReadSuccess on {}, was expecting link: {}", c_id, rc_id);
                    }
                }
                other => {
                    eprintln!("ReadSuccess when in state: {:?}", other);
                }
            }
        }
    }

    fn update_active_content_tags(
        &mut self,
        s_id: SwarmID,
        c_id: ContentID,
        d_type: DataType,
        main_page: Data,
    ) {
        // eprintln!("ToApp::NewContent({:?},{:?})", c_id, d_type);
        // eprintln!("CID-{} Main page: {:?}", c_id, main_page);
        let (tag_ids, header) = read_tags_and_header(d_type, main_page.clone());

        let ids_len = tag_ids.len();
        // eprintln!("{} tag ids: {:?}", c_id, tag_ids);
        // eprintln!("Manifest tags: {:?}", self.active_swarm.manifest.tags);
        let mut tags = Vec::with_capacity(tag_ids.len());
        for id in tag_ids {
            if let Some(tag) = self.active_swarm.manifest.tags.get(&id) {
                tags.push(tag.clone());
            }
        }
        if tags.len() != ids_len {
            eprintln!("Manifest not synced, shelving NewContent message");
            self.pending_notifications
                .entry(s_id)
                .or_insert(vec![])
                .push(ToApp::NewContent(s_id, c_id, d_type, main_page));
        } else {
            let (added, removed) =
                self.active_swarm
                    .update_tag_to_cid(c_id, d_type, tags, header.clone());
            eprintln!("added: {:?}\nremoved: {:?}", added, removed);
            if !added.is_empty() {
                let _ = self
                    .to_tui
                    .send(ToPresentation::AppendContent(c_id, d_type, added, header));
            }
            if !removed.is_empty() {
                let mut visible_removed = vec![];
                for str_name in &self.visible_streets.1 {
                    if removed.contains(&str_name) {
                        visible_removed.push(str_name.clone());
                    }
                }
                if !visible_removed.is_empty() {
                    let _ = self
                        .to_tui
                        .send(ToPresentation::HideContent(c_id, visible_removed));
                }
            }
        }
    }
    fn run_creator(&mut self) {
        let c_context = CreatorContext::Data {
            c_id: None,
            read_only: false,
            d_type: DataType::Data(0),
            description: Description::new(String::new()).unwrap(),
            tags: vec![],
        };
        self.state = TuiState::Creator(c_context);
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
    fn run_link_creator(
        &mut self,
        s_name: SwarmName,
        target_id: ContentID,
        description: Description,
        tags: Vec<u8>,
    ) {
        let c_context = CreatorContext::Link {
            c_id: None,
            read_only: false,
            s_name,
            target_id,
            description: description.clone(),
            tags,
            ti_opt: None,
        };
        self.state = TuiState::Creator(c_context);
        let read_only = false;
        let d_type = format!("Link");
        let tags = String::new();
        let _ = self.to_tui.send(ToPresentation::DisplayCreator(
            read_only,
            d_type,
            description.text(),
            tags,
        ));
    }
    fn show_public_ips(&self) {
        // eprintln!("We should show Public IPs");
        let mut public_ips = String::with_capacity(256);
        for (ip, port, nat, (rule, delta)) in &self.active_swarm.manifest.pub_ips {
            public_ips.push_str(&format!(
                "IP: {} PORT: {} NAT: {:?} Port alloc: {:?}({})",
                ip, port, nat, rule, delta
            ));
            public_ips.push('\n');
        }
        let _ = self.to_tui.send(ToPresentation::DisplayEditor(
            (true, false), // (read_only, can_edit)
            format!("Publiczne IP dla {}", self.active_swarm.swarm_name),
            Some(public_ips),
            false, // allow_newlines
            None,  // byte_limit
        ));
    }
    fn show_active_swarms(&mut self, active_swarms: Vec<String>) {
        // eprintln!("We should show active Swarms");
        // eprintln!("We should create mapping swarm_id => gnome_id");
        let _ = self
            .to_tui
            .send(ToPresentation::DisplayIndexer(active_swarms));
    }
    async fn swarm_disconnected(
        &mut self,
        is_reconnecting: bool,
        s_id: SwarmID,
        s_name: SwarmName,
    ) {
        if self.active_swarm.swarm_id == s_id {
            let _ = self
                .notification_sender
                .send(Some(format!("{} disconnected", s_name)))
                .await;
            let _ = self.to_tui.send(ToPresentation::SwapTiles(GnomeId::any()));
            self.active_swarm.clear_tag_to_cid();
            self.state = TuiState::ShowActiveSwarms(vec![]);
            let _ = self
                .to_app_mgr_send
                .send(ToAppMgr::ProvideGnomeToSwarmMapping)
                .await;
            // TODO: In this case if active swarm != our own swarm
            // we can simply switch to our own & send a notification
            // that current swarm got disconnected
            // If our own swarm is not operational or
            // if it was our own swarm then we could provide user
            // with a list of swarms that are still operational
            // to choose from.
            // If the number of operational swarms has changed while
            // we were being presented that list, it should get updated.
            // And when the list becomes empty we might
            // close swarm selector and post a note
            // informing that all swarms are disconnected, also
            // if the list is empty we simply reset presentation layer
            // to display only greenfields
            //
            // When reconnecting we could also lookup our storage for
            // public IPs to connect to
            eprintln!(
                "Active {} {} disconnected (Reconnecting: {})",
                s_id, s_name, is_reconnecting
            );
        } else if s_name == self.my_name {
            // TODO: Here our owned swarm got disconnected,
            // but our current swarm is operational,
            // so we might just post a notification
            eprintln!(
                "Owned {} {} disconnected (Reconnecting: {})",
                s_id, s_name, is_reconnecting
            );
        } else {
            // Here we log that a swarm got disconnected, but we do not
            // need to inform user about it, since this should be
            // common occurence
            eprintln!(
                "{} {} disconnected (Reconnecting: {})",
                s_id, s_name, is_reconnecting
            );
        }
    }
    async fn run_cmenu_action_on_content(
        &mut self,
        c_id: ContentID,
        d_type: DataType,
        action: usize,
    ) {
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
                self.query_content_for_indexer(c_id).await;
                self.state = TuiState::RemovePage(c_id);
            }
            3 => {
                eprintln!("We should copy a Link of selected Content");
                self.clipboard = Some((self.active_swarm.swarm_name.clone(), c_id));
                let _ = self
                    .notification_sender
                    .send(Some(format!("Zawartość {} skopiowana", c_id)))
                    .await;
            }
            other => {
                //TODO
                eprintln!("{} Context Menu action on Content", other);
            }
        }
    }
    async fn run_cmenu_action_on_home(&mut self, action: usize) {
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
            6 => {
                self.show_public_ips();
            }
            7 => {
                eprintln!("Show active Swarms");
                self.state = TuiState::ShowActiveSwarms(vec![]);
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ProvideGnomeToSwarmMapping)
                    .await;
            }
            7 => {
                eprintln!("Show known Swarms");
            }
            _other => {
                //TODO
            }
        }
    }
    async fn run_cmenu_action_on_field(&mut self, action: usize) {
        match action {
            1 => {
                //TODO: now we have to open up Creator filled with
                // Link data provided in clipboard, and allow user to create a Link
                // TODO: first we have to make sure clipboard holds SwarmName
                // and not SwarmID
                if let Some((s_name, c_id)) = self.clipboard.take() {
                    let _ = self
                        .notification_sender
                        .send(Some(format!("O właśnie :D")))
                        .await;
                    let descr = Description::new(format!("Link to {}-{}", s_name, c_id)).unwrap();
                    self.run_link_creator(s_name, c_id, descr, vec![]);
                } else {
                    let _ = self
                        .notification_sender
                        .send(Some(format!("Najpierw skopiuj Zawartość")))
                        .await;
                }
            }
            other => {
                //TODO
                eprintln!("{} Context Menu action on Content", other);
            }
        }
    }
    async fn follow_link(&mut self, c_id: ContentID) {
        eprintln!("In follow_link");
        // TODO
        // 1 read contents
        self.state = TuiState::ReadLinkToFollow(c_id, None);
        let _ = self
            .to_app_mgr_send
            .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id))
            .await;
    }
    async fn query_content_for_indexer(&mut self, c_id: ContentID) {
        eprintln!("In query_content_for_indexer");
        //TODO: first we need to retrieve Pages in order to have something to present
        if matches!(self.state, TuiState::Village) {
            self.state = TuiState::ReadRequestForIndexer(c_id);
            let _ = self
                .to_app_mgr_send
                .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id))
                .await;
        } else {
            eprintln!("Can not run query when in state: {:?}", self.state);
        }
    }

    async fn handle_key(&self, key: Key) -> bool {
        match key {
            Key::U => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::UploadData).await;
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
                let _ = self.to_app_mgr_send.send(ToAppMgr::StartBroadcast).await;
                // let res = service_request.send(Request::StartBroadcast);
                // b_req_sent = res.is_ok();
            }
            Key::ShiftB => {
                eprintln!("ShiftB");
                let _ = self.to_app_mgr_send.send(ToAppMgr::EndBroadcast).await;
                // let res = service_request.send(Request::StartBroadcast);
                // b_req_sent = res.is_ok();
            }
            Key::CtrlB => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::UnsubscribeBroadcast)
                    .await;
            }
            Key::CtrlU => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::TransformLinkRequest(Box::new(
                        SyncData::new(vec![27; 1024]).unwrap(),
                    )))
                    .await;
            }
            Key::M => {
                // Here we send request to read manifest data
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, 0))
                    .await;
            }
            Key::N => {
                let _res = self
                    .notification_sender
                    .send(Some(format!("Testowa notka")))
                    .await;
                // let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors).await;
            }
            Key::C => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ChangeContent(
                        self.active_swarm.swarm_id,
                        1,
                        DataType::from(0),
                        vec![Data::empty(0)],
                    ))
                    .await;
            }
            Key::S => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AppendContent(
                        self.active_swarm.swarm_id,
                        DataType::Data(0),
                        Data::empty(0),
                    ))
                    .await;
            }
            Key::ShiftS => {
                // TODO: extend this message with actual content
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AppendContent(
                        self.active_swarm.swarm_id,
                        DataType::Data(0),
                        Data::empty(0),
                    ))
                    .await;
            }
            Key::ShiftU => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::StartUnicast).await;
            }
            _ => eprintln!(),
        }
        false
    }
}
fn read_tags_and_header(d_type: DataType, data: Data) -> (Vec<u8>, String) {
    if data.is_empty() {
        return (vec![], String::new());
    }
    if d_type.is_link() {
        let link = data_to_link(data).unwrap();
        return (link.tag_ids(), link.description());
        // eprintln!("Not updating Links for now");
        // return (vec![], String::new());
    };
    let mut bytes = data.bytes();
    let how_many_tags = bytes.remove(0);
    // eprintln!("We have {} tags", how_many_tags);
    let mut tag_ids = Vec::with_capacity(how_many_tags as usize);
    for _i in 0..how_many_tags {
        if !bytes.is_empty() {
            tag_ids.push(bytes.remove(0));
        } else {
            eprintln!("This should not happen!");
        }
    }
    let header = if bytes.is_empty() {
        String::new()
    } else {
        String::from_utf8(bytes)
            .unwrap()
            .lines()
            .next()
            .unwrap()
            .trim()
            .to_string()
    };
    (tag_ids, header)
}
