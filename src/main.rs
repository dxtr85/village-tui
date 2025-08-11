use async_std::channel::{self as achannel, Receiver as AReceiver, Sender};
use async_std::task::spawn;
use dapp_lib::prelude::*;
use std::env::args;
use std::path::PathBuf;
mod catalog;
mod config;
mod forum;
use catalog::logic::CatalogLogic;
use catalog::tui::{instantiate_tui_mgr, FromCatalogView};
use config::Configuration;
use forum::logic::ForumLogic;

enum InternalMsg {
    Tui(FromCatalogView),
    User(ToApp),
    PresentOptionsForTag(u8, String),
}

#[async_std::main]
async fn main() {
    let dir = if let Some(arg) = args().nth(1) {
        let args = arg.to_string();
        PathBuf::new().join(args)
    } else {
        PathBuf::new()
    };

    let (to_application_send, to_application_recv) = achannel::unbounded();
    let (wrapped_sender, wrapped_receiver) = achannel::unbounded();
    let (to_app_mgr_send, to_app_mgr_recv) = achannel::unbounded();
    let mut config = Configuration::new(&dir).await;
    let storage_neighbors = if config.storage_neighbors.is_empty() {
        vec![]
    } else {
        std::mem::replace(&mut config.storage_neighbors, vec![])
    };
    let my_name = initialize(
        to_application_send,
        to_app_mgr_send.clone(),
        to_app_mgr_recv,
        dir.clone(),
        storage_neighbors,
    )
    .await;

    spawn(to_user_adapter(to_application_recv, wrapped_sender.clone()));
    let tui_mgr = instantiate_tui_mgr();
    // TODO: When logic.run() is done, it returns Option<AppType>,
    //       and if that option is Some, another logic is started
    //       for that new AppType.
    //       Along with new logic, serve_tui_mgr task should be informed
    //       about user interface switch. Upon receiving that info,
    //       serve_tui_mgr should break it's loop and spawn a new one for AppType.
    //       Also new from_tui_adapter should be spawned,
    //       old one should self-terminate on error receiving FromPresentation.
    // TODO: InternalMessage should serve every defined AppType, and Notification
    let mut next_app = Some((AppType::Catalog, wrapped_receiver, config, tui_mgr));
    loop {
        if let Some((app_type, wrapped_receiver, config, mut tui_mgr)) = next_app.take() {
            match app_type {
                AppType::Catalog => {
                    let c_logic = CatalogLogic::new(
                        my_name.clone(),
                        to_app_mgr_send.clone(),
                        &mut tui_mgr,
                        wrapped_sender.clone(),
                        wrapped_receiver,
                    );
                    next_app = c_logic.run(dir.clone(), config, tui_mgr).await;
                }
                AppType::Forum => {
                    let f_logic = ForumLogic::new(
                        to_app_mgr_send.clone(),
                        wrapped_sender.clone(),
                        wrapped_receiver,
                    );
                    next_app = f_logic
                        .run(my_name.founder, dir.clone(), config, tui_mgr)
                        .await;
                }
                AppType::Other(_x) => {
                    //TODO
                    break;
                }
            }
        } else {
            break;
        }
    }
    eprintln!("Main loop is done.");
}

async fn to_user_adapter(to_user: AReceiver<ToApp>, wrapped_sender: Sender<InternalMsg>) {
    while let Ok(to_app) = to_user.recv().await {
        let _ = wrapped_sender.send(InternalMsg::User(to_app)).await;
    }
}
