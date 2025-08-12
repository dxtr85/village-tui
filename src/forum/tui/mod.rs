use animaterm::prelude::Key;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::GnomeId;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use crate::config::Configuration;

pub enum ToForumView {}
pub enum FromForumView {
    SwitchTo(AppType),
    Quit,
}
pub fn serve_forum_tui(
    my_id: GnomeId,
    mut mgr: Manager,
    to_app: Sender<FromForumView>,
    // to_tui_send: Sender<ToPresentation>,
    to_tui_recv: Receiver<ToForumView>,
    config: Configuration,
) -> (Manager, Configuration) {
    loop {
        if let Some(key) = mgr.read_key() {
            match key {
                Key::ShiftQ => {
                    eprintln!("Forum ShiftQ");
                    to_app.send(FromForumView::Quit);
                    break;
                }
                Key::C => {
                    eprintln!("Forum C");
                    to_app.send(FromForumView::SwitchTo(AppType::Catalog));
                    break;
                }
                _other => {
                    //TODO
                }
            }
        }
    }
    eprintln!("serve_forum_tui is done");
    (mgr, config)
}
