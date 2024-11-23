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
pub use manifest::Manifest;
pub use manifest::Tag;
// use std::fs;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::path::PathBuf;
// use input::Input;
use crate::tui::{FromPresentation, ToPresentation};

struct SwarmShell {
    swarm_id: SwarmID,
    founder_id: GnomeId,
    manifest: Manifest,
}

pub struct ApplicationLogic {
    my_id: GnomeId,
    active_swarm: SwarmShell,
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
            active_swarm: SwarmShell {
                swarm_id: SwarmID(0),
                founder_id: my_id,
                manifest: Manifest::new(AppType::Catalog, HashMap::new()),
            },
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
        let mut should_send_manifest = false;

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
                        let _ = self.to_app_mgr_send.send(ToAppMgr::AppendContent(
                            self.active_swarm.swarm_id,
                            DataType::from(0),
                            data,
                        ));
                    }
                    FromPresentation::AddTags(tags) => {
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
                        };
                    }
                    FromPresentation::AddDataType(tag) => {
                        // TODO: first check if we can add a tag for given swarm
                        // and also check if given tag is not already added
                        //TODO: we need to temporarily add given tag to manifest,
                        // calculate hashes, and request app manager to modify or add Data
                        // blocks to datastore for given swarm
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
                        };
                    }
                    FromPresentation::NeighborSelected(gnome_id) => {
                        eprintln!("Selected neighbor: {:?}", gnome_id);
                        let _ = self.to_app_mgr_send.send(ToAppMgr::SetActiveApp(gnome_id));
                    }
                    FromPresentation::KeyPress(key) => {
                        if self.handle_key(key) {
                            // eprintln!("Sending ToAppMgr::Quit");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::Quit);
                            // break;
                        }
                    }
                    FromPresentation::ContentInquiry(c_id) => {
                        if !home_swarm_enforced {
                            buffered_from_tui.push(FromPresentation::ContentInquiry(c_id));
                            continue;
                        }
                        should_send_manifest = true;
                        let _ = self
                            .to_app_mgr_send
                            .send(ToAppMgr::ReadData(self.active_swarm.swarm_id, c_id));
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
                        // let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, 0));
                    }
                    ToApp::Neighbors(s_id, neighbors) => {
                        if !home_swarm_enforced {
                            let _ = self
                                .to_app_mgr_send
                                .send(ToAppMgr::SetActiveApp(self.my_id));
                        }
                        if s_id == self.active_swarm.swarm_id {
                            if self.my_id == self.active_swarm.founder_id {
                                let _ = self.to_tui_send.send(ToPresentation::Neighbors(neighbors));
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
                                let _ = self
                                    .to_tui_send
                                    .send(ToPresentation::Neighbors(new_neighbors));
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
                                .to_tui_send
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
                            let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(s_id, c_id));
                        }
                        if s_id == self.active_swarm.swarm_id && home_swarm_enforced {
                            eprintln!("recv ToApp::ContentChanged({:?})", c_id);
                            // let _ = self
                            //     .to_tui_send
                            //     .send(ToPresentation::AppendContent(c_id, d_type));
                        } else {
                            eprintln!(
                                "Not sending changed content, because my {:?} != {:?}",
                                self.active_swarm.swarm_id, s_id
                            );
                            // self.pending_notifications
                            //     .entry(s_id)
                            //     .or_insert(vec![])
                            //     .push(ToApp::ContentChanged(s_id, c_id, d_type));
                        }
                    }
                    ToApp::ReadSuccess(s_id, c_id, d_vec) => {
                        if s_id == self.active_swarm.swarm_id {
                            if c_id == 0 {
                                if should_send_manifest {
                                    eprintln!("Should show manifest, {}", d_vec.len());
                                    should_send_manifest = false;
                                    let manifest = Manifest::from(d_vec);
                                    self.active_swarm.manifest = manifest.clone();
                                    let _ =
                                        self.to_tui_send.send(ToPresentation::Manifest(manifest));
                                }
                            } else {
                                let mut text = String::new();
                                for data in d_vec {
                                    text.push_str(&String::from_utf8(data.bytes()).unwrap());
                                }
                                let _ = self.to_tui_send.send(ToPresentation::Contents(c_id, text));
                            }
                        } else {
                            eprintln!(
                                "Not sending read result, because my {:?} != {:?}",
                                self.active_swarm.swarm_id, s_id
                            );
                        }
                    }
                    ToApp::ReadError(s_id, c_id, error) => {
                        eprintln!("Received ReadError for {:?} {} {}", s_id, c_id, error);
                        if s_id == self.active_swarm.swarm_id {
                            if c_id == 0 && self.my_id == self.active_swarm.founder_id {
                                // let _ = self
                                //     .to_tui_send
                                //     .send(ToPresentation::QueryForManifestDefinition);
                            } else {
                                let _ = self
                                    .to_tui_send
                                    .send(ToPresentation::ContentsNotExist(c_id));
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
fn text_to_data(text: String) -> Data {
    Data::new(text.try_into().unwrap()).unwrap()
}
