use async_std::channel::{self as achannel, Receiver as AReceiver, Sender};
use async_std::task::{sleep, spawn, spawn_blocking};
use dapp_lib::prelude::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use std::{env::args, path::Path};
mod config;
mod logic;
mod tui;
use config::Configuration;
use logic::ApplicationLogic;
use tui::Notifier;
use tui::{instantiate_tui_mgr, serve_tui_mgr, FromPresentation};

enum InternalMsg {
    Tui(FromPresentation),
    User(ToApp),
}

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
    let (to_application_send, to_application_recv) = achannel::unbounded();
    let (wrapped_sender, wrapped_receiver) = achannel::unbounded();
    let (to_app_mgr_send, to_app_mgr_recv) = achannel::unbounded();

    let mut tui_mgr = instantiate_tui_mgr();
    //TODO: we want to make ApplicationLogic::run() async
    // Restrictions:
    // - We can not alter logic in tui module
    // - We can not make ToUser wrap FromTUI messages
    //
    // Solution:
    // - We create an enum wrapper that can carry both ToUser and FromPresentation
    // - We spawn two adapter services, one for wrapping ToUser messages
    //   and one for FromPresentation
    // - We modify run fn to only listen on a single async receiver

    let mut config = Configuration::new(&dir).await;
    let storage_neighbors = if config.storage_neighbors.is_empty() {
        vec![]
    } else {
        std::mem::replace(&mut config.storage_neighbors, vec![])
    };
    let my_id = initialize(
        to_application_send,
        to_app_mgr_send.clone(),
        to_app_mgr_recv,
        dir.clone(),
        storage_neighbors,
    )
    .await;

    spawn(to_user_adapter(to_application_recv, wrapped_sender.clone()));
    spawn(from_tui_adapter(
        from_presentation_msg_recv,
        wrapped_sender.clone(),
    ));
    let (notification_sender, notification_receiver) = achannel::unbounded();
    let mut logic = ApplicationLogic::new(
        my_id,
        to_app_mgr_send,
        to_presentation_msg_send.clone(),
        notification_sender.clone(), // from_presentation_msg_send.clone(),
        // from_presentation_msg_recv,
        wrapped_sender,
        wrapped_receiver,
    );
    let s_size = tui_mgr.screen_size();
    let notifier = Notifier::new(
        (s_size.0 as isize, 0),
        &mut tui_mgr,
        (notification_sender.clone(), notification_receiver),
        to_presentation_msg_send,
    );
    spawn(notifier.serve());
    let _res = notification_sender
        .send(Some(format!("Wciśnij F1 aby uzyskać pomoc")))
        .await;
    eprintln!("Sent testowa notka: {:?}", _res);
    let tui_join = spawn_blocking(move || {
        serve_tui_mgr(
            my_id,
            tui_mgr,
            from_presentation_msg_send,
            // to_presentation_msg_send,
            to_presentation_msg_recv,
            config,
        )
    });
    logic.run().await;
    tui_join.await;
}

async fn to_user_adapter(to_user: AReceiver<ToApp>, wrapped_sender: Sender<InternalMsg>) {
    let timeout = Duration::from_millis(16);
    // loop {
    // if let Ok(to_app) = to_user.recv_timeout(timeout) {
    while let Ok(to_app) = to_user.recv().await {
        wrapped_sender.send(InternalMsg::User(to_app)).await;
        // } else {
        //     sleep(timeout).await
        // }
    }
}

async fn from_tui_adapter(
    from_presentation: Receiver<FromPresentation>,
    wrapped_sender: Sender<InternalMsg>,
) {
    let timeout = Duration::from_millis(16);
    loop {
        if let Ok(from_tui) = from_presentation.recv_timeout(timeout) {
            wrapped_sender.send(InternalMsg::Tui(from_tui)).await;
        } else {
            sleep(timeout).await
        }
    }
}
