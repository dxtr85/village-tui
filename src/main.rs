use animaterm::prelude::*;
use async_std::task::sleep;
use async_std::task::spawn;
use dapp_lib::prelude::*;
use std::collections::HashMap;
// use std::borrow::Borrow;
use std::env::args;
use std::sync::mpsc::channel;
// use std::fs;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

fn manifest() -> ApplicationManifest {
    let mut header: [u8;511]=[0;511];
    let mut i = 0;
    for byte in "Catalog".bytes(){
        header[i]=byte;
        i+=1;
    }
    
    ApplicationManifest::new(header,HashMap::new())
}

#[async_std::main]
async fn main() {
    let mut app_data = Application::empty();
    let dir: String = args().nth(1).unwrap().parse().unwrap();
    let (key_send, key_recv) = channel();
    spawn(serve_tui_mgr(key_send));
    let (gmgr_send, gmgr_recv) = init(dir, app_data.root_hash());
    let mut next_val = 1;
    let man_resp_result = gmgr_recv.recv();
    let service_request;
    let (app_send, app_recv) = channel();
    let mut swarm_id = SwarmID(0);
    if let Ok(ManagerResponse::SwarmJoined(s_id, _s_name, service_req, service_resp)) =
        man_resp_result
    {
        service_request = service_req;
        swarm_id = s_id;
        spawn(serve_user_responses(
            Duration::from_millis(30),
            service_resp,
            app_send.clone(),
        ));
    } else {
        return;
    }

    loop {
        if let Ok(resp) = app_recv.try_recv() {
            match resp {
                Response::Block(_id, data) => {
                    let b_type = data.first_byte();
                    println!("Received block type: {}", b_type);
                    if b_type == 0 {
                        let manifest = ApplicationManifest::from(data);
                        app_data = Application::new(manifest);
                        let hash = app_data.root_hash();
                        println!("Sending updated hash: {}", hash);
                        // let res = gmgr_send.send(ManagerRequest::UpdateAppRootHash(swarm_id, hash));
                        let res = service_request.send(Request::UpdateAppRootHash(hash));
                        println!("Send res: {:?}", res);
                        // }else if b_type == 1{
                    }
                }
                Response::AppDataSynced(is_synced) => {
                    println!(
                        "AppDataSynced: {}, hash: {}",
                        is_synced,
                        app_data.root_hash()
                    );
                }
                Response::AppSync(sync_type, c_id, part_no, total, data) => {
                    println!("Got AppSync response {}", sync_type);

                    match sync_type {
                        0 => {
                            for chunk in data.bytes().chunks(8) {
                                let hash = u64::from_be_bytes(chunk[0..8].try_into().unwrap());
                                let tree = ContentTree::empty(hash);
                                let content = Content::Data(sync_type, tree);
                                let res = app_data.append(content);
                                println!("Datastore add: {:?}", res);
                                // let _ = service_request.send(Request::AskData(
                                //     gnome_id,
                                //     NeighborRequest::AppSyncRequest(1, data),
                                // ));
                            }
                        }
                        1 => {
                            println!("Content {} add part {} of {}", c_id, part_no, total);
                            if c_id == 0 {
                                println!("App manifest to add");
                                let content = Content::Data(0, ContentTree::Filled(data));
                                let res = app_data.update(0, content);
                                println!("App manifest add result: {:?}", res);
                            }
                        }
                        _ => {
                            //TODO
                        }
                    }
                }
                Response::AppSyncInquiry(gnome_id, sync_type, _data) => {
                    println!("Got AppSync inquiry");
                    let hashes = app_data.datastore_bottom_hashes();
                    let mut byte_hashes = vec![];
                    for hash in hashes.iter() {
                        for byte in hash.to_be_bytes() {
                            byte_hashes.push(byte);
                        }
                    }
                    let c_id = 0;
                    let part_no = 0;
                    let total = 0;
                    let _ = service_request.send(Request::SendData(
                        gnome_id,
                        NeighborResponse::AppSync(
                            sync_type,
                            c_id,
                            part_no,
                            total,
                            Data::new(byte_hashes).unwrap(),
                        ),
                    ));
                    println!("Sent Datastore response");

                    if let Ok(data_vec) = app_data.get_all_data(0) {
                        let total = data_vec.len();
                        for (part_no, data) in data_vec.into_iter().enumerate() {
                            let _ = service_request.send(Request::SendData(
                                gnome_id,
                                NeighborResponse::AppSync(
                                    1,
                                    0,
                                    part_no as u16,
                                    total as u16 - 1,
                                    data,
                                ),
                            ));
                        }
                        println!("Sent CID response");
                    }
                }
                _ => {
                    println!("Unserved by app: {:?}", resp);
                }
            }
        }
        if let Ok(key) = key_recv.try_recv() {
            // println!("some key");
            match key {
                Key::J => {
                    let _ = gmgr_send.send(ManagerRequest::JoinSwarm("trzat".to_string()));
                }
                Key::Q | Key::ShiftQ => {
                    let _ = gmgr_send.send(ManagerRequest::Disconnect);
                    // keep_running = false;
                    break;
                }
                // TODO: this should be served separately by sending to user_req
                Key::B => {
                    let _ = service_request.send(Request::StartBroadcast);
                }
                Key::M => {
                    let _ = service_request
                        .send(Request::AddData(manifest().to_data(Some(0))));
                }
                Key::N => {
                    let _ = service_request.send(Request::ListNeighbors);
                }
                Key::S => {
                    let _ =
                        service_request.send(Request::AddData(Data::new(vec![next_val]).unwrap()));
                    next_val += 1;
                }
                Key::ShiftS => {
                    let data = vec![next_val; 1024];

                    let _ = service_request.send(Request::AddData(Data::new(data).unwrap()));
                    next_val += 1;
                }
                Key::ShiftU => {
                    let res =
                        service_request.send(Request::StartUnicast(GnomeId(15561580566906229863)));
                    println!("UnicastReq: {:?}", res);
                    // next_val += 1;
                }
                _ => println!(),
            }
        }
        if let Ok(gnome_response) = gmgr_recv.try_recv() {
            match gnome_response {
                ManagerResponse::SwarmJoined(_swarm_id, _swarm_name, _user_req, user_res) => {
                    // TODO: serve user_req
                    let sleep_time = Duration::from_millis(30);
                    spawn(serve_user_responses(sleep_time, user_res, app_send.clone()));
                }
            }
        }
    }
}

async fn serve_user_responses(
    sleep_time: Duration,
    user_res: Receiver<Response>,
    to_app: Sender<Response>,
) {
    loop {
        let data = user_res.try_recv();
        if let Ok(resp) = data {
            // println!("SUR: {:?}", resp);
            match resp {
                Response::AppDataSynced(_synced) => {
                    let _ = to_app.send(resp);
                }
                Response::Broadcast(_s_id, c_id, recv_d) => {
                    spawn(serve_broadcast(c_id, Duration::from_millis(100), recv_d));
                }
                Response::Unicast(_s_id, c_id, recv_d) => {
                    spawn(serve_unicast(c_id, Duration::from_millis(100), recv_d));
                }
                Response::BroadcastOrigin(_s_id, c_id, send_d) => {
                    spawn(serve_broadcast_origin(
                        c_id,
                        Duration::from_millis(200),
                        send_d,
                    ));
                }
                Response::UnicastOrigin(_s_id, c_id, send_d) => {
                    spawn(serve_unicast_origin(
                        c_id,
                        Duration::from_millis(500),
                        send_d,
                    ));
                }
                _ => {
                    let _ = to_app.send(resp);
                }
            }
        } else {
            // println!("{:?}", data);
        }
        sleep(sleep_time).await;
    }
}
async fn serve_unicast(c_id: CastID, sleep_time: Duration, user_res: Receiver<Data>) {
    println!("Serving unicast {:?}", c_id);
    loop {
        let recv_res = user_res.try_recv();
        if let Ok(data) = recv_res {
            println!("U{:?}: {}", c_id, data);
        }
        sleep(sleep_time).await;
    }
}
async fn serve_tui_mgr(to_app: Sender<Key>) {
    println!("Serving TUI Manager");
    let capture_keyboard = true;
    let cols = Some(40);
    let rows = None; // use all rows available
    let glyph = Some(Glyph::default());
    let refresh_timeout = Some(Duration::from_millis(10));
    let mut mgr = Manager::new(capture_keyboard, cols, rows, glyph, refresh_timeout);
    loop {
        let recv_res = mgr.read_key();
        if let Some(key) = recv_res {
            let terminate = key == Key::Q || key == Key::ShiftQ;
            let res = to_app.send(key);
            if res.is_err() || terminate {
                break;
            }
        }
    }
    mgr.terminate();
}
async fn serve_broadcast(c_id: CastID, sleep_time: Duration, user_res: Receiver<Data>) {
    println!("Serving broadcast {:?}", c_id);
    loop {
        let recv_res = user_res.try_recv();
        if let Ok(data) = recv_res {
            println!("B{:?}: {}", c_id, data);
        }
        sleep(sleep_time).await;
    }
}
async fn serve_unicast_origin(c_id: CastID, sleep_time: Duration, user_res: Sender<Data>) {
    println!("Originating unicast {:?}", c_id);
    let mut i: u8 = 0;
    loop {
        let send_res = user_res.send(Data::new(vec![i]).unwrap());
        if send_res.is_ok() {
            println!("Unicasted {}", i);
        } else {
            println!(
                "Error while trying to unicast: {:?}",
                send_res.err().unwrap()
            );
        }
        i = i.wrapping_add(1);

        sleep(sleep_time).await;
    }
}
async fn serve_broadcast_origin(c_id: CastID, sleep_time: Duration, user_res: Sender<Data>) {
    println!("Originating broadcast {:?}", c_id);
    let mut i: u8 = 0;
    loop {
        let send_res = user_res.send(Data::new(vec![i]).unwrap());
        if send_res.is_ok() {
            println!("Broadcasted {}", i);
        } else {
            println!(
                "Error while trying to broadcast: {:?}",
                send_res.err().unwrap()
            );
        }
        i = i.wrapping_add(1);

        sleep(sleep_time).await;
    }
}
