use crate::common::poledit::decompose;
use crate::common::poledit::ReqTree;
use animaterm::prelude::*;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use async_std::task::sleep;
use async_std::task::spawn;
use async_std::task::spawn_blocking;
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
    RunningPolicies(Option<(u16, HashMap<u16, (Policy, Requirement)>)>),
    RunningCapabilities(Option<u16>),
    RunningByteSets(Option<u16>),
    StoredPolicies(Option<u16>),
    StoredCapabilities(Option<u16>),
    StoredByteSets(Option<u16>),
    SelectingOnePolicy(Vec<Policy>, ReqTree),
    SelectingOneRequirement(Vec<Requirement>, Policy, ReqTree),
    Pyramid(Policy, ReqTree),
}
use crate::common::poledit::PolAction;
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
                        //TODO: take running policies from
                        // underlying Swarm
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
                    _other => {
                        eprintln!("InternalMsg::User");
                    }
                },
                InternalMsg::Forum(from_tui) => {
                    // eprintln!("InternalMsg::Forum");
                    match from_tui {
                        FromForumView::Act(action) => {
                            match action {
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
                                }
                                Action::StoredByteSets => {
                                    //TODO:
                                    self.presentation_state =
                                        PresentationState::StoredByteSets(None);
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
                                Action::Topics => {
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
                // TODO: update selected policy to new value.
                // If it is a running policy,
                // send it to Gnome for uploading to swarm.
                // If it was a stored Policy, we should
                // update Manifest also at Swarm level
                //
                // TODO: allow for changing both
                // running & stored Policy at once
            }
            PolAction::Run(p, r) => {
                //TODO
                eprintln!("PolAction::Run");
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
            PresentationState::RunningCapabilities(page_opt) => {
                //TODO: get Capability & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (3)");
                    return;
                }
                let page = page_opt.unwrap();
                // similar to RunningPolicies
            }
            PresentationState::RunningByteSets(page_opt) => {
                //TODO: get ByteSet & show it to user
                if page_opt.is_none() {
                    eprintln!("Unable to tell which page is shown (4)");
                    return;
                }
                let page = page_opt.unwrap();
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
        }
        if let Some(n_s) = new_state {
            self.presentation_state = n_s;
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
