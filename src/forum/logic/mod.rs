use crate::catalog::logic::SwarmShell;
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
use dapp_lib::prelude::Capabilities;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::DataType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::Manifest;
use dapp_lib::prelude::Policy;
use dapp_lib::prelude::Requirement;
use dapp_lib::prelude::SwarmName;
use dapp_lib::Data;
use dapp_lib::ToApp;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

use dapp_lib::prelude::AppType;

#[derive(Debug)]
enum PresentationState {
    MainLobby(Option<u16>),
    Topic(u16, Option<u16>),
    ShowingPost(u16, u16),
    Settings,
    RunningPolicies(Option<(u16, HashMap<u16, (Policy, Requirement)>)>),
    RunningCapabilities(Option<(u16, HashMap<u16, (Capabilities, Vec<GnomeId>)>)>),
    ByteSets(bool, Option<(u8, HashMap<u8, ByteSet>)>),
    StoredPolicies(Option<u16>),
    StoredCapabilities(Option<u16>),
    // StoredByteSets(Option<u16>),
    SelectingOnePolicy(Vec<Policy>, ReqTree),
    SelectingOneRequirement(Vec<Requirement>, Policy, ReqTree),
    Pyramid(Policy, ReqTree),
    Capability(Capabilities, Vec<GnomeId>),
    SelectingOneCapability(Vec<Capabilities>, Box<PresentationState>),
    Editing(Option<u16>, Box<PresentationState>),
    CreatingByteSet(Vec<u16>, bool, Option<(u8, HashMap<u8, ByteSet>)>),
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
use crate::forum::tui::serve_forum_tui;
use crate::forum::tui::Action;
use crate::forum::tui::FromForumView;
use crate::forum::tui::ToForumView;
use crate::InternalMsg;
use crate::Toolset;
pub struct ForumLogic {
    my_id: GnomeId,
    shell: SwarmShell,
    entries: Vec<String>,
    // posts: ()
    presentation_state: PresentationState,
    to_app_mgr_send: ASender<ToAppMgr>,
    _to_user_send: ASender<InternalMsg>,
    to_user_recv: AReceiver<InternalMsg>,
    to_tui_send: Sender<ToForumView>,
    to_tui_recv: Option<Receiver<ToForumView>>,
    from_tui_send: Option<Sender<FromForumView>>,
}
impl ForumLogic {
    pub fn new(
        my_id: GnomeId,
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
            shell,
            entries: vec![],
            to_app_mgr_send,
            _to_user_send: to_user_send,
            to_user_recv,
            to_tui_send,
            to_tui_recv: Some(to_tui_recv),
            from_tui_send: Some(from_tui_send),
        }
    }
    pub async fn run(
        mut self,
        founder: GnomeId,
        _config_dir: PathBuf,
        toolset: Toolset,
        // mut config: Configuration,
        // mut tui_mgr: Manager,
        // ) -> Option<(AppType, AReceiver<InternalMsg>, Configuration, Manager)> {
    ) -> Option<(Option<AppType>, SwarmName, AReceiver<InternalMsg>, Toolset)> {
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
            Some((Some(switch_app), s_name, self.to_user_recv, toolset))
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

                    // eprintln!("Forum request FirstPages");
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                            s_id, None,
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
                    self.present_topics().await;
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
                        self.present_topics().await;
                    } else {
                        eprintln!("But no first page!");
                    }
                } else {
                    eprintln!("ContentChanged for {s_id}, my_id:{}", self.shell.swarm_id);
                }
            }
            ToApp::RunningPolicies(mut policies) => {
                eprintln!("#{} RunningPolicies in forum logic", policies.len());
                let pol_page = 0;

                // TODO: store how many entries there are in menu.
                let mut pols = Vec::with_capacity(10);
                let mut mapping = HashMap::with_capacity(policies.len());
                let entries_len = 10;
                for i in 0..policies.len() as u16 {
                    let pol = policies.remove(0);
                    if i < entries_len {
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
                let mut ccaps = Vec::with_capacity(10);
                let mut mapping = HashMap::with_capacity(caps.len());
                let entries_len = 10;
                for i in 0..caps.len() as u16 {
                    let (cap, gid_list) = caps.remove(0);
                    if i < entries_len {
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
                let mut bbsets = Vec::with_capacity(10);
                let mut mapping = HashMap::with_capacity(bsets.len());
                let entries_len = 10;
                while !bsets.is_empty() {
                    let (bs_id, bs_list) = bsets.remove(0);
                    if bbsets.len() < entries_len {
                        bbsets.push((bs_id as u16, format!("ByteSet({bs_id})")));
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
                    self.present_topics().await;
                } else {
                    eprintln!("FirstPages of other swarm");
                }
            }
            ToApp::NewContent(s_id, c_id, d_type, data) => {
                //TODO
                eprintln!("We need to process new content!");
                if s_id == self.shell.swarm_id {
                    self.append_first_page(c_id, d_type, data).await;
                    self.present_topics().await;
                }
            }
            ToApp::HeapData(s_id, m_type, _data, _signed_by) => {
                if s_id == self.shell.swarm_id {
                    eprintln!("Forum recv Heap Data {}", m_type);
                } else {
                    eprintln!("Heap data of {} ignored", s_id);
                }
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
                        self.presentation_state = PresentationState::StoredPolicies(None);
                        //TODO: Send policies from Manifest
                        let pol_page = 0;
                        let pols = vec![
                            (11, "Stored Policy One".to_string()),
                            (22, "Stored Policy Two".to_string()),
                        ];
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
                        self.presentation_state = PresentationState::StoredCapabilities(None);
                    }
                    Action::ByteSets(_is_run) => {
                        //TODO:
                        self.presentation_state = PresentationState::RunningCapabilities(None);
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::RunningByteSets(
                                self.shell.swarm_name.clone(),
                            )))
                            .await;
                    }
                    // Action::StoredByteSets => {
                    //     //TODO:
                    //     self.presentation_state =
                    //         PresentationState::ByteSets(false, None);
                    // }
                    Action::AddNew(param) => {
                        self.add_new_action(param).await;
                    }
                    Action::Edit(_id) => {
                        let curr_state = std::mem::replace(
                            &mut self.presentation_state,
                            PresentationState::Settings,
                        );
                        match curr_state {
                            PresentationState::Topic(c_id, pg_opt) => {
                                eprintln!("Action Edit when in Topic");
                                if let Some(page) = pg_opt {
                                    let post_id = (page * 10) + _id;
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
                                }
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
                    Action::NextPage => {
                        //TODO
                    }
                    Action::PreviousPage => {
                        //TODO
                    }
                    Action::FirstPage => {
                        //TODO
                    }
                    Action::LastPage => {
                        //TODO
                    }
                    Action::Filter(_filter) => {
                        //TODO
                    }
                    Action::Query(qt) => {
                        self.serve_query(qt).await;
                    }
                    Action::MainMenu => {
                        //TODO
                        eprintln!("Action::MainMenu presenting topics");
                        self.presentation_state = PresentationState::MainLobby(None);
                        self.entries = vec![];
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadFirstPages(
                                self.shell.swarm_id,
                                Some((1, 10)),
                            )))
                            .await;
                        let m_head: String = self
                            .shell
                            .manifest
                            .description
                            .lines()
                            .take(1)
                            // .trimmed()
                            .collect();
                        self.update_topic(0, m_head);
                        self.present_topics().await;
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
            FromForumView::SwitchTo(app_type, s_name) => {
                *switch_to_opt = Some((app_type, s_name));
                return true;
            }
        }
        false
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
        mut d_vec: Vec<Data>,
    ) {
        if d_vec.is_empty() {
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
                    // TODO: include start_page
                    let curr_state = std::mem::replace(
                        &mut self.presentation_state,
                        PresentationState::Settings,
                    );
                    match curr_state {
                        PresentationState::MainLobby(_pg_opt) => {
                            // if self.presentation_state.is_main_menu() {
                            //TODO
                            let first = d_vec.remove(0);
                            if let Ok(text) = std::str::from_utf8(&first.bytes()) {
                                let first_line = if let Some(line) = text.lines().next() {
                                    line.to_string()
                                } else {
                                    "Empty".to_string()
                                };
                                let header: String = first_line.chars().take(64).collect();
                                self.update_topic(c_id as usize, header.trim().to_string());
                            };
                            self.presentation_state = curr_state;
                        }
                        PresentationState::Topic(_t_id, _pg_opt) => {
                            // } else if self.presentation_state.is_topic() {
                            // TODO: include start_page
                            for (id, first) in d_vec.into_iter().enumerate() {
                                if let Ok(text) = std::str::from_utf8(&first.bytes()) {
                                    let first_line = if let Some(line) = text.lines().next() {
                                        line.to_string()
                                    } else {
                                        "Empty".to_string()
                                    };
                                    let header: String = first_line.chars().take(64).collect();
                                    self.update_topic(id, header.trim().to_string());
                                };
                            }
                            self.presentation_state = curr_state;
                        }
                        PresentationState::ShowingPost(_t_id, _p_id) => {
                            // } else if self.presentation_state.is_showing_post(&c_id, &start_page) {
                            let initial_text =
                                Some(String::from_utf8(d_vec[0].clone().bytes()).unwrap());
                            let e_p = EditorParams {
                                initial_text,
                                allow_newlines: true,
                                chars_limit: None,
                                text_limit: None,
                                read_only: true,
                            };
                            let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                            self.presentation_state = curr_state;
                        }

                        PresentationState::Editing(what_opt, prev_state) => {
                            eprintln!("ReadSuccess in Editing State");
                            // TODO: check if what we got matches what we want!

                            let new_what;
                            if what_opt.is_none() {
                                new_what = Some(start_page);
                                let initial_text =
                                    Some(String::from_utf8(d_vec[0].clone().bytes()).unwrap());
                                let e_p = EditorParams {
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
                        other => {
                            eprintln!("Unexpected ReadSuccess when in state: {:?}", other);
                            self.presentation_state = other;
                        }
                    }
                } else {
                    eprintln!("Forum DType: {id}");
                }
            }
            DataType::Link => {
                //TODO: link
            }
        }
    }

    fn update_topic(&mut self, c_id: usize, header: String) {
        let t_len = self.entries.len();
        for i in t_len..=c_id {
            self.entries.push(format!("Empty {}", i));
        }
        self.entries[c_id] = header;
    }

    fn process_manifest(&mut self, d_type: DataType, d_vec: Vec<Data>) {
        eprintln!("Mfest DType: {:?}", d_type);
        let manifest = Manifest::from(d_vec);
        let mut desc = if manifest.description.is_empty() {
            format!("Manifest: no description")
        } else {
            let line: String = manifest.description.lines().take(1).collect();
            line.chars().take(64).collect()
        };
        desc = desc.trim().to_string();
        if self.entries.is_empty() {
            self.entries.push(desc);
        } else {
            self.entries[0] = desc;
        }
        self.shell.manifest = manifest;
    }
    async fn present_topics(&mut self) {
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);
        match curr_state {
            PresentationState::MainLobby(pg_opt) => {
                let pg = if let Some(pg) = pg_opt { pg } else { 0 };
                let topics = self.read_posts(pg, 10).await;
                let _ = self.to_tui_send.send(ToForumView::TopicsPage(pg, topics));
                self.presentation_state = PresentationState::MainLobby(Some(pg));
            }
            PresentationState::Topic(t_id, pg_opt) => {
                let pg = if let Some(pg) = pg_opt { pg } else { 0 };
                let topics = self.read_posts(pg, 10).await;
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
        let first_idx = (page * page_size) as usize;
        let last_idx = ((page + 1) * page_size) as usize;
        let t_len = self.entries.len();
        let mut res = Vec::with_capacity(page_size as usize);
        for i in first_idx..last_idx {
            if i < t_len {
                res.push((i as u16, self.entries[i].clone()));
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
            PresentationState::MainLobby(page_opt) => {
                let e_p = EditorParams {
                    initial_text: Some(format!("Topic desrciption")),

                    allow_newlines: true,
                    chars_limit: None,
                    text_limit: Some(800),
                    read_only: false,
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                eprintln!("We should add a new topic");
                // self.presentation_state = PresentationState::MainLobby(page_opt);
                self.presentation_state = PresentationState::Editing(
                    None,
                    Box::new(PresentationState::MainLobby(page_opt)),
                );
            }
            PresentationState::Topic(c_id, pg_opt) => {
                let e_p = EditorParams {
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
                let e_p = EditorParams {
                    initial_text: Some(format!("GID-0123456789abcdef")),

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
                        sts.push(format!("{i}"));
                    }
                    (opts, sts)
                } else {
                    let mut opts = Vec::with_capacity(256);
                    let mut sts = Vec::with_capacity(256);
                    for i in 0..=255 {
                        opts.push(i);
                        sts.push(format!("{i}"));
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
                self.process_edit_result(None).await;
                yield_now().await;
            }
            other => {
                eprintln!("Deleting not supported yet");
                self.presentation_state = other;
            }
        }
    }

    async fn process_edit_result(&mut self, text: Option<String>) {
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
                        if let Some(text) = text {
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
                        let items_per_page: usize = 10;
                        let _page_no = id as usize / items_per_page;
                        let mut presentation_elems = Vec::with_capacity(10);
                        for i in 0..items_per_page {
                            if items_per_page * _page_no + i < v_len {
                                presentation_elems.push((
                                    i as u16,
                                    vec_gid[items_per_page * _page_no + i].to_string(),
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
                        if let Some(topic_desc) = text {
                            //TODO: add new topic to AppData
                            eprintln!("We should add a new topic:\n{topic_desc}");
                            let data = Data::new(topic_desc.as_bytes().to_vec()).unwrap();
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::AppendContent(
                                    self.shell.swarm_id,
                                    DataType::Data(0),
                                    data,
                                ))
                                .await;
                        }
                    }
                    PresentationState::Settings => {
                        if let Some(text) = text {
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
                            self.present_topics().await;
                        }
                    }
                    PresentationState::Topic(c_id, pg_opt) => {
                        if let Some(text) = text {
                            let data = Data::new(text.into_bytes()).unwrap();
                            if let Some(id) = id {
                                // TODO: update existing Post
                                eprintln!("Should update {id:?}");
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::UpdateData(self.shell.swarm_id, c_id, id, data))
                                    .await;
                            } else {
                                // request append data
                                let _ = self
                                    .to_app_mgr_send
                                    .send(ToAppMgr::AppendData(self.shell.swarm_id, c_id, data))
                                    .await;
                            }
                        } else {
                            eprintln!("Ignore, empty text");
                        }

                        self.presentation_state = PresentationState::Topic(c_id, pg_opt);
                        self.present_topics().await;
                        // present topics
                    }
                    other => {
                        eprintln!("Editing not supported yet");
                        self.presentation_state = other;
                    }
                }
            }
            PresentationState::ShowingPost(c_id, _pg_id) => {
                self.presentation_state = PresentationState::Topic(c_id, None);
                self.present_topics().await;
            }
            other => {
                eprintln!("Edit result when in state: {:?}", other);
                self.presentation_state = other;
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
        // TODO:
        if ids.is_empty() {
            eprintln!("No item was selected!");
            return;
        }
        let id = ids[0];
        eprintln!("Selected ids: {:?}", ids);
        let curr_state = std::mem::replace(
            &mut self.presentation_state,
            PresentationState::MainLobby(None),
        );
        match curr_state {
            PresentationState::SelectingOneCapability(caps, prev_state) => {
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
                        let mut ccaps = Vec::with_capacity(10);
                        let entries_len = 10;
                        for (i, (cap, _v_gids)) in mapping.iter() {
                            if *i < entries_len as u16 {
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
                let p = p_vec[id];
                // let r_tree = decompose(req.clone());
                self.presentation_state = PresentationState::Pyramid(p.clone(), r_tree.clone());
                let _ = self.to_tui_send.send(ToForumView::ShowPolicy(p, r_tree));
            }
            PresentationState::SelectingOneRequirement(req_vec, pol, mut r_tree) => {
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
                let mut bbsets = Vec::with_capacity(10);
                let entries_len = 10;
                for (id, bset) in &h_map {
                    if bbsets.len() < entries_len {
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
                let page = page_opt.unwrap();
                let topic_id = id + 10 * page;
                if topic_id == 0 {
                    //TODO: open Editor with Manifest desc
                    // in READ only mode
                    let e_params = EditorParams {
                        initial_text: Some(self.shell.manifest.description.clone()),
                        allow_newlines: true,
                        chars_limit: None,
                        text_limit: Some(1000),
                        read_only: true,
                    };
                    let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_params));
                } else {
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            topic_id,
                            0,
                            10,
                        )))
                        .await;
                    self.presentation_state = PresentationState::Topic(topic_id, None);
                    eprintln!("cleaning topics…");
                    self.entries = vec![];
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
                if let Some(page_nr) = page_opt {
                    let d_id = page_nr * 10 + id;
                    let _ = self
                        .to_app_mgr_send
                        .send(ToAppMgr::FromApp(dapp_lib::LibRequest::ReadPagesRange(
                            self.shell.swarm_id,
                            *c_id,
                            d_id,
                            d_id,
                        )))
                        .await;
                    new_state = Some(PresentationState::ShowingPost(*c_id, d_id));
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
                let items_per_page = 10;
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
                        for i in 0..items_per_page {
                            if items_per_page * (*_page_no as usize) + i < v_len {
                                presentation_elems.push((
                                    i as u16,
                                    v_gid[items_per_page * (*_page_no as usize) + i].to_string(),
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
                                strings.push(format!("Pair{p}"));
                                if bset.contains_pair(&p) {
                                    preselected.push(p as usize);
                                }
                            }
                            (strings, preselected)
                        } else {
                            let mut strings = vec![];
                            let mut preselected = vec![];
                            for p in 0..=255 {
                                strings.push(format!("Byte{p}"));
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
                // let page = page_opt.unwrap();
                // similar to RunningPolicies
            }
            PresentationState::StoredPolicies(page_opt) => {
                //TODO: get Policy & Requirements & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (5)");
                    return;
                }
                // let page = page_opt.unwrap();
                // Here we should store only a mapping from on_page_id
                // to actual policy.
                // And now we should retrieve that Policy from Manifest,
                // which should be kept up-to-date.
            }
            PresentationState::StoredCapabilities(page_opt) => {
                //TODO: get Capability & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (6)");
                    return;
                }
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
                let e_p = EditorParams {
                    initial_text: Some(format!("{g_id}")),

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
    async fn store_policy(&mut self, _pol: Policy, _req: Requirement) {
        eprintln!("In store_policy");
        // TODO: update selected policy to new value.
        // we should update Manifest also at Swarm level
        //
        // We probably need a SwarmShell instance.
        // Next we need to have retrinve Manifest from Swarm.
        // Then we need to update/add given Policy.
        // Now we send requests to Gnome in order to update
        //
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
