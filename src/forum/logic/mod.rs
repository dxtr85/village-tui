use crate::catalog::logic::SwarmShell;
mod message;
use crate::catalog::tui::CreatorResult;
use crate::catalog::tui::EditorResult;
use crate::common::poledit::decompose;
use crate::common::poledit::PolAction;
use crate::common::poledit::ReqTree;
use crate::forum::tui::EditorParams;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use async_std::task::sleep;
use async_std::task::spawn;
use async_std::task::spawn_blocking;
use async_std::task::yield_now;
use dapp_lib::prelude::ByteSet;
use dapp_lib::prelude::CapabiliTree;
use dapp_lib::prelude::Capabilities;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::Hit;
use dapp_lib::prelude::Manifest;
use dapp_lib::prelude::Policy;
use dapp_lib::prelude::Requirement;
use dapp_lib::prelude::SwarmID;
use dapp_lib::prelude::SwarmName;
use dapp_lib::prelude::SyncMessageType;
use dapp_lib::prelude::Tag;
use dapp_lib::Data;
use dapp_lib::ToApp;
use dapp_lib::ToAppMgr;
use message::ForumSyncMessage;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

use dapp_lib::prelude::AppType;
#[derive(Debug, Clone)]
pub struct TopicContext {
    pub t_id: Option<ContentID>,
    pub description: String,
    pub tags: Vec<usize>,
    pub tag_names: Vec<String>,
}
impl TopicContext {
    pub fn new(tag_names: Vec<String>) -> Self {
        TopicContext {
            t_id: None,
            description: "Topic description".to_string(),
            tags: vec![],
            tag_names,
        }
    }
}

#[derive(Debug)]
enum PresentationState {
    MainLobby(Option<u16>),
    Topic(u16, Option<u16>),
    ShowingPost(u16, u16),
    Settings,
    RunningPolicies(Option<(u16, HashMap<u16, (Policy, Requirement)>)>),
    RunningCapabilities(Option<(u16, HashMap<u16, (Capabilities, Vec<GnomeId>)>)>),
    ByteSets(bool, Option<(u8, HashMap<u8, ByteSet>)>),
    StoredPolicies(Option<u16>, HashMap<u8, Policy>),
    StoredCapabilities(Option<u16>),
    // StoredByteSets(Option<u16>),
    SelectingOnePolicy(Vec<Policy>, ReqTree),
    SelectingOneRequirement(Vec<Requirement>, Policy, ReqTree),
    Pyramid(Policy, ReqTree),
    Capability(Capabilities, Vec<GnomeId>),
    SelectingOneCapability(Vec<Capabilities>, Box<PresentationState>),
    Editing(Option<u16>, Box<PresentationState>),
    TopicEditing(TopicContext),
    HeapSorting(Option<(ForumSyncMessage, GnomeId)>, Option<Entry>),
    CreatingByteSet(Vec<u16>, bool, Option<(u8, HashMap<u8, ByteSet>)>),
    Fitlering,
}

// impl PresentationState {
//     pub fn is_main_menu(&self) -> bool {
//         if let PresentationState::MainLobby(_pg_o) = self {
//             true
//         } else {
//             false
//         }
//     }
//     pub fn is_topic(&self) -> bool {
//         if let PresentationState::Topic(_c_id, _pg_o) = self {
//             true
//         } else {
//             false
//         }
//     }
//     pub fn is_showing_post(&self, c_id: &u16, d_id: &u16) -> bool {
//         if let PresentationState::ShowingPost(c, d) = self {
//             if c == c_id && d == d_id {
//                 true
//             } else {
//                 false
//             }
//         } else {
//             false
//         }
//     }
//     pub fn is_editing(&self) -> bool {
//         if let PresentationState::Editing(_pg_o, _prev_st) = self {
//             true
//         } else {
//             false
//         }
//     }
// }
// use crate::common::poledit::PolAction;

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    author: GnomeId,
    tags: Vec<u8>,
    text: String,
    hash: u64,
}
impl Entry {
    pub fn from_data(data: Data, is_manifest_or_nonop_post: bool) -> Result<Self, ()> {
        if data.len() < 9 {
            // eprintln!("")
            return Err(());
        }
        let hash = data.get_hash();
        let mut bytes = data.bytes();
        // TODO: first go Tags
        let tags_count = if is_manifest_or_nonop_post {
            eprintln!("man or nonop");
            0
        } else {
            bytes.remove(0)
        };
        let mut tags = vec![];
        eprintln!("tags count: {tags_count}");
        for _i in 0..tags_count {
            tags.push(bytes.remove(0));
        }
        // if tags_count == 1 || tags_count == 0 {
        //     eprintln!("Temporary solution: {tags_count}");
        //     //temporary solution, I don't want to loose my topics lol
        //     bytes.remove(0);
        //     tags.push(bytes.remove(0));
        // }
        let g_id = u64::from_be_bytes([
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
            bytes.remove(0),
        ]);
        eprintln!("Attempt to create Entry from {:?} bytes", bytes);
        let str_res = String::from_utf8(bytes);
        if let Ok(text) = str_res {
            Ok(Entry::new(GnomeId(g_id), tags, text, hash))
        } else {
            eprintln!("Could not create entry: {}", str_res.err().unwrap());
            Err(())
        }
    }
    pub fn into_data(self, is_manifest_or_nonop_post: bool) -> Result<Data, Vec<u8>> {
        let mut bytes = Vec::with_capacity(9 + self.tags.len() + self.text.len());
        if !is_manifest_or_nonop_post {
            bytes.push(self.tags.len() as u8);
            for tag in self.tags {
                bytes.push(tag);
            }
        }
        for b in self.author.bytes() {
            bytes.push(b);
        }
        bytes.append(&mut self.text.into_bytes());
        Data::new(bytes)
    }
    pub fn entry_line(&self, size: usize) -> String {
        let mut text = if let Some(line) = self.text.lines().next() {
            line.trim().chars().take(size - 21).collect::<String>()
        } else {
            "No text".to_string()
        };
        let t_len = text.chars().count();

        for _i in t_len..size - 21 {
            text.push(' ');
        }
        format!("{} {}", text, self.author)
    }

    pub fn new(author: GnomeId, tags: Vec<u8>, text: String, hash: u64) -> Self {
        Entry {
            author,
            tags,
            text,
            hash,
        }
    }
    pub fn empty() -> Self {
        Entry {
            author: GnomeId::any(),
            tags: vec![],
            text: format!("Empty"),
            hash: 0,
        }
    }
}

use crate::forum::tui::serve_forum_tui;
use crate::forum::tui::Action;
use crate::forum::tui::FromForumView;
use crate::forum::tui::ToForumView;
use crate::InternalMsg;
use crate::Toolset;
pub struct ForumLogic {
    my_id: GnomeId,
    entries_count: u16,
    entry_max_len: usize,
    shell: SwarmShell,
    // we need to store local copy of first pages of every topic in order to
    // define and apply filtering logic
    all_topics: Vec<Entry>,
    category_filter: Option<Vec<u8>>,
    text_filter: Option<String>,
    menu_pages: Vec<Vec<u16>>,
    posts: Vec<Entry>,
    last_heap_msg: Option<(ForumSyncMessage, GnomeId)>,
    presentation_state: PresentationState,
    to_app_mgr_send: ASender<ToAppMgr>,
    _to_user_send: ASender<InternalMsg>,
    to_user_recv: AReceiver<InternalMsg>,
    to_tui_send: Sender<ToForumView>,
    to_tui_recv: Option<Receiver<ToForumView>>,
    from_tui_send: Option<Sender<FromForumView>>,
    clipboard: Option<(SwarmName, ContentID)>,
}
impl ForumLogic {
    pub fn new(
        my_id: GnomeId,
        screen_size: (usize, usize),
        swarm_name: SwarmName,
        to_app_mgr_send: ASender<ToAppMgr>,
        to_user_send: ASender<InternalMsg>,
        to_user_recv: AReceiver<InternalMsg>,
    ) -> Self {
        let (to_tui_send, to_tui_recv) = channel();
        let (from_tui_send, from_tui_recv) = channel();
        spawn(from_forum_tui_adapter(
            from_tui_recv,
            to_user_send.clone(),
            // wrapped_sender.clone(),
        ));
        let shell = SwarmShell::new(dapp_lib::prelude::SwarmID(0), swarm_name, AppType::Forum);
        ForumLogic {
            presentation_state: PresentationState::MainLobby(None),
            my_id,
            entries_count: ((screen_size.1 - 3) >> 1) as u16,
            entry_max_len: usize::max(60, screen_size.0 - 4),
            shell,
            posts: vec![],
            all_topics: vec![],
            menu_pages: vec![vec![]],
            category_filter: None,
            text_filter: None,
            last_heap_msg: None,
            to_app_mgr_send,
            _to_user_send: to_user_send,
            to_user_recv,
            to_tui_send,
            to_tui_recv: Some(to_tui_recv),
            from_tui_send: Some(from_tui_send),
            clipboard: None,
        }
    }
    pub async fn run(
        mut self,
        founder: GnomeId,
        _config_dir: PathBuf,
        toolset: Toolset,
        clipboard_opt: Option<(SwarmName, ContentID)>,
        // mut config: Configuration,
        // mut tui_mgr: Manager,
        // ) -> Option<(AppType, AReceiver<InternalMsg>, Configuration, Manager)> {
    ) -> Option<(
        Option<AppType>,
        SwarmName,
        AReceiver<InternalMsg>,
        Toolset,
        Option<(SwarmName, ContentID)>,
    )> {
        self.clipboard = clipboard_opt;
        let from_presentation_msg_send = self.from_tui_send.take().unwrap();
        let to_presentation_msg_recv = self.to_tui_recv.take().unwrap();
        let tui_join = spawn_blocking(move || {
            serve_forum_tui(
                founder,
                toolset,
                // tui_mgr,
                from_presentation_msg_send,
                to_presentation_msg_recv,
                // config,
            )
        });
        let mut switch_to_opt = None;
        let _ = self
            .to_app_mgr_send
            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::SetActiveApp(
                self.shell.swarm_name.clone(),
            )))
            .await;

        loop {
            let int_msg_res = self.to_user_recv.recv().await;
            if int_msg_res.is_err() {
                eprintln!("Forum error recv internal: {}", int_msg_res.err().unwrap());
                break;
            }
            let msg = int_msg_res.unwrap();
            match msg {
                InternalMsg::User(to_app) => self.process_to_app_msg(to_app).await,

                InternalMsg::Forum(from_tui) => {
                    if self
                        .process_from_tui_msg(from_tui, &mut switch_to_opt)
                        .await
                    {
                        break;
                    }
                }
                _other => {
                    eprintln!("Forum unexpected InternalMsg");
                }
            }
        }

        eprintln!("ForumLogic is done");
        let toolset = tui_join.await;

        eprintln!("Forum is all done.");
        if let Some((switch_app, s_name)) = switch_to_opt {
            let _ = self
                .to_app_mgr_send
                .send(ToAppMgr::FromApp(dapp_lib::LibRequest::SetActiveApp(
                    s_name.clone(),
                )))
                .await;
            Some((
                Some(switch_app),
                s_name,
                self.to_user_recv,
                toolset,
                self.clipboard,
            ))
        } else {
            let _ = self.to_app_mgr_send.send(ToAppMgr::Quit).await;
            toolset.discard();
            None
        }
    }
    async fn process_to_app_msg(&mut self, to_app: ToApp) {
        match to_app {
            ToApp::ActiveSwarm(s_name, s_id) => {
                eprintln!("Forum: ToApp::ActiveSwarm({s_id},{s_name})");
                if s_name == self.shell.swarm_name {
                    self.shell.swarm_id = s_id;

                    eprintln!(
                        "Forum request FirstPages up to incl: {}",
                        self.entries_count - 1
                    );
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                            s_id, None, // Some((0, self.entries_count - 1)),
                        )))
                        .await;
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadAllPages(
                            s_id, 0,
                        )))
                        .await;
                } else {
                    // TODO: wait a bit and try again
                    // How do we do that?
                    // The simplest way is to spawn
                    // an async task that will sleep for
                    // specified time period and then send
                    // provided message.
                    //
                    // We could extend InternalMessage
                    // but that just complicates things

                    let message = ToAppMgr::FromApp(dapp_lib::LibRequest::SetActiveApp(
                        self.shell.swarm_name.clone(),
                    ));
                    spawn(start_a_timer(
                        self.to_app_mgr_send.clone(),
                        message,
                        Duration::from_secs(3),
                    ));
                    eprintln!("Try again in 3 sec…)");
                }
            }
            ToApp::ReadSuccess(_s_id, s_name, c_id, d_type, start_page, d_vec) => {
                if s_name == self.shell.swarm_name {
                    if c_id == 0 && start_page == 0 {
                        self.process_manifest(d_type, d_vec);
                    } else {
                        self.process_content(c_id, d_type, start_page, d_vec).await;
                    }
                    self.present().await;
                } else {
                    eprintln!("Received Content from other Swarm");
                }
            }
            // ToApp::FirstPages(s_id, fp_vec) => {
            //     if s_id == self.shell.swarm_id {
            //         //TODO
            //         self.process_first_pages(fp_vec).await;
            //         self.present_topics().await;
            //         eprintln!("Got first pages");
            //     }
            // }
            ToApp::ContentChanged(s_id, c_id, d_type, first_page_opt) => {
                if s_id == self.shell.swarm_id {
                    eprintln!("ContentChanged for {c_id}");
                    if let Some(first_page) = first_page_opt {
                        if c_id == 0 {
                            // TODO: this will crash once
                            // we have tags
                            self.process_manifest(d_type, vec![first_page]);
                        } else {
                            self.process_content(c_id, d_type, 0, vec![first_page])
                                .await
                        }
                        self.present().await;
                    } else {
                        eprintln!("But no first page!");
                        if c_id == 0 {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadAllPages(
                                    self.shell.swarm_id,
                                    0,
                                )))
                                .await;
                        }
                        // TODO: check if we should read given contents
                        // (that is only when we are currently presenting given Topic)
                        match &self.presentation_state {
                            PresentationState::Topic(t_id, pg_opt) => {
                                if *t_id == c_id {
                                    if let Some(pg) = pg_opt {
                                        // TODO: if so, send ReadRequest to AppData asking for a set of Pages
                                        // that are being currently displayed
                                        let _ = self
                                            .to_app_mgr_send
                                            .send(ToAppMgr::FromApp(
                                                dapp_lib::LibRequest::ReadPagesRange(
                                                    s_id,
                                                    c_id,
                                                    (*pg) * self.entries_count,
                                                    (*pg) * self.entries_count + self.entries_count
                                                        - 1,
                                                ),
                                            ))
                                            .await;
                                    }
                                }
                            }
                            _other => {
                                //nah, we're fine
                            }
                        }
                    }
                } else {
                    eprintln!("ContentChanged for {s_id}, my_id:{}", self.shell.swarm_id);
                }
            }
            ToApp::RunningPolicies(mut policies) => {
                eprintln!("#{} RunningPolicies in forum logic", policies.len());
                let pol_page = 0;

                // TODO: store how many entries there are in menu.
                let mut pols = Vec::with_capacity(self.entries_count as usize);
                let mut mapping = HashMap::with_capacity(policies.len());
                // let entries_len = 10;
                for i in 0..policies.len() as u16 {
                    let pol = policies.remove(0);
                    if i < self.entries_count {
                        pols.push((i, pol.0.text()));
                    }
                    mapping.insert(i, pol);
                }
                self.presentation_state =
                    PresentationState::RunningPolicies(Some((pol_page, mapping)));
                let _ = self
                    .to_tui_send
                    .send(ToForumView::RunningPoliciesPage(pol_page, pols));
            }
            ToApp::RunningCapabilities(mut caps) => {
                eprintln!("#{} RunningCapabilities in forum logic", caps.len());
                let cs_page = 0;

                // TODO: store how many entries there are in menu.
                let mut ccaps = Vec::with_capacity(self.entries_count as usize);
                let mut mapping = HashMap::with_capacity(caps.len());
                // let entries_len = 10;
                for i in 0..caps.len() as u16 {
                    let (cap, gid_list) = caps.remove(0);
                    if i < self.entries_count {
                        ccaps.push((i, cap.text()));
                    }
                    mapping.insert(i, (cap, gid_list));
                }
                self.presentation_state =
                    PresentationState::RunningCapabilities(Some((cs_page, mapping)));
                let _ = self
                    .to_tui_send
                    .send(ToForumView::RunningCapabilitiesPage(cs_page, ccaps));
            }
            ToApp::RunningByteSets(mut bsets) => {
                //TODO

                eprintln!("#{} RunningByteSets in forum logic", bsets.len());
                let bs_page = 0;

                // TODO: store how many entries there are in menu.
                let mut bbsets = Vec::with_capacity(self.entries_count as usize);
                let mut mapping = HashMap::with_capacity(bsets.len());
                // let entries_len = 10;
                while !bsets.is_empty() {
                    let (bs_id, bs_list) = bsets.remove(0);
                    if bbsets.len() < self.entries_count as usize {
                        if bs_list.is_none() {
                            bbsets.push((bs_id as u16, format!("Empty({bs_id})")));
                        } else if bs_list.is_pair() {
                            bbsets.push((bs_id as u16, format!("Pairs({bs_id})")));
                        } else {
                            bbsets.push((bs_id as u16, format!("Bytes({bs_id})")));
                        }
                    }
                    mapping.insert(bs_id, bs_list);
                }
                let running = true;
                self.presentation_state =
                    PresentationState::ByteSets(running, Some((bs_page, mapping)));
                let _ = self
                    .to_tui_send
                    .send(ToForumView::RunningByteSetsPage(bs_page.into(), bbsets));
            }
            ToApp::FirstPages(s_id, pg_vec) => {
                if s_id == self.shell.swarm_id {
                    self.process_first_pages(pg_vec).await;
                    self.present().await;
                } else {
                    eprintln!("FirstPages of other swarm");
                }
            }
            ToApp::NewContent(s_id, c_id, d_type, data) => {
                //TODO
                eprintln!("We need to process new content!");
                if s_id == self.shell.swarm_id {
                    self.append_first_page(c_id, d_type, data).await;
                    self.present().await;
                }
            }
            ToApp::HeapData(s_id, app_msg, signed_by) => {
                if s_id == self.shell.swarm_id {
                    eprintln!("Forum recv Heap Data {}", app_msg.m_type());
                    let forum_msg = ForumSyncMessage::parse_app_msg(app_msg).unwrap();
                    if let Some((prev_msg, prev_sign)) =
                        std::mem::replace(&mut self.last_heap_msg, None)
                    {
                        if prev_msg == forum_msg && signed_by == prev_sign {
                            eprintln!("Same heap as before…");
                            self.heap_logic(false).await;
                        }
                    } else {
                        self.last_heap_msg = Some((forum_msg, signed_by));
                        self.heap_logic(false).await;
                    }
                } else {
                    eprintln!("Heap data of {} ignored", s_id);
                }
            }
            ToApp::HeapEmpty(s_id) => {
                if s_id == self.shell.swarm_id {
                    eprintln!("Forum recv HeapEmpty");
                    self.last_heap_msg = None;
                    self.heap_logic(true).await;
                }
            }
            ToApp::PolicyNotMet(s_id, sm_type, data) => {
                self.sync_request_rejected(s_id, sm_type, data).await;
            }
            ToApp::SearchResults(query, hits) => {
                self.process_search_results(query, hits).await;
            }
            _other => {
                eprintln!("InternalMsg::User {:?}", _other);
            }
        }
    }
    async fn process_from_tui_msg(
        &mut self,
        from_tui: FromForumView,
        switch_to_opt: &mut Option<(AppType, SwarmName)>,
    ) -> bool {
        match from_tui {
            FromForumView::Act(action) => {
                match action {
                    Action::Settings => {
                        self.presentation_state = PresentationState::Settings;
                    }
                    Action::RunningPolicies => {
                        self.presentation_state = PresentationState::RunningPolicies(None);
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::RunningPolicies(
                                self.shell.swarm_name.clone(),
                            )))
                            .await;
                    }
                    Action::StoredPolicies => {
                        // Send policies from Manifest
                        let p_keys = self.shell.manifest.policy_reg.keys();

                        let pol_page = 0;
                        let mut pols = Vec::with_capacity(p_keys.len());
                        let mut p_map = HashMap::with_capacity(p_keys.len());
                        for (i, key) in p_keys.enumerate() {
                            pols.push((i as u16, key.text()));
                            p_map.insert(i as u8, *key);
                        }
                        self.presentation_state = PresentationState::StoredPolicies(Some(0), p_map);
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::StoredPoliciesPage(pol_page, pols));
                    }
                    Action::RunningCapabilities => {
                        //TODO:
                        self.presentation_state = PresentationState::RunningCapabilities(None);
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::FromApp(
                                dapp_lib::LibRequest::RunningCapabilities(
                                    self.shell.swarm_name.clone(),
                                ),
                            ))
                            .await;
                    }
                    Action::StoredCapabilities => {
                        //TODO:
                        let c_keys = self.shell.manifest.capability_reg.keys();
                        let mut c_listing = Vec::with_capacity(c_keys.len());
                        for key in c_keys {
                            c_listing.push((key.byte() as u16, key.text()));
                        }
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::StoredCapabilitiesPage(0, c_listing));
                        self.presentation_state = PresentationState::StoredCapabilities(Some(0));
                    }
                    Action::ByteSets(_is_run) => {
                        //TODO:
                        if _is_run {
                            self.presentation_state = PresentationState::ByteSets(_is_run, None);
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::FromApp(dapp_lib::LibRequest::RunningByteSets(
                                    self.shell.swarm_name.clone(),
                                )))
                                .await;
                        } else {
                            eprintln!("Stored BS collecting");
                            let bss = &self.shell.manifest.byteset_reg;
                            let mut id_str_vec = Vec::with_capacity(bss.len());
                            let mut hm = HashMap::with_capacity(bss.len());
                            for id in 0..=255 {
                                if let Some(bs) = bss.get(&id) {
                                    let label = if bs.is_none() {
                                        format!("Empty({})", id)
                                    } else if bs.is_pair() {
                                        format!("Pairs({})", id)
                                    } else {
                                        format!("Bytes({})", id)
                                    };
                                    id_str_vec.push((id as u16, label));
                                    hm.insert(id, bs.clone());
                                } else {
                                    break;
                                }
                            }
                            let _ = self
                                .to_tui_send
                                .send(ToForumView::StoredByteSetsPage(0, id_str_vec));
                            self.presentation_state =
                                PresentationState::ByteSets(_is_run, Some((0, hm)));
                        }
                    }
                    // Action::StoredByteSets => {
                    //     //TODO:
                    //     self.presentation_state =
                    //         PresentationState::ByteSets(false, None);
                    // }
                    Action::AddNew(param) => {
                        self.add_new_action(param).await;
                    }
                    Action::Edit(id) => {
                        let curr_state = std::mem::replace(
                            &mut self.presentation_state,
                            PresentationState::Settings,
                        );
                        match curr_state {
                            PresentationState::Topic(c_id, pg_opt) => {
                                eprintln!("Action Edit when in Topic");
                                if let Some(page) = pg_opt {
                                    let post_id = (page * self.entries_count) + id;
                                    // TODO: read contents of post,
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::FromApp(
                                            dapp_lib::LibRequest::ReadPagesRange(
                                                self.shell.swarm_id,
                                                c_id,
                                                post_id,
                                                post_id,
                                            ),
                                        ))
                                        .await;
                                    self.presentation_state =
                                        PresentationState::Editing(None, Box::new(curr_state));
                                    // present them to user in writable editor
                                    // once finished editing send regular ChangeData msg (for now)
                                } else {
                                    eprintln!("Can not edit, unknown Page.");
                                    self.presentation_state =
                                        PresentationState::Topic(c_id, pg_opt);
                                }
                            }
                            PresentationState::Settings => {
                                if id == 0 {
                                    // TODO: Edit in this State means User opened Requests Menu
                                    // So we need to HeepPeek and, then also source page read
                                    // and after that send those two entries to User to decide.
                                    self.heap_logic(false).await;
                                } else if id == 1 {
                                    //TODO:
                                    self.add_category().await;
                                }
                            }
                            PresentationState::HeapSorting(fsm_opt, entry_opt) => {
                                if let Some((fsm, signer)) = fsm_opt {
                                    match fsm {
                                        ForumSyncMessage::EditPost(t_id, p_id, entry) => {
                                            if let Some(curr_e) = entry_opt {
                                                if curr_e.author == signer {
                                                    let _ = self
                                                        .to_app_mgr_send
                                                        .send(ToAppMgr::UpdateData(
                                                            self.shell.swarm_id,
                                                            t_id,
                                                            p_id,
                                                            entry.into_data(p_id > 0).unwrap(),
                                                        ))
                                                        .await;
                                                } else {
                                                    eprintln!(
                                                        "Can not update, Author: {} but edited by: {}",
                                                        curr_e.author, signer
                                                    );
                                                }
                                            } else {
                                                eprintln!(
                                                    "Can not update, unable to validate Author",
                                                );
                                            }
                                        }
                                        ForumSyncMessage::AddTopic(_t_id, entry) => {
                                            let _ = self
                                                .to_app_mgr_send
                                                .send(ToAppMgr::AppendContent(
                                                    self.shell.swarm_id,
                                                    DataType::Data(0),
                                                    entry.into_data(false).unwrap(),
                                                ))
                                                .await;
                                        }
                                        ForumSyncMessage::AddPost(t_id, entry) => {
                                            let _ = self
                                                .to_app_mgr_send
                                                .send(ToAppMgr::AppendData(
                                                    self.shell.swarm_id,
                                                    t_id,
                                                    entry.into_data(true).unwrap(),
                                                ))
                                                .await;
                                        }
                                    }
                                }
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::PopHeap(
                                        self.shell.swarm_id,
                                    )))
                                    .await;
                            }
                            _other => {
                                eprintln!("Edit not supported in {:?}", _other);
                            }
                        }
                        // Here we directly send a SyncMessage::UserDefined for testing
                        // let _ = self
                        //     .to_app_mgr_send
                        //     .send(ToAppMgr::UserDefined(
                        //         self.shell.swarm_id,
                        //         21,
                        //         17,
                        //         111,
                        //         Data::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).unwrap(),
                        //     ))
                        //     .await;
                        // eprintln!("Posted UserDefined request");
                    }
                    Action::Delete(id) => {
                        self.delete_action(id).await;
                    }
                    Action::Run(id_opt) => {
                        self.run_action(id_opt).await;
                    }
                    Action::Store(id) => {
                        self.store_action(id).await;
                    }
                    Action::NextPage => {
                        eprintln!("Action::NextPage ");
                        self.show_next_page().await;
                        self.present().await;
                    }
                    Action::PreviousPage => {
                        self.show_previous_page().await;
                        self.present().await;
                    }
                    Action::FirstPage => {
                        self.show_first_page().await;
                        self.present().await;
                    }
                    Action::LastPage => {
                        self.show_last_page().await;
                        self.present().await;
                    }
                    Action::Filter(is_cat_filter) => {
                        // TODO: define local filtering logic for current swarm only
                        // Once a filter is defined we should gather all first content pages
                        // and store them locally.
                        // Then we apply filters on those pages and present only filtered contents
                        self.presentation_state = PresentationState::Fitlering;
                        if is_cat_filter {
                            // open category selector
                            let _ = self.to_tui_send.send(ToForumView::Select(
                                true,
                                self.shell.manifest.tag_names(None),
                                vec![],
                            ));
                            // then ask Datastore for first pages of CIDs with Tags
                        } else {
                            // open Editor and pass results to local filtering logic

                            let e_p = EditorParams {
                                title: format!("Define Search text"),
                                initial_text: None,
                                allow_newlines: true,
                                chars_limit: None,
                                text_limit: None,
                                read_only: false,
                            };
                            let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                        }
                    }
                    Action::Query(qt) => {
                        self.serve_query(qt).await;
                    }
                    Action::MainMenu => {
                        //TODO
                        eprintln!("Action::MainMenu presenting topics");
                        self.presentation_state = PresentationState::MainLobby(Some(0));
                        self.filter_topics().await;
                        // self.entries = vec![];
                        // let _ = self
                        //     .to_app_mgr_send
                        //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                        //         self.shell.swarm_id,
                        //         Some((1, self.entries_count - 1)),
                        //     )))
                        //     .await;
                        // let m_head: Entry = Entry::new(
                        //     self.shell.swarm_name.founder,
                        //     vec![],
                        //     self.shell
                        //         .manifest
                        //         .description
                        //         .lines()
                        //         .take(1)
                        //         // .trimmed()
                        //         .collect(),
                        //     0,
                        // );
                        // self.update_topic(0, m_head);
                        self.present().await;
                    }
                    Action::Posts(_topic_id) => {
                        //TODO
                    }
                    Action::PolicyAction(p_act) => {
                        self.serve_pol_action(p_act).await;
                    }
                    Action::Selected(id_vec) => {
                        self.serve_selected(id_vec).await;
                    }
                    Action::EditorResult(str_o) => self.process_edit_result(str_o).await,
                    Action::CreatorResult(c_res) => self.process_creator_result(c_res).await,
                    Action::FollowLink(s_name, _c_id, _pg_id) => {
                        // Follow Link
                        if s_name.founder.is_any() {
                            eprintln!("Should go back to Village");
                            *switch_to_opt = Some((
                                AppType::Catalog,
                                SwarmName::new(self.my_id, s_name.name).unwrap(),
                            ));
                        } else {
                            // eprintln!("Should follow link");
                            *switch_to_opt = Some((AppType::Catalog, s_name));
                        }
                        let _ = self.to_tui_send.send(ToForumView::Finish);
                        return true;
                    }
                }
            }
            // FromForumView::RunningPolicies => {
            //     eprintln!("Should send running policies");
            // }
            // FromForumView::StoredPolicies => {
            //     eprintln!("Should send stored policies");
            // }
            FromForumView::Quit => {
                return true;
            }
            FromForumView::CopyToClipboard(which_one) => {
                self.set_clipboard(which_one);
            }
            FromForumView::SwitchTo(app_type, s_name) => {
                *switch_to_opt = Some((app_type, s_name));
                return true;
            }
        }
        false
    }
    async fn heap_logic(&mut self, heap_empty: bool) {
        // Remember to always set self.presentation_state (or rework logic)
        self.presentation_state = PresentationState::HeapSorting(None, None);
        if heap_empty {
            // TODO: send ToPresentation to inform User that heap is empty
            let _ = self.to_tui_send.send(ToForumView::Request(vec![(
                0,
                format!("No Requests pending"),
            )]));
            return;
        }
        // let last_msg = std::mem::replace(&mut self.last_heap_msg, None);
        if let Some((app_msg, signed_by)) = &self.last_heap_msg {
            self.presentation_state =
                PresentationState::HeapSorting(Some((app_msg.clone(), *signed_by)), None);
            match app_msg {
                ForumSyncMessage::AddTopic(_t_id, entry) => {
                    let _ = self.to_tui_send.send(ToForumView::Request(vec![(
                        0,
                        format!("Add Topic: {}", entry.entry_line(self.entry_max_len - 11)),
                    )]));
                }
                ForumSyncMessage::AddPost(t_id, entry) => {
                    let _ = self.to_tui_send.send(ToForumView::Request(vec![(
                        0,
                        format!(
                            "{} Add Post: {}",
                            t_id,
                            entry.entry_line(self.entry_max_len - 15)
                        ),
                    )]));
                }
                ForumSyncMessage::EditPost(t_id, p_id, entry) => {
                    //TODO: do not send until we have both entries
                    let _ = self.to_tui_send.send(ToForumView::Request(vec![
                        (0, format!("Orig: Pending ({}-{})", t_id, p_id)),
                        (
                            1,
                            format!("Modif: {}", entry.entry_line(self.entry_max_len - 7)),
                        ),
                    ]));
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            *t_id,
                            *p_id,
                            *p_id,
                        )))
                        .await;
                }
            }
            // TODO: depending on ForumMsg ask for more Data
            //       or act immediately by sending ToPresentation
            return;
            // }
        }
        let _ = self
            .to_app_mgr_send
            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::PeekHeap(
                self.shell.swarm_id,
            )))
            .await;
    }
    async fn add_category(&mut self) {
        // Remember to always set self.presentation_state (or rework logic)
        self.presentation_state =
            PresentationState::Editing(Some(1), Box::new(PresentationState::Settings));

        let e_p = EditorParams {
            title: format!("Add new Category (oneline, 32 bytes max)"),
            initial_text: Some(format!("NewCategory")),
            allow_newlines: false,
            chars_limit: None,
            text_limit: Some(32),
            read_only: false,
        };
        let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
    }

    async fn append_first_page(&mut self, c_id: ContentID, d_type: DataType, data: Data) {
        self.process_content(c_id, d_type, 0, vec![data]).await;
    }
    async fn process_first_pages(&mut self, pg_vec: Vec<(ContentID, DataType, Data)>) {
        // First pages start from CID==1
        if pg_vec.is_empty() {
            // eprintln!("process_first_pages EMPTY");
            return;
        }
        eprintln!("process_first_pages");
        for (c_id, d_type, data) in pg_vec {
            self.process_content(c_id, d_type, 0, vec![data]).await;
        }
    }

    async fn process_content(
        &mut self,
        c_id: ContentID,
        d_type: DataType,
        start_page: u16,
        d_vec: Vec<Data>,
    ) {
        if d_vec.is_empty() {
            return;
        }
        eprintln!(
            "Process CID{c_id} from:{start_page} len: {}",
            d_vec[0].len()
        );
        if d_vec[0].len() < 9 {
            return;
        }
        // This check is made elsewhere
        // if c_id == 0 {
        //     return self.process_manifest(d_type, d_vec);
        // }
        //TODO: We need to store entire Manifest,
        // and all first page headers
        // in order to present and filter Topics to be
        // displayed.
        // Once we select a Topic we should retrieve it's
        // Data - first 64 pages should be fine.
        // If user moves out of that range of 64 pages
        // then we read more data, we can discard previous
        // data as it is dapp-lib's responsibility to
        // provide up to date pages and do it fast.
        match d_type {
            DataType::Data(id) => {
                if id == 0 {
                    // regular Topic
                    let curr_state = std::mem::replace(
                        &mut self.presentation_state,
                        PresentationState::Settings,
                    );

                    let mut entry_line_changed = false;
                    if start_page == 0 {
                        let first = d_vec[0].clone();
                        entry_line_changed = self.update_topic(
                            c_id as usize,
                            Entry::from_data(first, c_id == 0).unwrap(),
                        );
                        if self.category_filter.is_none() && self.text_filter.is_none() {
                            self.extend_main_pages_until(c_id);
                        }
                    }
                    match curr_state {
                        PresentationState::MainLobby(_pg_opt) => {
                            self.presentation_state = curr_state;

                            if !entry_line_changed {
                                return;
                            }
                            // check if c_id is visible on screen,
                            if let Some(menu_page) = _pg_opt {
                                eprintln!("{menu_page} MPS len: {}", self.menu_pages.len());
                                if self.menu_pages[menu_page as usize].contains(&c_id) {
                                    // if so redraw it
                                    self.present().await;
                                }
                            }
                        }
                        PresentationState::Topic(_t_id, mut _pg_opt) => {
                            if c_id != _t_id {
                                eprintln!("Not my topic");
                                self.presentation_state = curr_state;
                                return;
                            }
                            if _pg_opt.is_none() {
                                eprintln!("Topic page unknown");
                                // TODO: we need to also include filtering logic into the mix
                                _pg_opt = Some(start_page / self.entries_count);
                                // return;
                            }
                            // let pg = _pg_opt.unwrap();
                            // let first_entry_id = pg * self.entries_count;
                            // if start_page != first_entry_id {
                            //     eprintln!("Pages mismatch");
                            //     self.presentation_state = curr_state;
                            //     return;
                            // }
                            self.presentation_state = PresentationState::Topic(_t_id, _pg_opt);

                            for (id, first) in d_vec.into_iter().enumerate() {
                                eprintln!("Updating post {} {}", id, first);

                                self.update_post(
                                    id + (start_page as usize),
                                    Entry::from_data(first, (id as u16) + start_page > 0).unwrap(),
                                );
                            }
                            self.presentation_state = curr_state;
                        }
                        PresentationState::ShowingPost(t_id, p_id) => {
                            if c_id == t_id {
                                if start_page == p_id || p_id == u16::MAX {
                                    let initial_text =
                                // Some(String::from_utf8(d_vec[0].clone().bytes()).unwrap());
                                Some(Entry::from_data(d_vec[0].clone(),p_id>0).unwrap().text);
                                    let e_p = EditorParams {
                                        title: format!("Topic #{t_id}, Post #{start_page}"),
                                        initial_text,
                                        allow_newlines: true,
                                        chars_limit: None,
                                        text_limit: None,
                                        read_only: true,
                                    };
                                    let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                                    self.presentation_state =
                                        PresentationState::ShowingPost(t_id, start_page);
                                } else {
                                    eprintln!("Got wrong page to present");
                                }
                            } else {
                                eprintln!("Got wrong Topic to present");
                            }
                        }

                        PresentationState::Editing(what_opt, prev_state) => {
                            eprintln!("ReadSuccess in Editing State");
                            // TODO: check if what we got matches what we want!

                            let new_what;
                            if what_opt.is_none() {
                                new_what = Some(start_page);
                                let initial_text =
                                    // Some(String::from_utf8(d_vec[0].clone().bytes()).unwrap());
                                    Some(Entry::from_data(d_vec[0].clone(),start_page>0).unwrap().text);
                                let e_p = EditorParams {
                                    title: format!("Editing Post #{start_page}"),
                                    initial_text,
                                    allow_newlines: true,
                                    chars_limit: None,
                                    text_limit: Some(1000),
                                    read_only: false,
                                };
                                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                            } else {
                                new_what = None;
                            }
                            eprintln!("NewWhat: {new_what:?}");
                            self.presentation_state =
                                PresentationState::Editing(new_what, prev_state);
                        }
                        PresentationState::HeapSorting(opt_fsm_sign, entry_opt) => {
                            if let Some((forum_msg, _sign)) = opt_fsm_sign {
                                //TODO
                                match &forum_msg {
                                    ForumSyncMessage::EditPost(t_id, p_id, new_entry) => {
                                        if *t_id == c_id && *p_id == start_page {
                                            let orig_entry =
                                                Entry::from_data(d_vec[0].clone(), *p_id > 0)
                                                    .unwrap();
                                            let _ =
                                                self.to_tui_send.send(ToForumView::Request(vec![
                                                    (0, orig_entry.entry_line(self.entry_max_len)),
                                                    (1, new_entry.entry_line(self.entry_max_len)),
                                                ]));
                                        } else {
                                            eprintln!("Received wrong data for comparison");
                                            eprintln!(
                                                "Expected: {}-{} got: {}-{}",
                                                t_id, p_id, c_id, start_page
                                            );
                                        }
                                    }
                                    ForumSyncMessage::AddTopic(_t_id, _entry) => {
                                        eprintln!("Unexpected Read Success, when adding new topic");
                                    }
                                    ForumSyncMessage::AddPost(_t_id, _entry) => {
                                        eprintln!("Unexpected Read Success, when adding a post");
                                    }
                                }
                                let entry =
                                    Entry::from_data(d_vec[0].clone(), start_page > 0).unwrap();
                                self.presentation_state = PresentationState::HeapSorting(
                                    Some((forum_msg, _sign)),
                                    Some(entry),
                                );
                            } else {
                                eprintln!("Unexpected Read Success, when HeapSorting empty");
                                self.presentation_state =
                                    PresentationState::HeapSorting(None, entry_opt);
                            }
                        }
                        PresentationState::TopicEditing(t_ctx) => {
                            if t_ctx.t_id.is_some() && start_page == 0 {
                                let t_id = t_ctx.t_id.unwrap();
                                let initial_text =
                                //     if t_id ==0{
                                //     Manifest::from(d_vec).description;
                                // }else{
                                    Some(Entry::from_data(d_vec[0].clone(), t_id==0).unwrap().text);
                                // }
                                let text_limit = if t_id == 0 { Some(1000) } else { Some(800) };
                                let e_p = EditorParams {
                                    title: format!("Edit Description for CID{}", t_id),
                                    initial_text,

                                    allow_newlines: true,
                                    chars_limit: None,
                                    text_limit,
                                    read_only: false,
                                };
                                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                                self.presentation_state = PresentationState::TopicEditing(t_ctx);
                            }
                        }
                        PresentationState::Fitlering => {
                            // TODO: define local filtering logic for current swarm only
                            eprintln!("TODO: Got some pages when Filtering");
                            self.presentation_state = PresentationState::Fitlering;
                        }
                        other => {
                            eprintln!("Unexpected ReadSuccess when in state: {:?}", other);
                            self.presentation_state = other;
                        }
                    }
                } else {
                    eprintln!("Unsupportetd Forum DType: {id}");
                }
            }
            DataType::Link => {
                //TODO: link
            }
        }
    }

    fn update_topic(&mut self, c_id: usize, header: Entry) -> bool {
        let new_line = header.entry_line(self.entry_max_len);
        let t_len = self.all_topics.len();
        for _i in t_len..=c_id {
            self.all_topics.push(Entry::empty());
        }
        let old_line = self.all_topics[c_id].entry_line(self.entry_max_len);
        self.all_topics[c_id] = header;
        new_line != old_line
    }

    fn update_post(&mut self, e_id: usize, header: Entry) {
        let e_len = self.posts.len();
        for _i in e_len..=e_id {
            self.posts.push(Entry::empty());
        }
        self.posts[e_id] = header;
    }

    fn extend_main_pages_until(&mut self, t_id: u16) {
        let mpl = self.menu_pages.len() as u16;
        let mut last_page = self.menu_pages.remove(self.menu_pages.len() - 1);
        let lpl = last_page.len() as u16;
        // eprintln!("mpl: {}", self.menu_pages.len());
        let included_topics = lpl + (mpl - 1) * self.entries_count;
        if t_id < included_topics {
            self.menu_pages.push(last_page);
            return;
        }
        for nt_id in included_topics..=t_id {
            if last_page.len() as u16 == self.entries_count {
                self.menu_pages.push(last_page);
                last_page = vec![];
            }
            last_page.push(nt_id);
        }
        if !last_page.is_empty() || self.menu_pages.is_empty() {
            self.menu_pages.push(last_page);
        }
        eprintln!("Extended mpl: {}", self.menu_pages.len());
    }

    fn process_manifest(&mut self, d_type: DataType, mut d_vec: Vec<Data>) {
        eprintln!("Mfest DType: {:?}", d_type);
        if d_vec.len() == 1 {
            //TODO: only first page has changed, other pagas are same as previous
            // so we take them from our shell Manifest
            let mut e_data = self.shell.manifest.to_data();
            e_data.remove(0);
            d_vec.append(&mut e_data);
        }
        let manifest = Manifest::from(d_vec);
        let text = if manifest.description.is_empty() {
            format!("Manifest: no description")
        } else {
            let line: String = manifest.description.lines().take(1).collect();
            line.chars().take(64).collect()
        };
        let desc = Entry::new(
            self.shell.swarm_name.founder,
            vec![],
            text.trim().to_string(),
            0,
        );
        if self.posts.is_empty() {
            self.posts.push(desc);
        } else {
            self.posts[0] = desc;
        }
        self.shell.manifest = manifest;
        let entry = Entry::new(
            self.shell.swarm_name.founder,
            vec![],
            self.shell.manifest.description.clone(),
            0,
        );
        self.update_topic(0, entry);
    }
    async fn present(&mut self) {
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);
        match curr_state {
            PresentationState::MainLobby(pg_opt) => {
                // TODO: here we need to include effects of filtering logic
                let mut pg = if let Some(pg) = pg_opt { pg } else { 0 };
                if pg == u16::MAX {
                    // pg = (self.entries.len() / self.entries_count as usize) as u16;
                    pg = (self.menu_pages.len() - 1) as u16;
                }
                // let mut topics = self.read_posts(pg, self.entries_count).await;
                let mut topics = Vec::with_capacity(self.entries_count as usize);
                // if topics.is_empty() {
                //     pg = 0;
                //     topics = self.read_posts(pg, self.entries_count).await;
                // }
                for id in &self.menu_pages[pg as usize] {
                    topics.push((
                        *id,
                        self.all_topics[*id as usize].entry_line(self.entry_max_len),
                    ));
                }
                // TopicsPage(u16, Vec<(u16, String)>),
                let _ = self.to_tui_send.send(ToForumView::TopicsPage(pg, topics));
                self.presentation_state = PresentationState::MainLobby(Some(pg));
            }
            PresentationState::Topic(t_id, pg_opt) => {
                let mut pg = if let Some(pg) = pg_opt { pg } else { 0 };
                if pg == u16::MAX {
                    pg = (self.posts.len() / self.entries_count as usize) as u16;
                }
                let mut topics = self.read_posts(pg, self.entries_count).await;
                if topics.is_empty() {
                    pg = 0;
                    topics = self.read_posts(pg, self.entries_count).await;
                }
                let _ = self.to_tui_send.send(ToForumView::PostsPage(pg, topics));
                self.presentation_state = PresentationState::Topic(t_id, Some(pg));
            }
            other => {
                eprintln!("Not presenting, state is: {other:?}");
                self.presentation_state = other;
            }
        }
    }
    async fn read_posts(&self, page: u16, page_size: u16) -> Vec<(u16, String)> {
        eprintln!("read_posts page: {page}, p_size:{page_size}");
        let first_idx;
        let last_idx;
        if page == u16::MAX {
            let e_len = self.posts.len();
            let remainder = e_len % (self.entries_count as usize);
            first_idx = e_len - remainder - 1;
            last_idx = e_len - 1;
        } else {
            first_idx = (page * page_size) as usize;
            last_idx = ((page + 1) * page_size) as usize;
        }
        let p_len = self.posts.len();
        let mut res = Vec::with_capacity(page_size as usize);
        for i in first_idx..last_idx {
            if i < p_len {
                res.push((i as u16, self.posts[i].entry_line(self.entry_max_len)));
            } else {
                break;
            }
        }
        res
    }

    async fn add_new_action(&mut self, param: bool) {
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);

        // Remember to always update self.presentation_state!
        //
        match curr_state {
            PresentationState::MainLobby(_page_opt) => {
                // TODO: instead of opening Editor, open Creator
                // let e_p = EditorParams {
                //     title: format!("Adding new Topic"),
                //     initial_text: Some(format!("Topic desrciption")),

                //     allow_newlines: true,
                //     chars_limit: None,
                //     text_limit: Some(800),
                //     read_only: false,
                // };
                // let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                eprintln!("We should add a new topic");
                let tag_names = self.shell.manifest.tag_names(None);
                let _ = self
                    .to_tui_send
                    .send(ToForumView::OpenCreator(TopicContext::new(
                        tag_names.clone(),
                    )));
                self.presentation_state =
                    PresentationState::TopicEditing(TopicContext::new(tag_names));
                // self.presentation_state = PresentationState::MainLobby(page_opt);
                // self.presentation_state = PresentationState::Editing(
                //     None,
                //     Box::new(PresentationState::MainLobby(page_opt)),
                // );
            }
            PresentationState::Topic(c_id, pg_opt) => {
                let e_p = EditorParams {
                    title: format!("Adding new Post"),
                    initial_text: Some(format!("Adding new post…")),

                    allow_newlines: true,
                    chars_limit: None,
                    text_limit: Some(1016),
                    read_only: false,
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                eprintln!("We should add a new post");
                // self.presentation_state = PresentationState::MainLobby(page_opt);
                self.presentation_state = PresentationState::Editing(
                    None,
                    Box::new(PresentationState::Topic(c_id, pg_opt)),
                );
            }
            PresentationState::Capability(c, v_gid) => {
                let initial_text = if let Some((s_name, _id)) = &self.clipboard {
                    Some(format!("GID-{:016x}", s_name.founder.0))
                } else {
                    Some(format!("GID-0123456789abcdef"))
                };

                let e_p = EditorParams {
                    title: format!("Adding new Capability"),
                    initial_text,
                    allow_newlines: false,
                    chars_limit: Some("0123456789abcdef".chars().collect()),
                    text_limit: Some(20),
                    read_only: false,
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                self.presentation_state = PresentationState::Editing(
                    None,
                    Box::new(PresentationState::Capability(c, v_gid)),
                );
                yield_now().await;
            }
            PresentationState::RunningCapabilities(r_caps_opt) => {
                let mut old_caps_opt = None;
                // TODO: Open Selector with a list of Capabilities
                // that are not yet defined in running list
                // and allow to choose one
                // TODO: Once Selector returns an item present
                // this Capability to user allowing him to add GIDs.
                //

                let (avail_caps, avail_cstrs) = if let Some((_c_id, caps)) = r_caps_opt {
                    let (mut c_list, mut s_list) = Capabilities::mapping();
                    let mut iter = c_list.iter();
                    for (cap, _v_gids) in caps.values().cloned() {
                        if let Some(pos) = iter.position(|c| c == &cap) {
                            c_list.remove(pos);
                            s_list.remove(pos);
                            iter = c_list.iter();
                        }
                    }
                    old_caps_opt = Some((_c_id, caps));
                    (c_list, s_list)
                } else {
                    Capabilities::mapping()
                };
                let _ = self
                    .to_tui_send
                    .send(ToForumView::Select(true, avail_cstrs, vec![]));
                self.presentation_state = PresentationState::SelectingOneCapability(
                    avail_caps,
                    Box::new(PresentationState::RunningCapabilities(old_caps_opt)),
                );
                yield_now().await;
            }
            PresentationState::StoredCapabilities(_p_opt) => {
                // let mut old_caps_opt = None;
                // TODO: Open Selector with a list of Capabilities
                // that are not yet defined in Manifest
                // and allow to choose one
                // TODO: Once Selector returns an item present
                // this Capability to user allowing him to add GIDs.
                //

                let (avail_caps, avail_cstrs) = {
                    let (mut c_list, mut s_list) = Capabilities::mapping();
                    let mut iter = c_list.iter();
                    for cap in self.shell.manifest.capability_reg.keys() {
                        if let Some(pos) = iter.position(|c| c == cap) {
                            c_list.remove(pos);
                            s_list.remove(pos);
                            iter = c_list.iter();
                        }
                    }
                    (c_list, s_list)
                };
                let _ = self
                    .to_tui_send
                    .send(ToForumView::Select(true, avail_cstrs, vec![]));

                self.presentation_state = PresentationState::SelectingOneCapability(
                    avail_caps,
                    Box::new(PresentationState::StoredCapabilities(_p_opt)),
                );
                yield_now().await;
            }
            PresentationState::ByteSets(is_run, existing_opt) => {
                //TODO: we need to open a Selector and
                // depending on param's value populate it with
                // 0..=255 or
                // 0..=65536 (or so)
                let (options, strgs) = if param {
                    let mut opts = Vec::with_capacity(u16::MAX as usize);
                    let mut sts = Vec::with_capacity(u16::MAX as usize);
                    for i in 0..=u16::MAX {
                        opts.push(i);
                        sts.push(format!("BB{i}"));
                    }
                    (opts, sts)
                } else {
                    let mut opts = Vec::with_capacity(256);
                    let mut sts = Vec::with_capacity(256);
                    for i in 0..=255 {
                        opts.push(i);
                        sts.push(format!("B{i}"));
                    }
                    (opts, sts)
                };
                let _ = self
                    .to_tui_send
                    .send(ToForumView::Select(false, strgs, vec![]));
                self.presentation_state =
                    PresentationState::CreatingByteSet(options, is_run, existing_opt);
            }
            other => {
                eprintln!("Adding not supported yet");
                self.presentation_state = other;
            }
        }
    }
    async fn run_action(&self, id_opt: Option<usize>) {
        match &self.presentation_state {
            PresentationState::Capability(c, v_gids) => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::FromApp(
                        dapp_lib::LibRequest::SetRunningCapability(
                            self.shell.swarm_name.clone(),
                            *c,
                            v_gids.clone(),
                        ),
                    ))
                    .await;
            }
            PresentationState::ByteSets(_is_run, id_map_opt) => {
                if let Some(id) = id_opt {
                    if let Some((_i, mapping)) = id_map_opt {
                        if let Some(bset) = mapping.get(&(id as u8)) {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::FromApp(dapp_lib::LibRequest::SetRunningByteSet(
                                    self.shell.swarm_name.clone(),
                                    id as u8,
                                    bset.clone(),
                                )))
                                .await;
                        }
                    }
                }
                // eprintln!("TODO: we should run a ByteSet({})", id_opt.unwrap());
            }
            _other => {
                eprintln!("Can not run_action ");
            }
        }
        yield_now().await;
    }
    async fn store_action(&mut self, _id: usize) {
        match &self.presentation_state {
            PresentationState::Capability(cap, gnome_ids) => {
                let mut c_tree = CapabiliTree::create();
                for gnome_id in gnome_ids {
                    eprintln!("adding {} to cap", gnome_id);
                    c_tree.insert(*gnome_id);
                }
                eprintln!(
                    "After adding ctree size: {}",
                    c_tree.get_all_members().len()
                );
                self.shell.manifest.capability_reg.insert(*cap, c_tree);
                let m_data = self.shell.manifest.to_data();
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ChangeContent(
                        self.shell.swarm_id,
                        0,
                        DataType::Data(0),
                        m_data,
                    ))
                    .await;
                //TODO
            }
            PresentationState::ByteSets(_running, _pg_map_opt) => {
                if let Some((_i, bs_map)) = _pg_map_opt {
                    if let Some(bs) = bs_map.get(&(_id as u8)) {
                        self.shell
                            .manifest
                            .byteset_reg
                            .insert(_id as u8, bs.clone());
                        let m_data = self.shell.manifest.to_data();
                        eprintln!("Sending Store BS request");
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::ChangeContent(
                                self.shell.swarm_id,
                                0,
                                DataType::Data(0),
                                m_data,
                            ))
                            .await;
                    }
                } else {
                    eprintln!("Can not store, can not figure what");
                }
            }
            _other => {
                // unsupported
            }
        }
        yield_now().await;
    }

    async fn delete_action(&mut self, id: u16) {
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);
        let id = id as usize;

        // Remember to always update self.presentation_state!
        //
        match curr_state {
            PresentationState::Capability(c, mut v_gid) => {
                if id < v_gid.len() {
                    v_gid.remove(id);
                }
                self.presentation_state = PresentationState::Editing(
                    None,
                    Box::new(PresentationState::Capability(c, v_gid)),
                );

                self.process_edit_result(EditorResult::Close).await;
                yield_now().await;
            }
            PresentationState::HeapSorting(_fsm_opt, _e_opt) => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::PopHeap(
                        self.shell.swarm_id,
                    )))
                    .await;
            }
            other => {
                eprintln!("Deleting not supported yet");
                self.presentation_state = other;
            }
        }
    }
    async fn show_first_page(&mut self) {
        eprintln!("show_first_page");
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(pg_opt) => {
                eprintln!("show_first_page MainLobby {pg_opt:?}");
                // if let Some(pg) = pg_opt {
                // TODO: maybe set new pgid?
                let next_pg = 0;
                new_state = Some(PresentationState::MainLobby(Some(next_pg)));
                // let new_range_start = 0;
                // let new_range_end = self.entries_count - 1;
                // eprintln!("Asking for: {}-{}", new_range_start, new_range_end);
                // let _ = self
                //     .to_app_mgr_send
                //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                //         self.shell.swarm_id,
                //         Some((new_range_start, new_range_end)),
                //     )))
                //     .await;
                // }
                // self.present().await;
            }
            PresentationState::Topic(t_id, pg_opt) => {
                eprintln!("show_first_page Topic {pg_opt:?}");
                // if let Some(pg) = pg_opt {
                // TODO: maybe set new pgid?
                let next_pg = 0;
                new_state = Some(PresentationState::Topic(*t_id, Some(next_pg)));
                let new_range_start = 0;
                let new_range_end = self.entries_count - 1;
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                        self.shell.swarm_id,
                        *t_id,
                        new_range_start,
                        new_range_end,
                    )))
                    .await;
                // }
                // self.present().await;
            }
            _other => {
                //TODO
            }
        }
        if let Some(state) = new_state {
            self.presentation_state = state;
        }
    }

    async fn show_last_page(&mut self) {
        eprintln!("show_last_page");
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(pg_opt) => {
                eprintln!("show_last_page MainLobby {pg_opt:?}");
                // if let Some(pg) = pg_opt {
                // TODO: maybe set new pgid?
                let next_pg = u16::MAX;
                new_state = Some(PresentationState::MainLobby(Some(next_pg)));
                // let new_range_start = self.entries_count;
                // let new_range_end = 0;
                // eprintln!("Asking for: {}-{}", new_range_start, new_range_end);
                // let _ = self
                //     .to_app_mgr_send
                //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                //         self.shell.swarm_id,
                //         Some((new_range_start, new_range_end)),
                //     )))
                //     .await;
                // }
                // self.present().await;
            }
            PresentationState::Topic(t_id, pg_opt) => {
                eprintln!("show_last_page Topic {pg_opt:?}");
                // if let Some(pg) = pg_opt {
                // TODO: maybe set new pgid?
                let next_pg = u16::MAX;
                new_state = Some(PresentationState::Topic(*t_id, Some(next_pg)));
                let new_range_start = self.entries_count;
                let new_range_end = 0;
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                        self.shell.swarm_id,
                        *t_id,
                        new_range_start,
                        new_range_end,
                    )))
                    .await;
                // }
                // self.present().await;
            }
            _other => {
                //TODO
            }
        }
        if let Some(state) = new_state {
            self.presentation_state = state;
        }
    }
    async fn show_next_page(&mut self) {
        eprintln!("show_next_page");
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(pg_opt) => {
                eprintln!(
                    "show_next_page MainLobby {pg_opt:?} mpl:{}",
                    self.menu_pages.len()
                );
                if let Some(pg) = pg_opt {
                    // TODO: maybe set new pgid?
                    let next_pg = {
                        let next_val = pg.saturating_add(1);
                        if self.menu_pages.len() <= next_val as usize {
                            0
                        } else {
                            next_val
                        }
                    };
                    new_state = Some(PresentationState::MainLobby(Some(next_pg)));
                    // let new_range_start = next_pg * self.entries_count;
                    // let new_range_end = (pg.saturating_add(2) * self.entries_count) - 1;
                    // eprintln!("Asking for: {}-{}", new_range_start, new_range_end);
                    // let _ = self
                    //     .to_app_mgr_send
                    //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                    //         self.shell.swarm_id,
                    //         Some((new_range_start, new_range_end)),
                    //     )))
                    //     .await;
                } else {
                    new_state = Some(PresentationState::MainLobby(Some(0)));
                }
                // self.present().await;
            }
            PresentationState::Topic(t_id, pg_opt) => {
                eprintln!("show_next_page Topic {pg_opt:?}");
                if let Some(pg) = pg_opt {
                    // TODO: maybe set new pgid?
                    let next_pg = pg.saturating_add(1);
                    new_state = Some(PresentationState::Topic(*t_id, Some(next_pg)));
                    let new_range_start = next_pg * self.entries_count;
                    let new_range_end = (pg.saturating_add(2) * self.entries_count) - 1;
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            *t_id,
                            new_range_start,
                            new_range_end,
                        )))
                        .await;
                }
                // self.present().await;
            }
            _other => {
                //TODO
            }
        }
        if let Some(state) = new_state {
            self.presentation_state = state;
        }
    }

    async fn show_previous_page(&mut self) {
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(pg_opt) => {
                if let Some(pg) = pg_opt {
                    if *pg == 0 {
                        return;
                    }
                    // TODO: maybe set new pgid?
                    let prev_pg = pg.saturating_sub(1);
                    new_state = Some(PresentationState::MainLobby(Some(prev_pg)));
                    let new_range_start = prev_pg * self.entries_count;
                    let new_range_end = (pg * self.entries_count) - 1;
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                            self.shell.swarm_id,
                            Some((new_range_start, new_range_end)),
                        )))
                        .await;
                }
                // self.present().await;
            }
            PresentationState::Topic(t_id, pg_opt) => {
                if let Some(pg) = pg_opt {
                    if *pg == 0 {
                        return;
                    }
                    // TODO: maybe set new pgid?
                    let prev_pg = pg.saturating_sub(1);
                    new_state = Some(PresentationState::Topic(*t_id, Some(prev_pg)));
                    let new_range_start = prev_pg * self.entries_count;
                    let new_range_end = (pg * self.entries_count) - 1;
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            *t_id,
                            new_range_start,
                            new_range_end,
                        )))
                        .await;
                }
                // self.present().await;
            }
            _other => {
                //TODO
            }
        }
        if let Some(state) = new_state {
            self.presentation_state = state;
        }
    }
    async fn process_edit_result(&mut self, ed_res: EditorResult) {
        // eprintln!("FLogic got: {:?}", text);
        // DONE: validate received text against
        // what was expected to arrive.
        // If valid update selected item with
        // new value & present it back to user.
        // DONE: in order to validate, we need
        // to know what was being edited.
        //
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);

        // Remember to always update self.presentation_state!
        //
        match curr_state {
            PresentationState::Editing(id, prev_state) => {
                eprintln!("State Editing: {id:?}");
                match *prev_state {
                    PresentationState::Capability(cap, mut vec_gid) => {
                        let id = if let Some(i) = id {
                            i
                        } else {
                            vec_gid.len() as u16
                        };
                        if let EditorResult::Text(text) = ed_res {
                            // eprintln!("Trying to construct GID from {text}");
                            if let Some(g_id) = GnomeId::from_string(text) {
                                if id as usize >= vec_gid.len() {
                                    vec_gid.push(g_id);
                                } else {
                                    vec_gid[id as usize] = g_id;
                                }
                            }
                        } else {
                            eprintln!("Got nothing from Editor");
                        }
                        let v_len = vec_gid.len();
                        // let items_per_page: usize = 10;
                        let _page_no = id as usize / self.entries_count as usize;
                        let mut presentation_elems = Vec::with_capacity(10);
                        for i in 0..self.entries_count as usize {
                            if self.entries_count as usize * _page_no + i < v_len {
                                presentation_elems.push((
                                    i as u16,
                                    vec_gid[self.entries_count as usize * _page_no + i].to_string(),
                                ));
                            } else {
                                break;
                            }
                        }
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::ShowCapability(cap, presentation_elems));
                        self.presentation_state = PresentationState::Capability(cap, vec_gid);
                    }
                    PresentationState::MainLobby(page_opt) => {
                        self.presentation_state = PresentationState::MainLobby(page_opt);
                        if let EditorResult::Text(topic_desc) = ed_res {
                            eprintln!("We should add a new topic:\n{topic_desc}");
                            // let can_directly_append = false;
                            let entry = Entry::new(self.my_id, vec![], topic_desc, 0);
                            // if can_directly_append {
                            let data = entry.into_data(false).unwrap();
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::AppendContent(
                                    self.shell.swarm_id,
                                    DataType::Data(0),
                                    data,
                                ))
                                .await;
                            // } else {
                            //     // TODO: send custom app sync message
                            //     let forum_msg = ForumSyncMessage::AddTopic(0, entry);
                            //     let _ = self
                            //         .to_app_mgr_send
                            //         .send(ToAppMgr::AppDefined(
                            //             self.shell.swarm_id,
                            //             forum_msg.into_app_msg().unwrap(),
                            //         ))
                            //         .await;
                            // }
                        }
                    }
                    PresentationState::Settings => {
                        if let EditorResult::Text(text) = ed_res {
                            // We distinguish between what to do by value of id
                            if None == id {
                                //TODO: we have an updated Manifest descr.
                                eprintln!("New descr: {}", text);
                                self.shell.manifest.set_description(text);
                                let d_vec = self.shell.manifest.to_data();
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ChangeContent(
                                        self.shell.swarm_id,
                                        0,
                                        DataType::Data(0),
                                        d_vec,
                                    ))
                                    .await;
                                self.presentation_state = PresentationState::MainLobby(None);
                                self.present().await;
                            } else if Some(1) == id {
                                eprintln!("Adding new Category: {text}");
                                let tag = Tag::new(text).unwrap();
                                self.shell.manifest.add_tags(vec![tag]);
                                let d_vec = self.shell.manifest.to_data();
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::ChangeContent(
                                        self.shell.swarm_id,
                                        0,
                                        DataType::Data(0),
                                        d_vec,
                                    ))
                                    .await;
                                self.presentation_state = PresentationState::MainLobby(None);
                                self.present().await;
                            }
                        }
                    }
                    PresentationState::Topic(c_id, pg_opt) => {
                        if let EditorResult::Text(text) = ed_res {
                            let entry = Entry::new(self.my_id, vec![], text, 0);
                            if let Some(id) = id {
                                // TODO: check if current user can edit given post
                                eprintln!("Should update {id:?}");
                                // let can_directly_edit = false;
                                // if can_directly_edit {
                                let data = entry.into_data(id > 0).unwrap();
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::UpdateData(self.shell.swarm_id, c_id, id, data))
                                    .await;
                                // } else {
                                //     // TODO: send custom app sync message
                                //     let forum_msg = ForumSyncMessage::EditPost(c_id, id, entry);
                                //     let _ = self
                                //         .to_app_mgr_send
                                //         .send(ToAppMgr::AppDefined(
                                //             self.shell.swarm_id,
                                //             forum_msg.into_app_msg().unwrap(),
                                //         ))
                                //         .await;
                                // }
                            } else {
                                // request append data
                                // TODO: check if current user can edit given post
                                // TODO: how about if instead of checking if Policy is met for this
                                // Gnome, we send as if it was met, and see what happens.
                                // If it actually is met, we are done,
                                // if it is not we should receive back some PolicyNotMet msg
                                // with rejected contents.
                                // These msg bubbles up to Application that has sent in and then
                                // we decide what to do: either give up, or try TwoStep procedure.
                                // This simplifies Application logic and does not introduce
                                // another policy verification logic on application level.
                                //
                                eprintln!("Should append a post {id:?}");
                                // let can_directly_append = false;
                                // if can_directly_append {
                                let data = entry.into_data(true).unwrap();
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::AppendData(self.shell.swarm_id, c_id, data))
                                    .await;
                                // } else {
                                //     // TODO: send custom app sync message
                                //     let forum_msg = ForumSyncMessage::AddPost(c_id, entry);
                                //     let _ = self
                                //         .to_app_mgr_send
                                //         .send(ToAppMgr::AppDefined(
                                //             self.shell.swarm_id,
                                //             forum_msg.into_app_msg().unwrap(),
                                //         ))
                                //         .await;
                                // }
                            }
                        } else {
                            eprintln!("Ignore, empty text");
                        }

                        self.presentation_state = PresentationState::Topic(c_id, pg_opt);
                        self.present().await;
                        // present topics
                    }
                    PresentationState::TopicEditing(t_ctx) => {
                        // Here we defined a new Tag to be added to Manifest
                        let mut t_names = t_ctx.tag_names;
                        if let Some(tag_id) = id {
                            if let EditorResult::Text(text) = ed_res {
                                //TODO:
                                if (tag_id as usize) < t_names.len() {
                                    eprintln!("Editing Tag({tag_id}): {text}");
                                    t_names[tag_id as usize] = text;
                                } else {
                                    eprintln!("Adding Tag({tag_id}): {text}");
                                    t_names.push(text);
                                }
                            }
                        } else {
                            eprintln!("Unexpected value for id: {id:?}");
                        }
                        let new_ctx = TopicContext {
                            t_id: t_ctx.t_id,
                            description: t_ctx.description,
                            tags: t_ctx.tags,
                            tag_names: t_names,
                        };
                        self.presentation_state = PresentationState::TopicEditing(new_ctx.clone());
                        let _ = self.to_tui_send.send(ToForumView::OpenCreator(new_ctx));
                    }
                    other => {
                        eprintln!("Editing not supported yet");
                        self.presentation_state = other;
                    }
                }
            }
            PresentationState::ShowingPost(c_id, _pg_id) => match ed_res {
                EditorResult::Close => {
                    self.presentation_state = PresentationState::Topic(c_id, None);
                    self.present().await;
                }
                EditorResult::FirstPage => {
                    self.presentation_state = PresentationState::ShowingPost(c_id, 0);
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            c_id,
                            0,
                            0,
                        )))
                        .await;
                }
                EditorResult::PrevPage => {
                    if _pg_id == 0 {
                        self.presentation_state = PresentationState::Topic(c_id, None);
                        self.present().await;
                        return;
                    }
                    let ppage = u16::max(0, _pg_id - 1);
                    self.presentation_state = PresentationState::ShowingPost(c_id, ppage);
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            c_id,
                            ppage,
                            ppage,
                        )))
                        .await;
                }
                EditorResult::NextPage => {
                    if _pg_id == u16::MAX {
                        self.presentation_state = PresentationState::Topic(c_id, None);
                        self.present().await;
                        return;
                    }
                    let npage = u16::min(u16::MAX, _pg_id + 1);
                    self.presentation_state = PresentationState::ShowingPost(c_id, npage);
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            c_id,
                            npage,
                            npage,
                        )))
                        .await;
                }
                EditorResult::LastPage => {
                    self.presentation_state = PresentationState::ShowingPost(c_id, u16::MAX);
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            c_id,
                            1,
                            0,
                        )))
                        .await;
                }
                EditorResult::Text(text) => {
                    eprintln!("Unexpected text, when showing read-only post: {}", text);
                }
            },
            PresentationState::TopicEditing(t_ctx) => {
                if let EditorResult::Text(description) = ed_res {
                    let mut new_ctx = t_ctx.clone();
                    new_ctx.description = description.clone();
                    self.presentation_state = PresentationState::TopicEditing(TopicContext {
                        t_id: t_ctx.t_id,
                        description,
                        tags: t_ctx.tags,
                        tag_names: t_ctx.tag_names,
                    });
                    let _ = self.to_tui_send.send(ToForumView::OpenCreator(new_ctx));
                }
            }
            PresentationState::Fitlering => {
                // TODO: define local filtering logic for current swarm only
                if let EditorResult::Text(text) = ed_res {
                    // let _ = self
                    //     .to_app_mgr_send
                    //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::Search(
                    //         text.clone(),
                    //     )))
                    //     .await;
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        self.text_filter = None;
                    } else {
                        self.text_filter = Some(trimmed.to_string());
                    }
                    // TODO: define local logic for filtering topics
                    // send a request to get search results from elsewhere
                    // when search engine had time to process query
                    // let _ = self
                    //     .to_app_mgr_send
                    //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::GetSearchResults(
                    //         text,
                    //     )))
                    //     .await;
                } else {
                    self.text_filter = None;
                }
                self.presentation_state = PresentationState::MainLobby(Some(0));
                self.filter_topics().await;
                self.present().await;
            }
            other => {
                eprintln!("Edit result when in state: {:?}", other);
                self.presentation_state = other;
            }
        }
    }
    async fn process_creator_result(&mut self, c_res: CreatorResult) {
        eprintln!("in process_creator_result, {c_res:?}");
        if let PresentationState::TopicEditing(_tctx) = &self.presentation_state {
            // self.presentation_state = PresentationState::TopicEditing(_tctx);
            match c_res {
                CreatorResult::SelectDType => {
                    eprintln!("Should select DType (always Topic…)");
                    // Right now we will ignore thiss call, since we only have Topic DType
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::OpenCreator(_tctx.clone()));
                }
                CreatorResult::SelectTags => {
                    let mut names = _tctx.tag_names.clone();
                    if let Some(t_id) = _tctx.t_id {
                        if t_id == 0 {
                            if names.len() < 256 {
                                names.push(format!("+ Add new…"));
                            }
                            eprintln!("All Tags: {names:?}");
                            let _ = self.to_tui_send.send(ToForumView::Select(
                                t_id == 0,
                                names,
                                vec![],
                            ));
                        } else {
                            eprintln!("Should select Tags from: {names:?}");
                            let _ = self.to_tui_send.send(ToForumView::Select(
                                t_id == 0,
                                names,
                                _tctx.tags.clone(),
                            ));
                        }
                    } else {
                        // We are adding a new Topic
                        let _ = self.to_tui_send.send(ToForumView::Select(
                            false,
                            names,
                            _tctx.tags.clone(),
                        ));
                    }
                }
                CreatorResult::SelectDescription => {
                    if let Some(t_id) = _tctx.t_id {
                        if t_id > 0 {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                                    self.shell.swarm_id,
                                    t_id,
                                    0,
                                    0,
                                )))
                                .await;
                        } else {
                            eprintln!("Edit Forum's description");
                            let e_p = EditorParams {
                                title: format!("Edit Forum's description"),
                                initial_text: Some(_tctx.description.clone()),

                                allow_newlines: true,
                                chars_limit: None,
                                text_limit: Some(1000),
                                read_only: false,
                            };
                            let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                        }
                    } else {
                        eprintln!("Should set description");
                        let e_p = EditorParams {
                            title: format!("Adding new Topic"),
                            initial_text: Some(_tctx.description.clone()),

                            allow_newlines: true,
                            chars_limit: None,
                            text_limit: Some(800),
                            read_only: false,
                        };
                        let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                    }
                }
                CreatorResult::Create => {
                    if _tctx.t_id.is_some() {
                        let t_id = _tctx.t_id.unwrap();
                        if t_id == 0 {
                            eprintln!("Should edit Forum's Manifest",);
                            //TODO: support Editing Categories
                            let mut manifest = self.shell.manifest.clone();
                            manifest.set_description(_tctx.description.clone());
                            let mut new_tags = vec![];
                            for i in manifest.tags.len().._tctx.tag_names.len() {
                                eprintln!("Adding Tag: {}", _tctx.tag_names[i]);
                                new_tags.push(Tag::new(_tctx.tag_names[i].clone()).unwrap());
                            }
                            if !new_tags.is_empty() {
                                manifest.add_tags(new_tags);
                            }
                            for (id, text) in _tctx.tag_names.iter().enumerate() {
                                manifest.update_tag(id as u8, Tag::new(text.clone()).unwrap());
                            }
                            let m_data = manifest.to_data();
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::ChangeContent(
                                    self.shell.swarm_id,
                                    0,
                                    DataType::Data(0),
                                    m_data,
                                ))
                                .await;
                            self.presentation_state = PresentationState::MainLobby(None);
                        } else {
                            eprintln!("Should edit existing Topic {}", t_id);
                            let mut tags_u8 = Vec::with_capacity(_tctx.tags.len());
                            for t in &_tctx.tags {
                                tags_u8.push(*t as u8);
                            }
                            let entry: Entry =
                                Entry::new(self.my_id, tags_u8, _tctx.description.clone(), 0);
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::UpdateData(
                                    self.shell.swarm_id,
                                    t_id,
                                    0,
                                    entry.into_data(false).unwrap(),
                                ))
                                .await;
                            self.presentation_state = PresentationState::Topic(t_id, None);
                        }
                        self.present().await;
                    } else {
                        eprintln!("Should create new Topic");
                        let prev_state = std::mem::replace(
                            &mut self.presentation_state,
                            PresentationState::MainLobby(None),
                        );
                        if let PresentationState::TopicEditing(t_ctx) = prev_state {
                            let mut bytes = Vec::with_capacity(1024);
                            bytes.push(t_ctx.tags.len() as u8);
                            for tag in t_ctx.tags {
                                bytes.push(tag as u8);
                            }
                            for bte in self.my_id.bytes() {
                                bytes.push(bte);
                            }
                            bytes.append(&mut t_ctx.description.into_bytes());
                            let new_topic = Data::new(bytes).unwrap();
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::AppendContent(
                                    self.shell.swarm_id,
                                    DataType::Data(0),
                                    new_topic,
                                ))
                                .await;
                            self.present().await;
                        } else {
                            self.presentation_state = prev_state;
                        }
                    }
                }
                CreatorResult::Cancel => {
                    eprintln!("Should cancel");
                    if let Some(t_id) = _tctx.t_id {
                        if t_id > 0 {
                            self.presentation_state = PresentationState::Topic(t_id, None);
                        } else {
                            self.presentation_state = PresentationState::MainLobby(None);
                        }
                    } else {
                        self.presentation_state = PresentationState::MainLobby(None);
                    }
                    self.present().await;
                }
            }
        }
    }

    async fn serve_pol_action(&mut self, pa: PolAction) {
        eprintln!("PolAction: {:?}", pa);
        match pa {
            PolAction::SelectPolicy => {
                let curr_state = std::mem::replace(
                    &mut self.presentation_state,
                    PresentationState::MainLobby(None),
                );
                if let PresentationState::Pyramid(_p, r) = curr_state {
                    let (policies, p_strings) = Policy::mapping();
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::Select(true, p_strings, vec![]));
                    self.presentation_state = PresentationState::SelectingOnePolicy(policies, r);
                } else {
                    self.presentation_state = curr_state;
                }
            }
            PolAction::SelectRequirement(mut r_tree) => {
                //TODO: open selector with available
                // reqs to select & allow for one to be
                // picked
                // Then return that req to PolEditor
                let curr_state = std::mem::replace(
                    &mut self.presentation_state,
                    PresentationState::MainLobby(None),
                );
                if let PresentationState::Pyramid(p, _r) = curr_state {
                    let user_def_caps: Vec<u8> = vec![0];
                    let byte_sets: Vec<u8> = vec![0, 1, 2];
                    let two_byte_sets: Vec<u8> = vec![3];
                    let incl_logic = r_tree.mark_location().0 != 4;
                    let (reqs, r_strings) =
                        Requirement::mapping(incl_logic, user_def_caps, byte_sets, two_byte_sets);
                    self.presentation_state =
                        PresentationState::SelectingOneRequirement(reqs, p, r_tree);
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::Select(true, r_strings, vec![]));
                } else {
                    self.presentation_state = curr_state;
                }
            }
            PolAction::Store(p, r) => {
                eprintln!("PolAction::Store");
                let r_res = r.requirement();
                if let Ok(r) = r_res {
                    self.store_policy(p, r).await;
                } else {
                    eprintln!("Failed to build Requirements");
                }
                let _ = self.to_tui_send.send(ToForumView::TopicsPage(0, vec![]));
                self.presentation_state = PresentationState::MainLobby(Some(0));
                // TODO: allow for changing both
                // running & stored Policy at once
            }
            PolAction::Run(p, r) => {
                eprintln!("PolAction::Run");
                let r_res = r.requirement();
                if let Ok(r) = r_res {
                    self.run_policy(p, r).await;
                } else {
                    // TODO: Notify user
                    eprintln!("Failed to build Requirements");
                }
                let _ = self.to_tui_send.send(ToForumView::TopicsPage(0, vec![]));
                self.presentation_state = PresentationState::MainLobby(Some(0));
            }
        }
    }
    async fn serve_selected(&mut self, ids: Vec<usize>) {
        // TODO: this can not stay here, since then we do not cover cases
        // when no element was selected. We should reset presentation_state then
        // and possibly send something to presentation
        eprintln!("Selected ids: {:?}", ids);
        let curr_state = std::mem::replace(
            &mut self.presentation_state,
            PresentationState::MainLobby(None),
        );
        match curr_state {
            PresentationState::SelectingOneCapability(caps, prev_state) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                let id = ids[0];
                //TODO: this should be the only served arm
                match *prev_state {
                    PresentationState::RunningCapabilities(r_caps_opt) => {
                        let new_cap = caps[id];
                        let (c_id, mapping) = if let Some((c_id, mut mapping)) = r_caps_opt {
                            let key = mapping.len() as u16;
                            if mapping.contains_key(&key) {
                                //TODO: fix this
                                eprintln!("Ups, we need to find a key");
                            } else {
                                mapping.insert(key, (new_cap, vec![]));
                            }
                            (c_id, mapping)
                        } else {
                            let mut mapping = HashMap::new();
                            mapping.insert(0, (new_cap, vec![]));
                            (0, mapping)
                        };
                        // TODO: send updated list to presentation
                        let mut ccaps = Vec::with_capacity(self.entries_count as usize);
                        // let entries_len = 10;
                        for (i, (cap, _v_gids)) in mapping.iter() {
                            if *i < self.entries_count {
                                ccaps.push((*i, cap.text()));
                            }
                        }
                        eprintln!("CCaps: {:?}", ccaps);
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::RunningCapabilitiesPage(c_id, ccaps));
                        self.presentation_state =
                            PresentationState::RunningCapabilities(Some((c_id, mapping)));
                    }
                    PresentationState::StoredCapabilities(_p_opt) => {
                        //TODO
                        let c_keys = self.shell.manifest.capability_reg.keys();
                        let mut c_listing = Vec::with_capacity(c_keys.len());
                        let new_one = caps[id];
                        c_listing.push((new_one.byte() as u16, new_one.text()));
                        for key in c_keys {
                            c_listing.push((key.byte() as u16, key.text()));
                        }
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::StoredCapabilitiesPage(0, c_listing));
                        self.presentation_state = PresentationState::StoredCapabilities(Some(0));
                    }
                    // PresentationState::RunningPolicies(r_pols_opt) => {
                    //     //TODO: move logic from below
                    //     // but this can get tricky, since we can both
                    //     // select policy & req in the same state…
                    //     // maybe opt_id can be the judge which one it is.
                    // }
                    other => {
                        eprintln!("Selecting not supported");
                        self.presentation_state = other;
                    }
                }
            }
            PresentationState::SelectingOnePolicy(p_vec, r_tree) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                let id = ids[0];
                let p = p_vec[id];
                // let r_tree = decompose(req.clone());
                self.presentation_state = PresentationState::Pyramid(p.clone(), r_tree.clone());
                let _ = self.to_tui_send.send(ToForumView::ShowPolicy(p, r_tree));
            }
            PresentationState::SelectingOneRequirement(req_vec, pol, mut r_tree) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                let id = ids[0];
                eprintln!("should put req at: {:?}", r_tree.mark_location());
                let r = req_vec[id].clone();
                if r_tree.replace_mark(r) {
                    self.presentation_state =
                        PresentationState::Pyramid(pol.clone(), r_tree.clone());
                    let _ = self.to_tui_send.send(ToForumView::ShowPolicy(pol, r_tree));
                } else {
                    eprintln!("Did not find a Marker to replace");
                }
            }
            PresentationState::CreatingByteSet(opts, is_run, exist_opt) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                // let id = ids[0];
                let bset = if opts.len() == 256 {
                    let mut hset = HashSet::with_capacity(ids.len());
                    for id in ids {
                        hset.insert(id as u8);
                    }
                    eprintln!("We have a ByteSet");
                    ByteSet::Bytes(hset)
                } else {
                    let mut hset = HashSet::with_capacity(ids.len());
                    for id in ids {
                        hset.insert(id as u16);
                    }
                    eprintln!("We have a Pair ByteSet");
                    ByteSet::Pairs(hset)
                };
                let (id, mut h_map) = if let Some((i, s)) = exist_opt {
                    (i, s)
                } else {
                    (0, HashMap::new())
                };
                // If we run out of avail IDs, 0 will get overwritten…
                let mut avail_id = 0;
                for i in 0..=255 {
                    if !h_map.contains_key(&i) {
                        avail_id = i;
                        break;
                    }
                }
                h_map.insert(avail_id, bset);
                // TODO: rework below
                let mut bbsets = Vec::with_capacity(self.entries_count as usize);
                // let entries_len = 10;
                for (id, bset) in &h_map {
                    if bbsets.len() < self.entries_count as usize {
                        if bset.is_pair() {
                            bbsets.push((*id as u16, format!("PairSet({id})")));
                        } else {
                            bbsets.push((*id as u16, format!("ByteSet({id})")));
                        }
                    }
                }
                self.presentation_state = PresentationState::ByteSets(is_run, Some((id, h_map)));
                if is_run {
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::RunningByteSetsPage(id.into(), bbsets));
                } else {
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::StoredByteSetsPage(id.into(), bbsets));
                }
                // TODO: now we return back to previous presentation_state
                // with updated list.
                // User can now hit "Run" or "Store" button for selected BSet
            }
            PresentationState::ByteSets(is_run, mapping_opt) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                // let id = ids[0];
                eprintln!("Got Selected while in ByteSets");
                if let Some((id, mut hm)) = mapping_opt {
                    if let Some(bset) = hm.get_mut(&id) {
                        //TODO: update contents of ByteSet
                        let new_bset = if bset.is_pair() {
                            let mut new_hset = HashSet::with_capacity(ids.len());
                            for i in ids {
                                new_hset.insert(i as u16);
                            }
                            ByteSet::Pairs(new_hset)
                        } else {
                            let mut new_hset = HashSet::with_capacity(ids.len());
                            for i in ids {
                                new_hset.insert(i as u8);
                            }
                            ByteSet::Bytes(new_hset)
                        };
                        hm.insert(id, new_bset);
                        self.presentation_state =
                            PresentationState::ByteSets(is_run, Some((id, hm)));
                    } else {
                        // TODO: create new bset (this should not happen)
                    }
                }
            }
            PresentationState::TopicEditing(t_ctx) => {
                if ids.is_empty() {
                    eprintln!("No item was selected!");
                    return;
                }
                let id = ids[0];
                //TODO: If let Some(t_id) && t_id == 0{
                // if ids[0] == tags.len()      Then we create a new Tag
                // }
                let mut editing_tag = false;
                let new_ctxt = if let Some(t_id) = t_ctx.t_id {
                    if t_id == 0 {
                        // let t_len = t_ctx.tag_names.len();
                        // if id == t_len && t_len < 256 {
                        // TODO: Open Editor and allow new Tag to be created
                        // TODO: How to distinguish between Description & new Tag?
                        // Answer: We set self.presentation_state to different value
                        //
                        editing_tag = true;
                        TopicContext {
                            t_id: t_ctx.t_id,
                            description: t_ctx.description,
                            tags: t_ctx.tags,
                            tag_names: t_ctx.tag_names.clone(),
                        }
                        // } else {
                        //     TopicContext {
                        //         t_id: t_ctx.t_id,
                        //         description: t_ctx.description,
                        //         tags: t_ctx.tags,
                        //         tag_names: t_ctx.tag_names,
                        //     }
                        // }
                    } else {
                        TopicContext {
                            t_id: t_ctx.t_id,
                            description: t_ctx.description,
                            tags: ids,
                            tag_names: t_ctx.tag_names.clone(),
                        }
                    }
                } else {
                    TopicContext {
                        t_id: t_ctx.t_id,
                        description: t_ctx.description,
                        tags: ids,
                        tag_names: t_ctx.tag_names.clone(),
                    }
                };
                if editing_tag {
                    self.presentation_state = PresentationState::Editing(
                        Some(id as u16),
                        Box::new(PresentationState::TopicEditing(new_ctxt.clone())),
                    );
                    let initial_text = if id < t_ctx.tag_names.len() {
                        Some(t_ctx.tag_names[id].clone())
                    } else {
                        Some(format!("NewCategory"))
                    };
                    let e_params = EditorParams {
                        title: format!("Add new Category (oneline, 32 bytes max)"),
                        initial_text,
                        allow_newlines: false,
                        chars_limit: None,
                        text_limit: Some(32),
                        read_only: false,
                    };
                    let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_params));
                } else {
                    self.presentation_state = PresentationState::TopicEditing(new_ctxt.clone());
                    let _ = self.to_tui_send.send(ToForumView::OpenCreator(new_ctxt));
                }
            }
            PresentationState::Fitlering => {
                let mut t_ids = Vec::with_capacity(ids.len());
                for id in ids {
                    t_ids.push(id as u8);
                }
                self.presentation_state = PresentationState::MainLobby(None);
                self.category_filter = if t_ids.is_empty() { None } else { Some(t_ids) };

                // let _ = self
                //     .to_app_mgr_send
                //     .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                //         self.shell.swarm_id,
                //         Some((1, u16::MAX)),
                //     )))
                //     .await;
                self.filter_topics().await;
                self.present().await;
            }
            other => {
                self.presentation_state = other;
            }
        }
    }
    async fn serve_query(&mut self, id: u16) {
        eprintln!("Logic got a query {:?} ({:?})", id, self.presentation_state);
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(page_opt) => {
                //TODO: retrieve a Topic and present it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (1)");
                    return;
                }
                let _page = page_opt.unwrap();
                // let topic_id = id + self.entries_count * page;
                let topic_id = id;
                if topic_id == 0 {
                    //TODO: open Editor with Manifest desc
                    // in READ only mode
                    // let e_params = EditorParams {
                    //     title: format!("Viewing Forum's description"),
                    //     initial_text: Some(self.shell.manifest.description.clone()),
                    //     allow_newlines: true,
                    //     chars_limit: None,
                    //     text_limit: Some(1000),
                    //     read_only: true,
                    // };
                    // let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_params));
                    let description = self.shell.manifest.description.clone();
                    let mut tags: Vec<usize> = vec![];
                    let mut tag_names: Vec<String> = vec![];
                    for i in 0..=255 {
                        if let Some(tag) = self.shell.manifest.tags.get(&i) {
                            tags.push(i as usize);
                            tag_names.push(tag.0.clone());
                        }
                    }

                    let _ = self
                        .to_tui_send
                        .send(ToForumView::OpenCreator(TopicContext {
                            t_id: Some(0),
                            description: description.clone(),
                            tags: tags.clone(),
                            tag_names: tag_names.clone(),
                        }));
                    self.presentation_state = PresentationState::TopicEditing(TopicContext {
                        t_id: Some(topic_id),
                        description,
                        tags,
                        tag_names,
                    });
                } else {
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            topic_id,
                            0,
                            self.entries_count - 1,
                        )))
                        .await;
                    self.presentation_state = PresentationState::Topic(topic_id, None);
                    eprintln!("cleaning topics…");
                    self.posts = vec![];
                    // TODO: instead of opening Editor
                    // switch to selected Topic Menu
                    // and present Posts
                    // let e_params = EditorParams {
                    //     initial_text: Some(self.topics[topic_id as usize].clone()),
                    //     allow_newlines: true,
                    //     chars_limit: None,
                    //     text_limit: Some(1000),
                    //     read_only: true,
                    // };
                    // let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_params));
                }
                // TODO: we need to store a mapping from id_on_page
                // to absolute id to determine which content to present
                //
                // When we know CID, we request a bunch of it's pages from
                // datastore.
                // We only need to show as many pages as there are
                // entry buttons on screen.
                // Here logic ends, and once we receive those pages
                // other logic handles them.
            }
            PresentationState::Topic(c_id, page_opt) => {
                //TODO: map query ID to DID using page_opt
                // and ask AppManager for selected Data.
                // Once received open Editor
                // in ReadOnly mode.
                if let Some(_page_nr) = page_opt {
                    eprintln!("Topic {c_id} P#{_page_nr} ID: {id}");
                    // let d_id = page_nr * self.entries_count + id;
                    let d_id = id;
                    if d_id == 0 {
                        // TODO:
                        // let _ = self
                        //     .to_tui_send
                        //     .send(ToForumView::OpenCreator(TopicContext {
                        //         t_id: Some(*c_id0),
                        //         description: description.clone(),
                        //         tags: tags.clone(),
                        //     }));
                        eprintln!("NS TopicEditing");

                        let mut tags_usize = Vec::with_capacity(self.posts[0].tags.len());
                        for t in self.posts[0].tags.iter() {
                            tags_usize.push(*t as usize);
                        }

                        let description = self.posts[0].text.clone();
                        new_state = Some(PresentationState::TopicEditing(TopicContext {
                            t_id: Some(*c_id),
                            description: description.clone(),
                            tags: tags_usize.clone(),
                            tag_names: self.shell.manifest.tag_names(None),
                        }));
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::OpenCreator(TopicContext {
                                t_id: Some(*c_id),
                                description: description.clone(),
                                tags: tags_usize,
                                tag_names: self
                                    .shell
                                    .manifest
                                    .tag_names(Some(self.posts[0].tags.clone())),
                            }));
                    } else {
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                                self.shell.swarm_id,
                                *c_id,
                                d_id,
                                d_id,
                            )))
                            .await;
                        eprintln!("NS ShowingPost");
                        new_state = Some(PresentationState::ShowingPost(*c_id, d_id));
                    }
                } else {
                    eprintln!("Got a Query in Topics,no page");
                }
            }
            PresentationState::Settings => {
                // TODO: when we get Query in settings
                // it means User wants to change Manifest
                // description
                eprintln!("Should edit Manifest Descr.");
                let e_params = EditorParams {
                    title: format!("Editing Forum's description"),
                    initial_text: Some(self.shell.manifest.description.clone()),
                    allow_newlines: true,
                    chars_limit: None,
                    text_limit: Some(1000),
                    read_only: false,
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_params));
                // TODO: maybe we should create a dedicated
                // state for Editing anything?
                new_state = Some(PresentationState::Editing(
                    None,
                    Box::new(PresentationState::Settings),
                ));
            }
            PresentationState::RunningPolicies(page_opt) => {
                if let Some((_page_no, mapping)) = page_opt {
                    if let Some((p, r)) = mapping.get(&id) {
                        let r_tree = decompose(r.clone());
                        new_state = Some(PresentationState::Pyramid(p.clone(), r_tree.clone()));
                        //TODO: get Policy & Requirements & show it to user
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::ShowPolicy(p.clone(), r_tree));
                    } else {
                        eprintln!("Did not find running policy for {id}");
                    }
                } else {
                    eprintln!("Unable to tell which page is shown (2)");
                    return;
                };
                // Here we should also store running policies in
                // local cache as those are not often changed.
                // But once we swich PresentationState to a different
                // view that cache should be wiped out.
            }
            PresentationState::RunningCapabilities(r_caps) => {
                //TODO: get Capability & show it to user
                // let items_per_page = 10;
                if r_caps.is_none() {
                    eprintln!("Unable to tell which page is shown (3)");
                    return;
                }
                eprintln!("Got non-empty Caps page");
                if let Some((_page_no, mapping)) = r_caps {
                    if let Some((c, v_gid)) = mapping.get(&id) {
                        let v_len = v_gid.len();
                        // TODO: make use of items_per_page
                        // TODO: items_per_page should be predefined
                        let mut presentation_elems = Vec::with_capacity(10);
                        for i in 0..self.entries_count as usize {
                            if self.entries_count as usize * (*_page_no as usize) + i < v_len {
                                presentation_elems.push((
                                    i as u16,
                                    v_gid[self.entries_count as usize * (*_page_no as usize) + i]
                                        .to_string(),
                                ));
                            } else {
                                break;
                            }
                        }
                        new_state = Some(PresentationState::Capability(c.clone(), v_gid.clone()));

                        let _ = self
                            .to_tui_send
                            .send(ToForumView::ShowCapability(*c, presentation_elems));
                    }
                }
            }
            PresentationState::ByteSets(is_run, page_opt) => {
                //TODO: get ByteSet & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (4)");
                    return;
                }
                eprintln!("Got non-empty BSets page");
                if let Some((_i, hm)) = page_opt {
                    if let Some(bset) = hm.get(&(id as u8)) {
                        let (strings, preselected) = if bset.is_pair() {
                            let mut strings = vec![];
                            let mut preselected = vec![];
                            for p in 0..=u16::MAX {
                                strings.push(format!("Pair {p}"));
                                if bset.contains_pair(&p) {
                                    preselected.push(p as usize);
                                }
                            }
                            (strings, preselected)
                        } else {
                            let mut strings = vec![];
                            let mut preselected = vec![];
                            for p in 0..=255 {
                                strings.push(format!("Byte {p}"));
                                if bset.contains(&p) {
                                    preselected.push(p as usize);
                                }
                            }
                            (strings, preselected)
                        };
                        let _ =
                            self.to_tui_send
                                .send(ToForumView::Select(false, strings, preselected));
                        new_state = Some(PresentationState::ByteSets(
                            *is_run,
                            Some((id as u8, hm.clone())),
                        ));
                    };
                }
            }
            PresentationState::StoredPolicies(_page_opt, pol_map) => {
                //TODO: get Policy & Requirements & show it to user
                // if page_opt.is_none() {
                //     eprintln!("Unable to tell which page is shown (5)");
                //     return;
                // }
                if let Some(pol) = pol_map.get(&(id as u8)) {
                    if let Some(req) = self.shell.manifest.policy_reg.get(pol) {
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::ShowPolicy(*pol, decompose(req.clone())));
                    }
                }
                // let page = page_opt.unwrap();
                // Here we should store only a mapping from on_page_id
                // to actual policy.
                // And now we should retrieve that Policy from Manifest,
                // which should be kept up-to-date.
            }
            PresentationState::StoredCapabilities(_page_opt) => {
                let cap = Capabilities::from(id as u8);
                if let Some(c_tree) = self.shell.manifest.capability_reg.get(&cap) {
                    let all_members = c_tree.get_all_members();
                    eprintln!("Members #:{}", all_members.len());
                    let mut mem_enum = Vec::with_capacity(all_members.len());
                    for (i, member) in all_members.iter().enumerate() {
                        mem_enum.push((i as u16, format!("GID-{:016x}", member.0)));
                    }
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::ShowCapability(cap, mem_enum));
                    new_state = Some(PresentationState::Capability(cap, all_members));
                } else {
                    // We just created a Capability, that does not exist in Manifest yet
                    let _ = self
                        .to_tui_send
                        .send(ToForumView::ShowCapability(cap, vec![]));
                    new_state = Some(PresentationState::Capability(cap, vec![]));
                }
                //TODO: get Capability & show it to user
                // if page_opt.is_none() {
                //     eprintln!("Unable to tell which page is shown (6)");
                //     return;
                // }
                // let page = page_opt.unwrap();
                // similar to StoredPolicies
            }
            // PresentationState::StoredByteSets(page_opt) => {
            //     //TODO: get ByteSet & show it to user
            //     if page_opt.is_none() {
            //         eprintln!("Unable to tell which page is shown (7)");
            //         return;
            //     }
            //     let page = page_opt.unwrap();
            //     // similar to StoredPolicies
            // }
            PresentationState::SelectingOnePolicy(_pol_vec, _req) => {
                //TODO
                eprintln!("serve queue while in SelectingOnePolicy");
            }
            PresentationState::SelectingOneRequirement(_req_vec, _pol, _r_tree) => {
                eprintln!("serve queue while in SelectingOneRequirement");
            }
            PresentationState::Pyramid(_pol, _req) => {
                eprintln!("serve queue while in Pyramid");
            }
            PresentationState::Capability(c, v_gid) => {
                eprintln!("TODO: We should open an Editor with Selected GID");
                // new_state = Some(PresentationState::Capability(c.clone(), v_gid.clone()));

                let g_id = v_gid[id as usize];
                let initial_text = if let Some((s_name, _id)) = &self.clipboard {
                    Some(format!("GID-{:016x}", s_name.founder.0))
                } else {
                    Some(format!("{g_id}"))
                };

                let e_p = EditorParams {
                    title: format!("Adding new Capability"),
                    initial_text,
                    allow_newlines: false,
                    chars_limit: Some("0123456789abcdef".chars().collect()),
                    text_limit: Some(20),
                    read_only: false,
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                new_state = Some(PresentationState::Editing(
                    Some(id),
                    Box::new(PresentationState::Capability(*c, v_gid.clone())),
                ));
            }
            PresentationState::HeapSorting(_opt, entry_opt) => {
                if let Some((f_msg, _signer)) = _opt {
                    match &f_msg {
                        ForumSyncMessage::EditPost(_t_id, _p_id, entry) => {
                            // TODO
                            if id == 1 {
                                let e_p = EditorParams {
                                    title: format!("Reviewing modified Post"),
                                    initial_text: Some(entry.text.clone()),
                                    allow_newlines: false,
                                    chars_limit: None,
                                    text_limit: None,
                                    read_only: true,
                                };
                                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                                // new_state = Some(PresentationState::Editing(
                                //     Some(id),
                                //     Box::new(PresentationState::HeapSorting(
                                //         Some((f_msg.clone(), *signer)),
                                //         *entry_opt,
                                //     )),
                                // ));
                            } else if id == 0 {
                                if let Some(entry) = entry_opt {
                                    let e_p = EditorParams {
                                        title: format!("Viewing original Post"),
                                        initial_text: Some(entry.text.clone()),
                                        allow_newlines: false,
                                        chars_limit: None,
                                        text_limit: None,
                                        read_only: true,
                                    };
                                    let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                                    // new_state = Some(PresentationState::Editing(
                                    //     Some(id),
                                    //     Box::new(PresentationState::HeapSorting(
                                    //         Some((f_msg.clone(), *signer)),
                                    //         *entry_opt,
                                    //     )),
                                    // ));
                                } else {
                                    // new_state = Some(PresentationState::Editing(
                                    //     Some(id),
                                    //     Box::new(PresentationState::HeapSorting(
                                    //         Some((f_msg.clone(), *signer)),
                                    //         None,
                                    //     )),
                                    // ));
                                }
                            }
                        }
                        ForumSyncMessage::AddTopic(_t_id, entry) => {
                            if id == 0 {
                                let e_p = EditorParams {
                                    title: format!("Reviewing new Topic"),
                                    initial_text: Some(entry.text.clone()),
                                    allow_newlines: false,
                                    chars_limit: None,
                                    text_limit: None,
                                    read_only: true,
                                };
                                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                            } else {
                                eprintln!("Only id=0 expected when reviewing add topic, {id}");
                            }
                        }
                        ForumSyncMessage::AddPost(_t_id, entry) => {
                            if id == 0 {
                                let e_p = EditorParams {
                                    title: format!("Reviewing new Post"),
                                    initial_text: Some(entry.text.clone()),
                                    allow_newlines: false,
                                    chars_limit: None,
                                    text_limit: None,
                                    read_only: true,
                                };
                                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                            } else {
                                eprintln!("Only id=0 expected when reviewing add post, {id}");
                            }
                        }
                    }
                } else {
                    // Query is used to trigger reading from heap
                    // new_state = Some(PresentationState::HeapSorting(None));
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::PeekHeap(
                            self.shell.swarm_id,
                        )))
                        .await;
                }
            }
            PresentationState::Editing(_id, _prev_state) => {
                eprintln!("Got ID when in state Editing");
            }
            PresentationState::SelectingOneCapability(_v_caps, _p_state) => {
                eprintln!("Got ID when in state SelectingOneCapability");
            }
            PresentationState::CreatingByteSet(_opts, _is_run, _ex_opt) => {
                eprintln!("Got ID when creating ByteSet");
            }
            PresentationState::ShowingPost(_c, _p) => {
                eprintln!("Got ID when showing Post");
            }
            PresentationState::TopicEditing(_tctx) => {
                //ignore
            }
            PresentationState::Fitlering => {
                eprintln!("Querying not supported while defining Filtering conditions");
            }
        }
        if let Some(n_s) = new_state {
            self.presentation_state = n_s;
        }
    }
    async fn run_policy(&mut self, pol: Policy, req: Requirement) {
        eprintln!("In run_policy");
        // TODO: update selected policy to new value by
        // sending a msg to Gnome in order for
        // him to reconfigure Swarm.
        let _ = self
            .to_app_mgr_send
            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::SetRunningPolicy(
                self.shell.swarm_name.clone(),
                pol,
                req,
            )))
            .await;
    }
    async fn store_policy(&mut self, pol: Policy, req: Requirement) {
        eprintln!("In store_policy");
        self.shell.manifest.policy_reg.insert(pol, req);
        let m_data = self.shell.manifest.to_data();
        let _ = self
            .to_app_mgr_send
            .send(ToAppMgr::ChangeContent(
                self.shell.swarm_id,
                0,
                DataType::Data(0),
                m_data,
            ))
            .await;
        // TODO: update selected policy to new value.
        // we should update Manifest also at Swarm level
        //
        // We probably need a SwarmShell instance.
        // Next we need to have retrinve Manifest from Swarm.
        // Then we need to update/add given Policy.
        // Now we send requests to Gnome in order to update
        //
    }
    async fn sync_request_rejected(&self, s_id: SwarmID, sm_type: SyncMessageType, data: Data) {
        if s_id != self.shell.swarm_id {
            // Only support our own msgs
            return;
        }

        eprintln!("Forum app got rejected: {sm_type:?}");
        match sm_type {
            SyncMessageType::AppendContent(d_type) => {
                if d_type.byte() == 0 {
                    // try 2step process of adding Topic
                    let msg = ForumSyncMessage::AddTopic(0, Entry::from_data(data, false).unwrap())
                        .into_app_msg()
                        .unwrap();
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::AppDefined(self.shell.swarm_id, msg))
                        .await;
                }
            }
            SyncMessageType::ChangeContent(_c_id, _d_type, _op) => {
                //TODO
            }
            SyncMessageType::AppendData(c_id) => {
                // try 2step process of adding Topic
                let msg = ForumSyncMessage::AddPost(c_id, Entry::from_data(data, true).unwrap())
                    .into_app_msg()
                    .unwrap();
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AppDefined(self.shell.swarm_id, msg))
                    .await;
            }
            SyncMessageType::AppendShelledDatas(_c_id) => {
                //TODO
            }
            SyncMessageType::RemoveData(_c_id, _d_id) => {
                //TODO
            }
            SyncMessageType::UpdateData(c_id, d_id) => {
                // try 2step process of adding Topic
                let msg = ForumSyncMessage::EditPost(
                    c_id,
                    d_id,
                    Entry::from_data(data, d_id > 0).unwrap(),
                )
                .into_app_msg()
                .unwrap();
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AppDefined(self.shell.swarm_id, msg))
                    .await;
            }
            SyncMessageType::InsertData(_c_id, _d_id) => {
                //TODO
            }
            SyncMessageType::ExtendData(_c_id, _d_id) => {
                //TODO
            }
            SyncMessageType::AppDefined(_m_type, _c_id, _d_id) => {
                //TODO: here if a msg got rejected and we are out of options
                // we could send User a notification, that his request can not be fullfilled
            }
        }
    }

    async fn process_search_results(&mut self, query: String, hits: Vec<Hit>) {
        //TODO
        eprintln!(
            "in process_search_results {} have {} hits",
            query,
            hits.len()
        );
    }

    async fn filter_topics(&mut self) {
        // TODO: filtering logic should allow for fast switching between MainMenu pages.
        // For that to work we need some sort of helper structure that will contain
        // all topic pages after filtering is done.
        // Each page should contain indices of topic to include.
        // Above logic should only apply for MainMenu not for Topic menu type.
        // If no filters are defined, we should present all topics.
        // If both category_filter & text_filter are defined we should filter all topics twice:
        // first by category_filter, second by text filter.
        // If any filter changes we should restart filtering from scratch.
        //
        // First we filter by category, if cat_fltr is defined.
        // We take every topic & check if it has any category that is included in cat_fltr.
        let all_topics: Vec<u16> = (0..self.all_topics.len() as u16).collect();
        let first_result = if let Some(categories) = &self.category_filter {
            let mut filtered_by_cat = vec![];
            for t_id in all_topics {
                for t in &self.all_topics[t_id as usize].tags {
                    if categories.contains(&t) {
                        filtered_by_cat.push(t_id);
                        break;
                    }
                }
            }
            filtered_by_cat
        } else {
            all_topics
        };
        //
        // Second we take those topics and run text filter, if defined.
        // Here we take the header of a topic and check if it contains a specific text.
        // If so we include such topic in results.
        //
        let final_result = if let Some(text_f) = &self.text_filter {
            let mut filtered_by_text = vec![];
            for t_id in first_result {
                if self.all_topics[t_id as usize].text.contains(text_f) {
                    filtered_by_text.push(t_id);
                }
            }
            filtered_by_text
        } else {
            first_result
        };
        // Last step is to take resulting topics and build out main_pages from them.

        self.menu_pages = vec![];
        let mut curr_page = Vec::with_capacity(self.entries_count as usize);
        for t_id in final_result {
            curr_page.push(t_id);
            if curr_page.len() == self.entries_count as usize {
                self.menu_pages.push(curr_page);
                curr_page = Vec::with_capacity(self.entries_count as usize);
            }
        }
        if !curr_page.is_empty() || self.menu_pages.is_empty() {
            self.menu_pages.push(curr_page);
        }
        // eprintln!("in filter_topics");
    }
    fn set_clipboard(&mut self, which: u16) {
        match &self.presentation_state {
            PresentationState::MainLobby(pg_opt) => {
                if let Some(pg) = pg_opt {
                    self.clipboard = Some((
                        self.shell.swarm_name.clone(),
                        pg * self.entries_count + which,
                    ));
                } else {
                    self.clipboard = Some((self.shell.swarm_name.clone(), which));
                }
            }
            PresentationState::Topic(t_id, _pg_opt) => {
                self.clipboard = Some((self.shell.swarm_name.clone(), *t_id));
            }
            _other => {
                self.clipboard = Some((self.shell.swarm_name.clone(), which));
            }
        }
    }
}

pub async fn from_forum_tui_adapter(
    from_presentation: Receiver<FromForumView>,
    wrapped_sender: ASender<InternalMsg>,
) {
    let timeout = Duration::from_millis(16);
    loop {
        let recv_res = from_presentation.recv_timeout(timeout);
        match recv_res {
            Ok(from_tui) => {
                let _ = wrapped_sender.send(InternalMsg::Forum(from_tui)).await;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => sleep(timeout).await,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
    eprintln!("from_tui_adapter is done");
}
pub async fn start_a_timer(sender: ASender<ToAppMgr>, message: ToAppMgr, timeout: Duration) {
    // let timeout = Duration::from_secs(5);
    sleep(timeout).await;
    eprintln!("Timeout {:?} is over, sending message…", timeout);
    let _ = sender.send(message).await;
}
