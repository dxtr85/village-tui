use crate::common::poledit::decompose;
use crate::common::poledit::PolAction;
use crate::common::poledit::ReqTree;
use crate::forum::tui::EditorParams;
use animaterm::prelude::*;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use async_std::task::sleep;
use async_std::task::spawn;
use async_std::task::spawn_blocking;
use async_std::task::yield_now;
use dapp_lib::prelude::ByteSet;
use dapp_lib::prelude::Capabilities;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::Policy;
use dapp_lib::prelude::Requirement;
use dapp_lib::prelude::SwarmName;
use dapp_lib::ToApp;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

use dapp_lib::prelude::AppType;

#[derive(Debug)]
enum PresentationState {
    MainLobby(Option<u16>),
    Settings,
    RunningPolicies(Option<(u16, HashMap<u16, (Policy, Requirement)>)>),
    RunningCapabilities(Option<(u16, HashMap<u16, (Capabilities, Vec<GnomeId>)>)>),
    RunningByteSets(Option<(u16, HashMap<u8, ByteSet>)>),
    StoredPolicies(Option<u16>),
    StoredCapabilities(Option<u16>),
    StoredByteSets(Option<u16>),
    SelectingOnePolicy(Vec<Policy>, ReqTree),
    SelectingOneRequirement(Vec<Requirement>, Policy, ReqTree),
    Pyramid(Policy, ReqTree),
    Capability(Capabilities, Vec<GnomeId>),
    SelectingOneCapability(Vec<Capabilities>, Box<PresentationState>),
    Editing(Option<u16>, Box<PresentationState>),
}
// use crate::common::poledit::PolAction;
use crate::forum::tui::serve_forum_tui;
use crate::forum::tui::Action;
use crate::forum::tui::FromForumView;
use crate::forum::tui::ToForumView;
use crate::InternalMsg;
use crate::Toolset;
pub struct ForumLogic {
    swarm_name: SwarmName,
    presentation_state: PresentationState,
    to_app_mgr_send: ASender<ToAppMgr>,
    to_user_send: ASender<InternalMsg>,
    to_user_recv: AReceiver<InternalMsg>,
    to_tui_send: Sender<ToForumView>,
    to_tui_recv: Option<Receiver<ToForumView>>,
    from_tui_send: Option<Sender<FromForumView>>,
}
impl ForumLogic {
    pub fn new(
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
        ForumLogic {
            presentation_state: PresentationState::MainLobby(None),
            swarm_name,
            to_app_mgr_send,
            to_user_send,
            to_user_recv,
            to_tui_send,
            to_tui_recv: Some(to_tui_recv),
            from_tui_send: Some(from_tui_send),
        }
    }
    pub async fn run(
        mut self,
        founder: GnomeId,
        config_dir: PathBuf,
        toolset: Toolset,
        // mut config: Configuration,
        // mut tui_mgr: Manager,
        // ) -> Option<(AppType, AReceiver<InternalMsg>, Configuration, Manager)> {
    ) -> Option<(AppType, SwarmName, AReceiver<InternalMsg>, Toolset)> {
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

        loop {
            let int_msg_res = self.to_user_recv.recv().await;
            if int_msg_res.is_err() {
                eprintln!("Forum error recv internal: {}", int_msg_res.err().unwrap());
                break;
            }
            let msg = int_msg_res.unwrap();
            match msg {
                InternalMsg::User(to_app) => match to_app {
                    ToApp::ActiveSwarm(s_name, s_id) => {
                        eprintln!("Forum: ToApp::ActiveSwarm({s_id},{s_name})");
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
                        self.presentation_state =
                            PresentationState::RunningByteSets(Some((bs_page, mapping)));
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::RunningByteSetsPage(bs_page, bbsets));
                    }
                    _other => {
                        eprintln!("InternalMsg::User");
                    }
                },
                InternalMsg::Forum(from_tui) => {
                    // eprintln!("InternalMsg::Forum");
                    match from_tui {
                        FromForumView::Act(action) => {
                            match action {
                                Action::Settings => {
                                    self.presentation_state = PresentationState::Settings;
                                }
                                Action::RunningPolicies => {
                                    self.presentation_state =
                                        PresentationState::RunningPolicies(None);
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::FromApp(
                                            dapp_lib::LibRequest::RunningPolicies(
                                                self.swarm_name.clone(),
                                            ),
                                        ))
                                        .await;
                                }
                                Action::StoredPolicies => {
                                    self.presentation_state =
                                        PresentationState::StoredPolicies(None);
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
                                    self.presentation_state =
                                        PresentationState::RunningCapabilities(None);
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::FromApp(
                                            dapp_lib::LibRequest::RunningCapabilities(
                                                self.swarm_name.clone(),
                                            ),
                                        ))
                                        .await;
                                }
                                Action::StoredCapabilities => {
                                    //TODO:
                                    self.presentation_state =
                                        PresentationState::StoredCapabilities(None);
                                }
                                Action::RunningByteSets => {
                                    //TODO:
                                    self.presentation_state =
                                        PresentationState::RunningCapabilities(None);
                                    let _ = self
                                        .to_app_mgr_send
                                        .send(ToAppMgr::FromApp(
                                            dapp_lib::LibRequest::RunningByteSets(
                                                self.swarm_name.clone(),
                                            ),
                                        ))
                                        .await;
                                }
                                Action::StoredByteSets => {
                                    //TODO:
                                    self.presentation_state =
                                        PresentationState::StoredByteSets(None);
                                }
                                Action::AddNew => {
                                    self.add_new_action().await;
                                }
                                Action::Delete(id) => {
                                    self.delete_action(id).await;
                                }
                                Action::Run => {
                                    self.run_action().await;
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
                                Action::Filter(filter) => {
                                    //TODO
                                }
                                Action::Query(qt) => {
                                    self.serve_query(qt).await;
                                }
                                Action::MainMenu => {
                                    //TODO
                                }
                                Action::Posts(topic_id) => {
                                    //TODO
                                }
                                Action::PolicyAction(p_act) => {
                                    self.serve_pol_action(p_act).await;
                                }
                                Action::OneSelected(id) => {
                                    self.serve_one_selected(id).await;
                                }
                                Action::EditorResult(str_o) => self.process_edit_result(str_o),
                            }
                        }
                        // FromForumView::RunningPolicies => {
                        //     eprintln!("Should send running policies");
                        // }
                        // FromForumView::StoredPolicies => {
                        //     eprintln!("Should send stored policies");
                        // }
                        FromForumView::Quit => {
                            break;
                        }
                        FromForumView::SwitchTo(app_type, s_name) => {
                            switch_to_opt = Some((app_type, s_name));
                            break;
                        }
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
            Some((switch_app, s_name, self.to_user_recv, toolset))
        } else {
            toolset.discard();
            None
        }
    }
    async fn add_new_action(&mut self) {
        let curr_state =
            std::mem::replace(&mut self.presentation_state, PresentationState::Settings);

        // Remember to always update self.presentation_state!
        //
        match curr_state {
            PresentationState::Capability(c, v_gid) => {
                let e_p = EditorParams {
                    initial_text: Some(format!("GID-0123456789abcdef")),

                    allow_newlines: false,
                    chars_allowed: Some("0123456789abcdef".chars().collect()),
                    text_limit: Some(20),
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
                let _ = self.to_tui_send.send(ToForumView::SelectOne(avail_cstrs));
                self.presentation_state = PresentationState::SelectingOneCapability(
                    avail_caps,
                    Box::new(PresentationState::RunningCapabilities(old_caps_opt)),
                );
                yield_now().await;
            }
            other => {
                eprintln!("Adding not supported yet");
                self.presentation_state = other;
            }
        }
    }
    async fn run_action(&self) {
        match &self.presentation_state {
            PresentationState::Capability(c, v_gids) => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::FromApp(
                        dapp_lib::LibRequest::SetRunningCapability(
                            self.swarm_name.clone(),
                            *c,
                            v_gids.clone(),
                        ),
                    ))
                    .await;
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
                self.process_edit_result(None);
                yield_now().await;
            }
            other => {
                eprintln!("Deleting not supported yet");
                self.presentation_state = other;
            }
        }
    }

    fn process_edit_result(&mut self, text: Option<String>) {
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
                    other => {
                        eprintln!("Editing not supported yet");
                        self.presentation_state = other;
                    }
                }
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
                if let PresentationState::Pyramid(p, r) = curr_state {
                    let (policies, p_strings) = Policy::mapping();
                    let _ = self.to_tui_send.send(ToForumView::SelectOne(p_strings));
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
                if let PresentationState::Pyramid(p, r) = curr_state {
                    let user_def_caps: Vec<u8> = vec![0];
                    let byte_sets: Vec<u8> = vec![0, 1, 2];
                    let two_byte_sets: Vec<u8> = vec![3];
                    let incl_logic = r_tree.mark_location().0 != 4;
                    let (reqs, r_strings) =
                        Requirement::mapping(incl_logic, user_def_caps, byte_sets, two_byte_sets);
                    self.presentation_state =
                        PresentationState::SelectingOneRequirement(reqs, p, r_tree);
                    let _ = self.to_tui_send.send(ToForumView::SelectOne(r_strings));
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
    async fn serve_one_selected(&mut self, id: usize) {
        // TODO:
        eprintln!("OneSelected id: {id}");
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
            other => {
                self.presentation_state = other;
            }
        }
    }
    async fn serve_query(&mut self, id: u16) {
        eprintln!("Logic got a query {:?}", id);
        let mut new_state = None;
        match &self.presentation_state {
            PresentationState::MainLobby(page_opt) => {
                //TODO: retrieve a Topic and present it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (1)");
                    return;
                }
                let page = page_opt.unwrap();
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
            PresentationState::Settings => {
                // TODO: do we need to do anything here?
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
            PresentationState::RunningByteSets(page_opt) => {
                //TODO: get ByteSet & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (4)");
                    return;
                }
                eprintln!("Got non-empty BSets page");
                // let page = page_opt.unwrap();
                // similar to RunningPolicies
            }
            PresentationState::StoredPolicies(page_opt) => {
                //TODO: get Policy & Requirements & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (5)");
                    return;
                }
                let page = page_opt.unwrap();
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
                let page = page_opt.unwrap();
                // similar to StoredPolicies
            }
            PresentationState::StoredByteSets(page_opt) => {
                //TODO: get ByteSet & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (7)");
                    return;
                }
                let page = page_opt.unwrap();
                // similar to StoredPolicies
            }
            PresentationState::SelectingOnePolicy(pol_vec, req) => {
                //TODO
                eprintln!("serve queue while in SelectingOnePolicy");
            }
            PresentationState::SelectingOneRequirement(req_vec, pol, r_tree) => {
                eprintln!("serve queue while in SelectingOneRequirement");
            }
            PresentationState::Pyramid(pol, req) => {
                eprintln!("serve queue while in Pyramid");
            }
            PresentationState::Capability(c, v_gid) => {
                eprintln!("TODO: We should open an Editor with Selected GID");
                // new_state = Some(PresentationState::Capability(c.clone(), v_gid.clone()));

                let g_id = v_gid[id as usize];
                let e_p = EditorParams {
                    initial_text: Some(format!("{g_id}")),

                    allow_newlines: false,
                    chars_allowed: Some("0123456789abcdef".chars().collect()),
                    text_limit: Some(20),
                };
                let _ = self.to_tui_send.send(ToForumView::OpenEditor(e_p));
                new_state = Some(PresentationState::Editing(
                    Some(id),
                    Box::new(PresentationState::Capability(*c, v_gid.clone())),
                ));
            }
            PresentationState::Editing(id, _prev_state) => {
                eprintln!("Got ID when in state Editing");
            }
            PresentationState::SelectingOneCapability(_v_caps, _p_state) => {
                eprintln!("Got ID when in state SelectingOneCapability");
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
                self.swarm_name.clone(),
                pol,
                req,
            )))
            .await;
    }
    async fn store_policy(&mut self, pol: Policy, req: Requirement) {
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
