use animaterm::prelude::*;
use async_std::task::sleep;
use async_std::task::spawn;
use dapp_lib::prelude::*;
use std::collections::HashMap;
use std::env::args;
use std::sync::mpsc::channel;
// use std::fs;
// use std::net::IpAddr;
// use std::net::Ipv4Addr;
// use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

fn manifest() -> ApplicationManifest {
    let mut header: [u8; 495] = [0; 495];
    for (i, byte) in "Catalog".bytes().enumerate() {
        header[i] = byte;
    }

    ApplicationManifest::new(header, HashMap::new())
}

struct BigChunk(u8, u16);
impl BigChunk {
    fn next(&mut self) -> Option<Data> {
        if self.1 == 0 {
            return None;
        }
        let value = ((self.0 as u32 * self.1 as u32) % 255) as u8;
        self.1 -= 1;
        let data = Data::new(vec![value; 1024]).unwrap();
        Some(data)
    }
}
#[async_std::main]
async fn main() {
    let mut app_data = Application::empty();
    let mut b_cast_origin: Option<(CastID, Sender<CastData>)> = None;
    let mut b_req_sent = false;
    let dir: String = args().nth(1).unwrap().parse().unwrap();
    let (key_send, key_recv) = channel();

    let mgr = instantiate_tui_mgr();
    spawn(serve_tui_mgr(mgr, key_send));
    let (gmgr_send, gmgr_recv) = init(dir, app_data.root_hash());
    let mut next_val = 1;
    let man_resp_result = gmgr_recv.recv();
    let service_request;
    let (app_send, app_recv) = channel();
    // let mut swarm_id = SwarmID(0);
    if let Ok(ManagerResponse::SwarmJoined(_s_id, _s_name, service_req, service_resp)) =
        man_resp_result
    {
        service_request = service_req;
        // swarm_id = s_id;
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
                    // println!("Processing data...");
                    let process_result = app_data.process(data);
                    if process_result.is_none() {
                        continue;
                    }
                    // println!("Process response: {:?}", process_result);
                    // println!("Process response");
                    let SyncMessage {
                        m_type,
                        requirements,
                        data,
                    } = process_result.unwrap();

                    // let b_type = data.first_byte();
                    println!("Received m_type: {:?}", m_type);
                    match m_type {
                        SyncMessageType::SetManifest => {
                            let old_manifest = app_data.get_all_data(0);
                            if !requirements.pre_validate(0, &app_data) {
                                println!("PRE validation failed");
                            } else {
                                let content = Content::Data(0, ContentTree::Filled(data));
                                let next_id = app_data.next_c_id().unwrap();
                                let res = if next_id == 0 {
                                    app_data.append(content).is_ok()
                                } else {
                                    app_data.update(0, content).is_ok()
                                };
                                println!("Manifest result: {:?}", res);
                                if !requirements.post_validate(0, &app_data) {
                                    println!("POST validation failed");
                                    if let Ok(data_vec) = old_manifest {
                                        let c_tree = ContentTree::from(data_vec);
                                        let old_content = Content::Data(0, c_tree);
                                        let res = app_data.update(0, old_content);
                                        println!("Restored old manifest {:?}", res.is_ok());
                                    } else {
                                        let content = Content::Data(0, ContentTree::Empty(0));
                                        let _ = app_data.update(0, content);
                                        println!("Zeroed manifest");
                                    }
                                }
                                let hash = app_data.root_hash();
                                println!("Sending updated hash: {}", hash);
                                let res = service_request.send(Request::UpdateAppRootHash(hash));
                                println!("Send res: {:?}", res);
                                // println!("Root hash: {}", app_data.root_hash());
                            }
                        }
                        SyncMessageType::AddContent => {
                            // TODO: potentially for AddContent & ChangeContent
                            // post requirements could be empty
                            // pre requirements can not be empty since we need
                            // ContentID
                            let c_id = app_data.next_c_id().unwrap();
                            if !requirements.pre_validate(c_id, &app_data) {
                                println!("PRE validation failed for AddContent");
                            } else if let Some(next_id) = app_data.next_c_id() {
                                if requirements.post.len() != 1 {
                                    println!(
                                        "POST validation failed for AddContent 1 ({:?})",
                                        requirements.post
                                    );
                                    continue;
                                }
                                let content = Content::from(data).unwrap();
                                println!("Content: {:?}", content);
                                let (recv_id, recv_hash) = requirements.post[0];
                                println!("Recv id: {}, next id: {}", recv_id, next_id);
                                println!("Recv hash: {}, next hash: {}", recv_hash, content.hash());
                                if recv_id == next_id && recv_hash == content.hash() {
                                    let res = app_data.append(content);
                                    println!("Content added: {:?}", res);
                                } else {
                                    println!("POST validation failed for AddContent two");
                                }
                                // println!("Root hash: {}", app_data.root_hash());
                            }
                        }
                        SyncMessageType::ChangeContent(c_id) => {
                            println!("ChangeContent");
                            if !requirements.pre_validate(c_id, &app_data) {
                                println!("PRE validation failed for ChangeContent");
                                continue;
                            }
                            // let (pre_recv_id, _hash) = requirements.pre[0];
                            // let (post_recv_id, recv_hash) = requirements.post[0];
                            // if pre_recv_id != post_recv_id {
                            //     println!("POST validation failed for ChangeContent 1");
                            //     continue;
                            // }
                            // if requirements.post.len() != 1 {
                            //     println!("POST validation failed for ChangeContent 2");
                            //     continue;
                            // }
                            let content = Content::from(data).unwrap();
                            let res = app_data.update(c_id, content);
                            if let Ok(old_content) = res {
                                if !requirements.post_validate(c_id, &app_data) {
                                    let restore_res = app_data.update(c_id, old_content);
                                    println!("POST validation failed on ChangeContent");
                                    println!("Restore result: {:?}", restore_res);
                                } else {
                                    println!(
                                        "ChangeContent completed successfully ({})",
                                        app_data.root_hash()
                                    );
                                }
                            } else {
                                println!("Update procedure failed: {:?}", res);
                            }
                            // if recv_hash == content.hash() {
                            //     println!("Content changed: {:?}", res);
                            // } else {
                            //     println!("POST validation failed for ChangeContent");
                            // }
                        }
                        SyncMessageType::AppendData(c_id) => {
                            //TODO
                            println!("SyncMessageType::AppendData ");
                            if !requirements.pre_validate(c_id, &app_data) {
                                println!("PRE validation failed for AppendData");
                                continue;
                            }
                            // let (pre_recv_id, _hash) = requirements.pre[0];
                            // let (post_recv_id, _recv_hash) = requirements.post[0];
                            // if pre_recv_id != post_recv_id {
                            //     println!("POST validation failed for ChangeContent 1");
                            //     continue;
                            // }
                            // TODO
                            let res = app_data.append_data(c_id, data);
                            if res.is_ok() {
                                if !requirements.post_validate(c_id, &app_data) {
                                    println!("POST validation failed for AppendData");
                                    // TODO: restore previous order
                                    let res = app_data.pop_data(c_id);
                                    println!("Restore result: {:?}", res);
                                } else {
                                    println!(
                                        "Data appended successfully ({})",
                                        app_data.root_hash()
                                    );
                                }
                            }
                        }
                        SyncMessageType::RemoveData(c_id, d_id) => {
                            //TODO
                            println!("SyncMessageType::RemoveData ");
                            if !requirements.pre_validate(c_id, &app_data) {
                                println!("PRE validation failed for RemoveData");
                                continue;
                            }
                            // let (pre_recv_id, _hash) = requirements.pre[0];
                            // let (post_recv_id, _recv_hash) = requirements.post[0];
                            // if pre_recv_id != post_recv_id {
                            //     println!("POST validation failed for RemoveData 1");
                            //     continue;
                            // }
                            // TODO
                            // let mut bytes = data.bytes();
                            // let data_idx =
                            //     u16::from_be_bytes([data.first_byte(), data.second_byte()]);
                            let res = app_data.remove_data(c_id, d_id);
                            if let Ok(removed_data) = res {
                                if !requirements.post_validate(c_id, &app_data) {
                                    println!("POST validation failed for RemoveData");
                                    // TODO: restore previous order
                                    let res = app_data.insert_data(c_id, d_id, removed_data);
                                    println!("Restore result: {:?}", res);
                                } else {
                                    println!(
                                        "Data appended successfully ({})",
                                        app_data.root_hash()
                                    );
                                }
                            }
                        }
                        SyncMessageType::UpdateData(c_id, d_id) => {
                            //TODO
                            println!("SyncMessageType::UpdateData ");
                            if !requirements.pre_validate(c_id, &app_data) {
                                println!("PRE validation failed for UpdateData");
                                continue;
                            }
                            // let (pre_recv_id, _hash) = requirements.pre[0];
                            // let (post_recv_id, _recv_hash) = requirements.post[0];
                            // if pre_recv_id != post_recv_id {
                            //     println!("POST validation failed for UpdateData 1");
                            //     continue;
                            // }
                            // TODO
                            // let mut bytes = data.bytes();
                            // let data_idx =
                            //     u16::from_be_bytes([data.first_byte(), data.second_byte()]);
                            let res = app_data.remove_data(c_id, d_id);
                            if let Ok(removed_data) = res {
                                if !requirements.post_validate(c_id, &app_data) {
                                    println!("POST validation failed for RemoveData");
                                    // TODO: restore previous order
                                    let res = app_data.insert_data(c_id, d_id, removed_data);
                                    println!("Restore result: {:?}", res);
                                } else {
                                    println!(
                                        "Data appended successfully ({})",
                                        app_data.root_hash()
                                    );
                                }
                            }
                        }
                        SyncMessageType::InsertData(c_id, d_id) => {
                            //TODO
                            println!("SyncMessageType::InsertData ");
                        }
                        SyncMessageType::ExtendData(c_id, d_id) => {
                            //TODO
                            println!("SyncMessageType::ExtendData ");
                        }
                        SyncMessageType::UserDefined(_value) => {
                            //TODO
                            println!("SyncMessageType::UserDefined({})", _value);
                        }
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
                    println!(
                        "Got AppSync response {} for {} [{}/{}]:\n{:?}",
                        sync_type,
                        c_id,
                        part_no,
                        total,
                        data.len()
                    );

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
                                let content = Content::Data(
                                    0,
                                    ContentTree::Filled(Data::new(data.bytes()).unwrap()),
                                );
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
                    let hashes = app_data.all_content_root_hashes();
                    let c_id = 0;
                    let total = hashes.len() as u16 - 1;
                    for (part_no, group) in hashes.into_iter().enumerate() {
                        let mut byte_hashes = vec![];
                        for hash in group.iter() {
                            for byte in hash.to_be_bytes() {
                                byte_hashes.push(byte);
                            }
                        }
                        let _ = service_request.send(Request::SendData(
                            gnome_id,
                            NeighborResponse::AppSync(
                                sync_type,
                                c_id,
                                part_no as u16,
                                total,
                                SyncData::new(byte_hashes).unwrap(),
                            ),
                        ));
                    }
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
                                    data.to_sync(),
                                ),
                            ));
                        }
                        println!("Sent CID response");
                    }
                }
                Response::BroadcastOrigin(_s_id, c_id, send) => b_cast_origin = Some((c_id, send)),
                _ => {
                    println!("Unserved by app: {:?}", resp);
                }
            }
        }
        if let Ok(key) = key_recv.try_recv() {
            // println!("some key");
            match key {
                Key::U => {
                    if b_cast_origin.is_none() {
                        println!("Unable to upload - no active broadcast.");
                        if !b_req_sent {
                            println!("Requesting broadcast channel.");
                            let res = service_request.send(Request::StartBroadcast);
                            b_req_sent = res.is_ok();
                        }
                        continue;
                    }
                    let (broadcast_id, bcast_send) = b_cast_origin.clone().unwrap();
                    // TODO: here we need to write a procedure for data upload
                    // 1. Select data to upload
                    // 2. Split data into 64MibiByte chunks
                    //
                    let d_type = 7;
                    let size = 128;
                    let big_chunks = vec![BigChunk(0, size)];
                    // Then for each big-chunk:
                    for mut big_chunk in big_chunks.into_iter() {
                        let description = String::new();
                        let missing_hashes = vec![];
                        let data_hashes = vec![];
                        let mut data_vec = Vec::with_capacity(big_chunk.1 as usize);
                        let mut hashes = Vec::with_capacity(big_chunk.1 as usize);
                        println!("// 3. Split big-chunk into 1024byte small-chunks");
                        while let Some(small_chunk) = big_chunk.next() {
                            // 4. Compute hash for each small-chunk
                            hashes.push(small_chunk.hash());
                            // TODO: build proper CastData from Data
                            data_vec.push(small_chunk.to_cast());
                        }
                        println!("// 5. Compute root hash from previous hashes.");
                        let root_hash = get_root_hash(&hashes);
                        println!("// 6. Instantiate TransformInfo");
                        let ti = TransformInfo {
                            d_type,
                            tags: vec![],
                            size,
                            root_hash,
                            broadcast_id,
                            description,
                            missing_hashes,
                            data_hashes,
                        };
                        //
                        println!("// 7. SyncMessage::Append as many Data::Link to Datastore as necessary");
                        if let Some(next_id) = app_data.next_c_id() {
                            let pre: Vec<(ContentID, u64)> = vec![(next_id, 0)];
                            let link =
                                Content::Link(GnomeId(u64::MAX), String::new(), u16::MAX, Some(ti));
                            let link_hash = link.hash();
                            println!("Link hash: {}", link_hash);
                            let data = link.link_to_data().unwrap();
                            println!("Link data: {:?}", data);
                            println!("Data hash: {}", data.hash());
                            // let post: Vec<(ContentID, u64)> = vec![(next_id, data.hash())];
                            let post: Vec<(ContentID, u64)> = vec![(next_id, link_hash)];
                            let reqs = SyncRequirements { pre, post };
                            let msg = SyncMessage::new(SyncMessageType::AddContent, reqs, data);
                            let parts = msg.into_parts();
                            for part in parts {
                                let _ = service_request.send(Request::AddData(part));
                            }
                            next_val += 1;
                        }
                        println!("// 8. For each Link Send computed Hashes via broadcast");
                        let (done_send, done_recv) = channel();
                        let mut hash_bytes = vec![];
                        for chunk in hashes.chunks(128) {
                            let mut outgoing_bytes = Vec::with_capacity(1024);
                            for hash in chunk {
                                for byte in u64::to_be_bytes(*hash) {
                                    outgoing_bytes.push(byte)
                                }
                            }
                            hash_bytes.push(CastData::new(outgoing_bytes).unwrap());
                        }
                        spawn(serve_broadcast_origin(
                            broadcast_id,
                            Duration::from_millis(400),
                            bcast_send.clone(),
                            hash_bytes,
                            done_send.clone(),
                        ));
                        let _done_res = done_recv.recv();
                        println!("Hashes sent: {}", _done_res.is_ok());
                        // TODO
                        // 9. SyncMessage::Transform a Link into Data

                        //10. Send Data chunks via broadcast
                        spawn(serve_broadcast_origin(
                            broadcast_id,
                            Duration::from_millis(200),
                            bcast_send.clone(),
                            data_vec,
                            done_send.clone(),
                        ));
                        // let done_res = done_recv.recv();
                    }
                }
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
                    let res = service_request.send(Request::StartBroadcast);
                    b_req_sent = res.is_ok();
                }
                Key::M => {
                    let mut prebytes = vec![0];
                    if let Ok(hash) = app_data.content_root_hash(0) {
                        for byte in hash.to_be_bytes() {
                            prebytes.push(byte);
                        }
                    } else {
                        for _i in 0..8 {
                            prebytes.push(0);
                        }
                    };
                    // println!("Prebytes: {:?}", prebytes);
                    let mani = manifest();
                    let pre: Vec<(ContentID, u64)> = vec![(0, 0)];
                    let post: Vec<(ContentID, u64)> = vec![(0, mani.hash())];
                    let reqs = SyncRequirements { pre, post };
                    let msg =
                        SyncMessage::new(SyncMessageType::SetManifest, reqs, mani.to_data(None));
                    let parts = msg.into_parts();
                    // println!(
                    //     "to_data len: {}, hash: {:?}",
                    //     mani.to_data(None).len(),
                    //     mani.to_data(None).hash().to_be_bytes()
                    // );
                    // let manifest_hash = mani.hash();
                    // prebytes.append(&mut Vec::from(manifest_hash.to_be_bytes()));
                    // println!("Prebytes: {:?}", prebytes);

                    for part in parts {
                        let _ = service_request.send(Request::AddData(part));
                    }
                }
                Key::N => {
                    let _ = service_request.send(Request::ListNeighbors);
                }
                Key::C => {
                    let c_id: u16 = 1;
                    let pre_hash_result = app_data.content_root_hash(c_id);
                    // println!("About to change content {:?}", pre_hash_result);
                    if let Ok(pre_hash) = pre_hash_result {
                        // let pre_hash = pre_hash_result.unwrap();
                        let pre: Vec<(ContentID, u64)> = vec![(c_id, pre_hash)];
                        let data = Data::new(vec![next_val]).unwrap();
                        let post: Vec<(ContentID, u64)> = vec![(c_id, data.hash())];
                        // We prepend 0 to indicate it is not a Link
                        let data = Data::new(vec![0, next_val]).unwrap();
                        let reqs = SyncRequirements { pre, post };
                        let msg =
                            SyncMessage::new(SyncMessageType::ChangeContent(c_id), reqs, data);
                        let parts = msg.into_parts();
                        for part in parts {
                            let _ = service_request.send(Request::AddData(part));
                        }
                        next_val += 1;
                    }
                }
                Key::S => {
                    if let Some(next_id) = app_data.next_c_id() {
                        let pre: Vec<(ContentID, u64)> = vec![(next_id, 0)];
                        let data = Data::new(vec![next_val]).unwrap();
                        let post: Vec<(ContentID, u64)> = vec![(next_id, data.hash())];
                        let data = Data::new(vec![0, next_val]).unwrap();
                        let reqs = SyncRequirements { pre, post };
                        let msg = SyncMessage::new(SyncMessageType::AddContent, reqs, data);
                        let parts = msg.into_parts();
                        for part in parts {
                            let _ = service_request.send(Request::AddData(part));
                        }
                        next_val += 1;
                    }
                }
                Key::ShiftS => {
                    let data = vec![next_val; 1024];

                    let _ = service_request.send(Request::AddData(SyncData::new(data).unwrap()));
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
                Response::BroadcastOrigin(_s_id, ref _c_id, ref _send_d) => {
                    let _ = to_app.send(resp);
                    // spawn(serve_broadcast_origin(
                    //     c_id,
                    //     Duration::from_millis(200),
                    //     send_d,
                    // ));
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
async fn serve_unicast(c_id: CastID, sleep_time: Duration, user_res: Receiver<CastData>) {
    println!("Serving unicast {:?}", c_id);
    loop {
        let recv_res = user_res.try_recv();
        if let Ok(data) = recv_res {
            println!("U{:?}: {}", c_id, data);
        }
        sleep(sleep_time).await;
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
async fn serve_tui_mgr(mut mgr: Manager, to_app: Sender<Key>) {
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
async fn serve_broadcast(c_id: CastID, sleep_time: Duration, user_res: Receiver<CastData>) {
    println!("Serving broadcast {:?}", c_id);
    loop {
        let recv_res = user_res.try_recv();
        if let Ok(data) = recv_res {
            println!("B{:?}: {}", c_id, data);
        }
        sleep(sleep_time).await;
    }
}
async fn serve_unicast_origin(c_id: CastID, sleep_time: Duration, user_res: Sender<CastData>) {
    println!("Originating unicast {:?}", c_id);
    let mut i: u8 = 0;
    loop {
        let send_res = user_res.send(CastData::new(vec![i]).unwrap());
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
async fn serve_broadcast_origin(
    c_id: CastID,
    sleep_time: Duration,
    user_res: Sender<CastData>,
    data_vec: Vec<CastData>,
    done: Sender<()>,
) {
    println!("Originating broadcast {:?}", c_id);
    // TODO: indexing
    for data in data_vec {
        // loop {
        let send_res = user_res.send(data);
        if send_res.is_ok() {
            println!("Broadcasted ",);
        } else {
            println!(
                "Error while trying to broadcast: {:?}",
                send_res.err().unwrap()
            );
        }
        // i = i.wrapping_add(1);

        sleep(sleep_time).await;
    }
    done.send(());
}

fn get_root_hash(hashes: &Vec<u64>) -> u64 {
    let h_len = hashes.len();
    let mut sub_hashes = Vec::with_capacity((h_len >> 1 as usize) + 1);
    for i in 0..h_len >> 1 {
        sub_hashes.push(double_hash(hashes[2 * i], hashes[2 * i + 1]));
    }
    if h_len % 2 == 1 {
        sub_hashes.push(hashes[h_len - 1]);
    }
    if sub_hashes.len() == 1 {
        return sub_hashes[0];
    } else {
        get_root_hash(&sub_hashes)
    }
}
