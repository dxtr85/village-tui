use async_std::channel as achannel;
use async_std::task::spawn_blocking;
use dapp_lib::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::{env::args, path::Path};
mod config;
mod logic;
mod tui;
use config::Configuration;
use logic::ApplicationLogic;
use tui::{instantiate_tui_mgr, serve_tui_mgr};

#[async_std::main]
async fn main() {
    let dir = if let Some(arg) = args().nth(1) {
        let args = arg.to_string();
        PathBuf::new().join(args)
    } else {
        PathBuf::new()
    };

    let (to_presentation_msg_send, to_presentation_msg_recv) = channel();
    let (from_presentation_msg_send, from_presentation_msg_recv) = channel();
    let (to_application_send, to_application_recv) = channel();
    let (to_app_mgr_send, to_app_mgr_recv) = achannel::unbounded();

    let tui_mgr = instantiate_tui_mgr();

    let mut config = Configuration::new(&dir).await;
    let storage_neighbors = if config.storage_neighbors.is_empty() {
        vec![]
    } else {
        std::mem::replace(&mut config.storage_neighbors, vec![])
    };
    let my_id = initialize(
        to_application_send.clone(),
        to_app_mgr_send.clone(),
        to_app_mgr_recv,
        dir.clone(),
        storage_neighbors,
    )
    .await;
    let mut logic = ApplicationLogic::new(
        my_id,
        to_app_mgr_send,
        to_presentation_msg_send,
        from_presentation_msg_send.clone(),
        from_presentation_msg_recv,
        to_application_send,
        to_application_recv,
    );

    let tui_join = spawn_blocking(move || {
        serve_tui_mgr(
            my_id,
            tui_mgr,
            from_presentation_msg_send,
            to_presentation_msg_recv,
            config,
        )
    });
    logic.run().await;
    tui_join.await;
}
