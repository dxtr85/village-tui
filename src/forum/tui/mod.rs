use animaterm::Manager;
use dapp_lib::prelude::GnomeId;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use crate::config::Configuration;

pub enum ToForumView {}
pub enum FromForumView {}
pub fn serve_forum_tui(
    my_id: GnomeId,
    mut mgr: Manager,
    // to_app: Sender<FromForumView>,
    // to_tui_send: Sender<ToPresentation>,
    // to_tui_recv: Receiver<ToForumView>,
    config: Configuration,
) -> (Manager, Configuration) {
    eprintln!("serve_forum_tui is done");
    (mgr, config)
}
