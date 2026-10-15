use dapp_lib::{
    prelude::{extract_chop_data, CastData, ChopData, SwarmID},
    ToAppMgr,
};
use smol::channel::{Receiver, Sender};

pub mod poledit;
// pub use poledit::PolicyEditor;
pub async fn handle_bcast(
    swarm_id: SwarmID,
    recv: Receiver<CastData>,
    to_app_mgr: Sender<ToAppMgr>,
) {
    // TODO: for larger files we might collect all pages
    // for one or more Contents before we AppendMultipleContents
    // to Datastore. dapp_lib should be able to cover this.
    // User should not have to worry about this ordering.
    // eprintln!("In HANDLE_BCAST");
    //
    // TODO:: send first leaf data at the end of transmission so that we know
    //        it is time to break the loop
    let mut first_leaf_data = None;
    let mut leaf_hashes = Vec::with_capacity(512);
    let mut leaf_data = Vec::with_capacity(u16::MAX as usize);
    loop {
        if let Ok(msg) = recv.recv().await {
            let ext_res = extract_chop_data(msg);
            // eprintln!("BCast, ");
            if let Ok(chop) = ext_res {
                if chop.is_hashes() {
                    let last_hash = chop.is_last_hash();
                    leaf_hashes.push(chop);
                    if last_hash {
                        // TODO: send what we have till now
                        let _ = to_app_mgr
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::StoreChopData(
                                swarm_id,
                                std::mem::replace(&mut leaf_hashes, Vec::with_capacity(512)),
                            )))
                            .await;
                    }
                } else if chop.is_data() {
                    if first_leaf_data.is_none() {
                        first_leaf_data = chop.data_hash();
                    } else {
                        if let Some(first) = first_leaf_data {
                            if let Some(dhash) = chop.data_hash() {
                                if first == dhash {
                                    //TODO: send it
                                    let _ = to_app_mgr
                                        .send(ToAppMgr::FromApp(
                                            dapp_lib::LibRequest::StoreChopData(
                                                swarm_id,
                                                std::mem::replace(&mut leaf_data, Vec::new()),
                                            ),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }
                    }
                    leaf_data.push(chop);
                    if leaf_data.len() == u16::MAX as usize {
                        //TODO: send it
                        let _ = to_app_mgr
                            .send(ToAppMgr::FromApp(dapp_lib::LibRequest::StoreChopData(
                                swarm_id,
                                std::mem::replace(
                                    &mut leaf_data,
                                    Vec::with_capacity(u16::MAX as usize),
                                ),
                            )))
                            .await;
                    }
                } else {
                    //TODO: some other ChopData type?
                }
            } else {
                //TODO: not ChopData received
            }
        } else {
            // Do we get an error here when broadcast is terminated by origin?
            // TODO: send all we've got
            if !leaf_hashes.is_empty() {
                let _ = to_app_mgr
                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::StoreChopData(
                        swarm_id,
                        std::mem::replace(&mut leaf_hashes, Vec::new()),
                    )))
                    .await;
            }
            if !leaf_data.is_empty() {
                let _ = to_app_mgr
                    .send(ToAppMgr::FromApp(dapp_lib::LibRequest::StoreChopData(
                        swarm_id,
                        std::mem::replace(&mut leaf_data, Vec::new()),
                    )))
                    .await;
            }
            break;
        }
    }
    eprintln!("END of handle_bcast ");
}
