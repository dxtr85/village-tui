use animaterm::Manager;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use async_std::task::spawn_blocking;
use dapp_lib::prelude::GnomeId;
use dapp_lib::ToAppMgr;
use std::path::PathBuf;

use dapp_lib::prelude::AppType;

use crate::config::Configuration;
use crate::forum::tui::serve_forum_tui;
use crate::InternalMsg;
pub struct ForumLogic {
    to_app_mgr_send: ASender<ToAppMgr>,
    to_user_send: ASender<InternalMsg>,
    to_user_recv: AReceiver<InternalMsg>,
}
impl ForumLogic {
    pub fn new(
        to_app_mgr_send: ASender<ToAppMgr>,
        to_user_send: ASender<InternalMsg>,
        to_user_recv: AReceiver<InternalMsg>,
    ) -> Self {
        ForumLogic {
            to_app_mgr_send,
            to_user_send,
            to_user_recv,
        }
    }
    pub async fn run(
        mut self,
        founder: GnomeId,
        config_dir: PathBuf,
        mut config: Configuration,
        mut tui_mgr: Manager,
    ) -> Option<(AppType, AReceiver<InternalMsg>, Configuration, Manager)> {
        tui_mgr.new_display(false);
        eprintln!("Forum {}", tui_mgr.screen_size().0);
        let tui_join = spawn_blocking(move || {
            serve_forum_tui(
                founder, tui_mgr,
                // from_presentation_msg_send,
                // to_presentation_msg_recv,
                config,
            )
        });
        eprintln!("ForumLogic is done");
        (tui_mgr, config) = tui_join.await;
        tui_mgr.terminate();
        eprintln!("Forum is all done.");
        None
    }
}
