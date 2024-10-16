use animaterm::prelude::*;
use async_std::task::sleep;
use async_std::task::spawn_blocking;
use dapp_lib::prelude::SwarmID;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::env::args;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::{channel, Sender};
use std::time::Duration;
// use std::fs;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::path::PathBuf;
mod input;
mod tui;
// use input::Input;
use tui::{instantiate_tui_mgr, serve_tui_mgr, Direction, FromPresentation, ToPresentation};

struct ApplicationLogic {
    my_id: GnomeId,
    active_swarm_id: (GnomeId, SwarmID),
    to_app_mgr_send: Sender<ToAppMgr>,
    to_tui_send: Sender<ToPresentation>,
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
            active_swarm_id: (my_id, SwarmID(0)),
            to_app_mgr_send,
            to_tui_send,
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
                    FromPresentation::NewUserEntry(text) => {
                        if !home_swarm_enforced {
                            buffered_from_tui.push(FromPresentation::NewUserEntry(text));
                            continue;
                        }
                        //TODO: we need to sync this data with Swarm
                        //TODO: we need to create logic that converts user data like String
                        //      into SyncData|CastData before we can send it to Swarm
                        let data = text_to_data(text);
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::AddContent(self.active_swarm_id.1, data));
                    }
                    FromPresentation::NeighborSelected(gnome_id) => {
                        eprintln!("Selected neighbor: {:?}", gnome_id);
                        let _ = self.to_app_mgr_send.send(ToAppMgr::SetActiveApp(gnome_id));
                    }
                    FromPresentation::KeyPress(key) => {
                        if self.handle_key(key) {
                            eprintln!("Sending ToAppMgr::Quit");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::Quit);
                            // break;
                        }
                    }
                    FromPresentation::ContentInquiry(c_id) => {
                        if !home_swarm_enforced {
                            buffered_from_tui.push(FromPresentation::ContentInquiry(c_id));
                            continue;
                        }
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::ReadData(self.active_swarm_id.1, c_id));
                    }
                }
            }
            while let Ok(msg) = self.to_app.try_recv() {
                match msg {
                    ToApp::ActiveSwarm(f_id, s_id) => {
                        if !home_swarm_enforced {
                            home_swarm_enforced = true;
                            for msg in buffered_from_tui.iter_mut() {
                                let _ = self.from_tui_send.send(msg.clone());
                            }
                        }
                        eprintln!("Set active {:?}", s_id);
                        self.active_swarm_id = (f_id, s_id);
                        if let Some(pending_notifications) =
                            self.pending_notifications.remove(&self.active_swarm_id.1)
                        {
                            for note in pending_notifications.into_iter() {
                                let _ = self.to_user_send.send(note);
                            }
                        }
                        let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors);
                    }
                    ToApp::Neighbors(s_id, neighbors) => {
                        if !home_swarm_enforced {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::SetActiveApp(self.my_id));
                        }
                        if s_id == self.active_swarm_id.1 {
                            if self.my_id == self.active_swarm_id.0 {
                                let _ = self.to_tui_send.send(ToPresentation::Neighbors(neighbors));
                            } else {
                                //TODO: here we need to insert our id as a Neighbor, and remove Swarm's founder from Neighbor list
                                let mut new_neighbors = vec![self.my_id];
                                for n in neighbors {
                                    if n == self.active_swarm_id.0 {
                                        eprintln!("Removing founder from neighbors list");
                                        continue;
                                    }
                                    new_neighbors.push(n);
                                }
                                let _ = self
                                    .to_tui_send
                                    .send(ToPresentation::Neighbors(new_neighbors));
                            }
                        } else {
                            eprintln!(
                                "Not sending neighbors, because my {:?} != {:?}",
                                self.active_swarm_id, s_id
                            );
                            self.pending_notifications
                                .entry(s_id)
                                .or_insert(vec![])
                                .push(ToApp::Neighbors(s_id, neighbors));
                        }
                    }
                    ToApp::NewContent(s_id, c_id, d_type) => {
                        if s_id == self.active_swarm_id.1 && home_swarm_enforced {
                            let _ = self
                                .to_tui_send
                                .send(ToPresentation::AddContent(c_id, d_type));
                        } else {
                            eprintln!(
                                "Not sending new content, because my {:?} != {:?}",
                                self.active_swarm_id, s_id
                            );
                            self.pending_notifications
                                .entry(s_id)
                                .or_insert(vec![])
                                .push(ToApp::NewContent(s_id, c_id, d_type));
                        }
                    }
                    ToApp::ReadResult(s_id, c_id, d_vec) => {
                        if s_id == self.active_swarm_id.1 {
                            let mut text = String::new();
                            for data in d_vec {
                                text.push_str(&String::from_utf8(data.bytes()).unwrap());
                            }
                            let _ = self.to_tui_send.send(ToPresentation::Contents(c_id, text));
                        } else {
                            eprintln!(
                                "Not sending read result, because my {:?} != {:?}",
                                self.active_swarm_id, s_id
                            );
                        }
                    }
                    ToApp::Disconnected => {
                        eprintln!("Main job is done.");
                        break 'outer;
                    }
                }
            }
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
                let _ = self.to_app_mgr_send.send(ToAppMgr::SendManifest);
            }
            Key::N => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::ListNeighbors);
            }
            Key::C => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::ChangeContent(0, Data::empty(0)));
            }
            Key::S => {
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AddContent(self.active_swarm_id.1, Data::empty(0)));
            }
            Key::ShiftS => {
                // TODO: extend this message with actual content
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AddContent(self.active_swarm_id.1, Data::empty(0)));
                // let data = vec![next_val; 1024];

                // // TODO: indirect send via AppMgr
                // let _ = service_request.send(Request::AddData(SyncData::new(data).unwrap()));
                // next_val += 1;
            }
            // Key::Left => {
            //     let _ = self.to_tui_send.send(ToTui::MoveSelection(Direction::Left));
            // }
            // Key::Right => {
            //     let _ = self
            //         .to_tui_send
            //         .send(ToTui::MoveSelection(Direction::Right));
            // }
            // Key::Up => {
            //     let _ = self.to_tui_send.send(ToTui::MoveSelection(Direction::Up));
            // }
            // Key::Down => {
            //     let _ = self.to_tui_send.send(ToTui::MoveSelection(Direction::Down));
            // }
            Key::ShiftU => {
                let _ = self.to_app_mgr_send.send(ToAppMgr::StartUnicast);
            }
            _ => eprintln!(),
        }
        false
    }
}

#[async_std::main]
async fn main() {
    let dir: String = args().nth(1).unwrap().parse().unwrap();
    let (key_send, key_recv) = channel();

    let tui_mgr = instantiate_tui_mgr();

    let config = Configuration::new(dir, 0);
    let (to_user_send, to_user_recv) = channel();
    let (to_app_mgr_send, to_app_mgr_recv) = channel();
    let my_id = initialize(
        to_user_send.clone(),
        to_app_mgr_send.clone(),
        to_app_mgr_recv,
        config,
    );
    let (to_tui_send, to_tui_recv) = channel();
    let mut logic = ApplicationLogic::new(
        my_id,
        to_app_mgr_send,
        to_tui_send,
        key_send.clone(),
        key_recv,
        to_user_send,
        to_user_recv,
    );
    let _tui_join = spawn_blocking(move || serve_tui_mgr(my_id, tui_mgr, key_send, to_tui_recv));

    // TODO: separate user input, manager input and app loop - there will be multiple
    // swarms running under single app - those should be served separately
    // each in it's own loop
    // User's input will be also served through that loop, wrapped in
    // Response::FromUser or something similar that will contain all necessary data

    // We should have an ApplicationManager
    // AppMgr will be responsible for handling input from both User and SwarmManager.
    // User will switch between Swarms in an Application or switch to entirely
    // different application type.
    // User will have only one application active at a time. Other apps will run in
    // background. Only active application will be sent user input.
    // Output to user may be sent from all applications, depending on user's config.
    // Once AppMgr receives SwarmJoined info from SwarmMgr it spawns a new task
    // that contains a loop in which Responses are handled.
    // Those responses include both messages from underlying swarm as well as
    // user's input, if given app/swarm is currently active one.
    logic.run().await;
    eprintln!("logic finished");
    // tui_join.await;
}

fn text_to_data(text: String) -> Data {
    Data::new(text.try_into().unwrap()).unwrap()
}
