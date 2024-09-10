use animaterm::prelude::*;
// use async_std::task::sleep;
use async_std::task::spawn;
use async_std::task::spawn_blocking;
use async_std::task::yield_now;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
// use std::collections::HashMap;
// use std::collections::HashSet;
use std::env::args;
use std::sync::mpsc::channel;
// use std::fs;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::Duration;

#[async_std::main]
async fn main() {
    let dir: String = args().nth(1).unwrap().parse().unwrap();
    let (key_send, key_recv) = channel();

    let tui_mgr = instantiate_tui_mgr();
    spawn_blocking(|| serve_tui_mgr(tui_mgr, key_send));

    let config = Configuration::new(dir, 0);
    let (to_app_mgr_send, to_user_recv) = initialize(config);
    // let (gmgr_send, gmgr_recv) = init(dir, app_data.root_hash());
    // let mut next_val = 1;
    // let man_resp_result = gmgr_recv.recv();
    // let service_request;
    // let (app_send, app_recv) = channel();
    // let mut swarm_id = SwarmID(0);
    // if let Ok(ManagerResponse::SwarmJoined(_s_id, _s_name, service_req, service_resp)) =
    //     man_resp_result
    // {
    //     service_request = service_req;
    //     // swarm_id = s_id;
    //     spawn(serve_user_responses(
    //         Duration::from_millis(30),
    //         service_resp,
    //         app_send.clone(),
    //     ));
    // } else {
    //     return;
    // }

    // TODO: separate user input, manager input and app loop - there will be multiple
    // swarms running under single app - those should be served separately
    // each in it's own loop
    // User's input will be also served through that loop, wrapped in
    // Response::FromUser or something similar that will contain all necessary data

    // Whe should have an ApplicationManager
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
    loop {
        yield_now().await;
        if let Ok(key) = key_recv.try_recv() {
            // println!("some key");
            match key {
                Key::U => {
                    let _ = to_app_mgr_send.send(ToAppMgr::UploadData);
                }
                Key::J => {
                    // TODO: indirect send via AppMgr
                    // let _ = gmgr_send.send(ManagerRequest::JoinSwarm("trzat".to_string()));
                }
                Key::Q | Key::ShiftQ => {
                    // TODO: indirect send via AppMgr
                    // let _ = gmgr_send.send(ManagerRequest::Disconnect);
                    // keep_running = false;
                    break;
                }
                // TODO: this should be served separately by sending to user_req
                Key::B => {
                    let _ = to_app_mgr_send.send(ToAppMgr::StartBroadcast);
                    // let res = service_request.send(Request::StartBroadcast);
                    // b_req_sent = res.is_ok();
                }
                Key::M => {
                    let _ = to_app_mgr_send.send(ToAppMgr::SendManifest);
                }
                Key::N => {
                    let _ = to_app_mgr_send.send(ToAppMgr::ListNeighbors);
                    // let _ = service_request.send(Request::ListNeighbors);
                }
                Key::C => {
                    let _ = to_app_mgr_send.send(ToAppMgr::ChangeContent);
                }
                Key::S => {
                    let _ = to_app_mgr_send.send(ToAppMgr::AddContent);
                }
                Key::ShiftS => {
                    // TODO: extend this message with actual content
                    let _ = to_app_mgr_send.send(ToAppMgr::AddContent);
                    // let data = vec![next_val; 1024];

                    // // TODO: indirect send via AppMgr
                    // let _ = service_request.send(Request::AddData(SyncData::new(data).unwrap()));
                    // next_val += 1;
                }
                Key::ShiftU => {
                    let _ = to_app_mgr_send.send(ToAppMgr::StartUnicast);
                }
                _ => println!(),
            }
        }

        // // TODO: this should be handled somewhere else
        // // THis should start another main loop
        // if let Ok(gnome_response) = gmgr_recv.try_recv() {
        //     match gnome_response {
        //         ManagerResponse::SwarmJoined(_swarm_id, _swarm_name, _user_req, user_res) => {
        //             // TODO: serve user_req
        //             let sleep_time = Duration::from_millis(30);
        //             spawn(serve_user_responses(sleep_time, user_res, app_send.clone()));
        //         }
        //     }
        // }
    }
}

fn instantiate_tui_mgr() -> Manager {
    let capture_keyboard = true;
    let cols = Some(40);
    let rows = None; // use all rows available
    let glyph = Some(Glyph::default());
    let refresh_timeout = Some(Duration::from_millis(10));
    Manager::new(
        capture_keyboard,
        cols,
        rows,
        glyph,
        refresh_timeout,
        Some(vec![(Key::AltM, MacroSequence::empty())]),
    )
}
fn serve_tui_mgr(mut mgr: Manager, to_app: Sender<Key>) {
    println!("Serving TUI Manager");
    loop {
        let key = mgr.read_key();
        let terminate = key == Key::Q || key == Key::ShiftQ;
        let res = to_app.send(key);
        if res.is_err() || terminate {
            break;
        }
    }
    mgr.terminate();
}
