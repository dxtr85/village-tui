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
enum PresentationState {
    MainLobby(Option<u16>),
    RunningPolicies(Option<(u16, HashMap<u16, (Policy, Requirement)>)>),
    RunningCapabilities(Option<u16>),
    RunningByteSets(Option<u16>),
    StoredPolicies(Option<u16>),
    StoredCapabilities(Option<u16>),
    StoredByteSets(Option<u16>),
}
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
    async fn serve_query(&mut self, id: u16) {
        eprintln!("Logic got a query {:?}", id);
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
                        //TODO: get Policy & Requirements & show it to user
                        let _ = self
                            .to_tui_send
                            .send(ToForumView::ShowPolicy(p.clone(), r.clone()));
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
