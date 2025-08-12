use animaterm::prelude::*;
use async_std::channel::Receiver as AReceiver;
use async_std::channel::Sender as ASender;
use async_std::task::sleep;
use async_std::task::spawn;
use async_std::task::spawn_blocking;
use dapp_lib::prelude::GnomeId;
use dapp_lib::ToAppMgr;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

use dapp_lib::prelude::AppType;

use crate::config::Configuration;
use crate::forum::tui::serve_forum_tui;
use crate::forum::tui::FromForumView;
use crate::forum::tui::ToForumView;
use crate::InternalMsg;
pub struct ForumLogic {
    to_app_mgr_send: ASender<ToAppMgr>,
    to_user_send: ASender<InternalMsg>,
    to_user_recv: AReceiver<InternalMsg>,
    to_tui_send: Sender<ToForumView>,
    to_tui_recv: Option<Receiver<ToForumView>>,
    from_tui_send: Option<Sender<FromForumView>>,
}
impl ForumLogic {
    pub fn new(
        to_app_mgr_send: ASender<ToAppMgr>,
        to_user_send: ASender<InternalMsg>,
        to_user_recv: AReceiver<InternalMsg>,
    ) -> Self {
        let (to_tui_send, to_tui_recv) = channel();
        let (from_tui_send, from_tui_recv) = channel();
        spawn(from_forum_tui_adapter(
            from_tui_recv,
            to_user_send.clone(),
            // wrapped_sender.clone(),
        ));
        ForumLogic {
            to_app_mgr_send,
            to_user_send,
            to_user_recv,
            to_tui_send,
            to_tui_recv: Some(to_tui_recv),
            from_tui_send: Some(from_tui_send),
        }
    }
    pub async fn run(
        mut self,
        founder: GnomeId,
        config_dir: PathBuf,
        mut config: Configuration,
        mut tui_mgr: Manager,
    ) -> Option<(AppType, AReceiver<InternalMsg>, Configuration, Manager)> {
        let _main_display = tui_mgr.new_display(false);
        let (cols, rows) = tui_mgr.screen_size();
        let frame = vec![Glyph::green(); cols * rows];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let bg = Graphic::new(cols, rows, 0, library, None);
        let bg_idx = tui_mgr.add_graphic(bg, 1, (0, 0)).unwrap();
        tui_mgr.set_graphic(bg_idx, 0, true);
        eprintln!("Forum background index: {}", bg_idx);
        // eprintln!("Forum {}", tui_mgr.screen_size().0);
        let from_presentation_msg_send = self.from_tui_send.take().unwrap();
        let to_presentation_msg_recv = self.to_tui_recv.take().unwrap();
        let tui_join = spawn_blocking(move || {
            serve_forum_tui(
                founder,
                tui_mgr,
                from_presentation_msg_send,
                to_presentation_msg_recv,
                config,
            )
        });
        let mut switch_to_opt = None;

        loop {
            let int_msg_res = self.to_user_recv.recv().await;
            if int_msg_res.is_err() {
                eprintln!("Forum error recv internal: {}", int_msg_res.err().unwrap());
                break;
            }
            let msg = int_msg_res.unwrap();
            match msg {
                InternalMsg::User(to_app) => match to_app {
                    dapp_lib::ToApp::ActiveSwarm(s_name, s_id) => {
                        eprintln!("Forum: ToApp::ActiveSwarm({s_id},{s_name})");
                    }
                    _other => {
                        eprintln!("InternalMsg::User");
                    }
                },
                InternalMsg::Forum(from_tui) => {
                    eprintln!("InternalMsg::Forum");
                    match from_tui {
                        FromForumView::Quit => {
                            break;
                        }
                        FromForumView::SwitchTo(app_type) => {
                            switch_to_opt = Some(app_type);
                            break;
                        }
                    }
                }
                _other => {
                    eprintln!("Forum unexpected InternalMsg");
                }
            }
        }

        eprintln!("ForumLogic is done");
        (tui_mgr, config) = tui_join.await;

        eprintln!("Forum is all done.");
        if let Some(switch_app) = switch_to_opt {
            Some((switch_app, self.to_user_recv, config, tui_mgr))
        } else {
            tui_mgr.terminate();
            None
        }
    }
}

pub async fn from_forum_tui_adapter(
    from_presentation: Receiver<FromForumView>,
    wrapped_sender: ASender<InternalMsg>,
) {
    let timeout = Duration::from_millis(16);
    loop {
        let recv_res = from_presentation.recv_timeout(timeout);
        match recv_res {
            Ok(from_tui) => {
                let _ = wrapped_sender.send(InternalMsg::Forum(from_tui)).await;
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => sleep(timeout).await,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
    eprintln!("from_tui_adapter is done");
}
