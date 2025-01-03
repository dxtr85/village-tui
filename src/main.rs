use async_std::task::spawn_blocking;
use dapp_lib::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::{env::args, path::Path};
mod config;
mod logic;
mod tui;
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
    let config = Configuration::new(dir.clone());

    let (to_presentation_msg_send, to_presentation_msg_recv) = channel();
    let (from_presentation_msg_send, from_presentation_msg_recv) = channel();
    let (to_application_send, to_application_recv) = channel();
    let (to_app_mgr_send, to_app_mgr_recv) = channel();

    let tui_mgr = instantiate_tui_mgr();

    let my_id = initialize(
        to_application_send.clone(),
        to_app_mgr_send.clone(),
        to_app_mgr_recv,
        dir.clone(),
    );
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
            dir,
        )
    });
    logic.run().await;
    tui_join.await;
}
