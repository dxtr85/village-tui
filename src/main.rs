use animaterm::prelude::*;
use async_std::task::sleep;
use async_std::task::spawn_blocking;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
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
use tui::{instantiate_tui_mgr, serve_tui_mgr, Direction, FromTui, ToTui};

struct ApplicationLogic {
    to_app_mgr_send: Sender<ToAppMgr>,
    to_tui_send: Sender<ToTui>,
    from_tui_recv: Receiver<FromTui>,
    to_user_recv: Receiver<ToUser>,
}
impl ApplicationLogic {
    pub fn new(
        to_app_mgr_send: Sender<ToAppMgr>,
        to_tui_send: Sender<ToTui>,
        from_tui_recv: Receiver<FromTui>,
        to_user_recv: Receiver<ToUser>,
    ) -> Self {
        ApplicationLogic {
            to_app_mgr_send,
            to_tui_send,
            from_tui_recv,
            to_user_recv,
        }
    }
    pub async fn run(&self) {
        let dur = Duration::from_millis(32);
        'outer: loop {
            sleep(dur).await;
            if let Ok(from_tui) = self.from_tui_recv.try_recv() {
                match from_tui {
                    FromTui::NewUserEntry(text) => {
                        //TODO: we need to sync this data with Swarm
                        //TODO: we need to create logic that converts user data like String
                        //      into SyncData|CastData before we can send it to Swarm
                        let data = text_to_data(text);
                        let _ = self.to_app_mgr_send.send(ToAppMgr::AddContent(data));
                    }
                    FromTui::KeyPress(key) => {
                        if self.handle_key(key) {
                            eprintln!("Sending ToAppMgr::Quit");
                            let _ = self.to_app_mgr_send.send(ToAppMgr::Quit);
                            // break;
                        }
                    }
                    FromTui::ContentInquiry(c_id) => {
                        let _ = self.to_app_mgr_send.send(ToAppMgr::ReadData(c_id));
                    }
                }
            }
            while let Ok(to_user) = self.to_user_recv.try_recv() {
                match to_user {
                    ToUser::Neighbors(s_name, neighbors) => {
                        let _ = self.to_tui_send.send(ToTui::Neighbors(s_name, neighbors));
                    }
                    ToUser::NewContent(c_id, d_type) => {
                        let _ = self.to_tui_send.send(ToTui::AddContent(c_id, d_type));
                    }
                    ToUser::ReadResult(c_id, d_vec) => {
                        let mut text = String::new();
                        for data in d_vec {
                            text.push_str(&String::from_utf8(data.bytes()).unwrap());
                        }
                        let _ = self.to_tui_send.send(ToTui::Contents(c_id, text));
                    }
                    ToUser::Disconnected => {
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
                    .send(ToAppMgr::AddContent(Data::empty(0)));
            }
            Key::ShiftS => {
                // TODO: extend this message with actual content
                let _ = self
                    .to_app_mgr_send
                    .send(ToAppMgr::AddContent(Data::empty(0)));
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
    let (to_app_mgr_send, to_user_recv) = initialize(config);
    let (to_tui_send, to_tui_recv) = channel();
    let logic = ApplicationLogic::new(to_app_mgr_send, to_tui_send, key_recv, to_user_recv);
    let _tui_join = spawn_blocking(|| serve_tui_mgr(tui_mgr, key_send, to_tui_recv));

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
