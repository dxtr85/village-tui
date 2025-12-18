use animaterm::Manager;
// use async_std::channel::{self as achannel, Receiver as AReceiver, Sender};
// use async_std::task::spawn;
use dapp_lib::prelude::*;
use dapp_lib::ToAppMgr;
use smol::block_on;
use smol::channel::Receiver;
use smol::LocalExecutor;
use std::env::args;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
mod catalog;
mod common;
mod config;
mod forum;
use catalog::logic::CatalogLogic;
pub use catalog::tui::Creator;
pub use catalog::tui::Editor;
pub use catalog::tui::Indexer;
pub use catalog::tui::Selector;
use catalog::tui::{instantiate_tui_mgr, FromCatalogView};
use config::Configuration;
use forum::logic::ForumLogic;
use smol::channel as achannel;
// use smol_macros::main;

use crate::common::poledit::PolicyEditor;
use crate::forum::tui::FromForumView;

enum InternalMsg {
    Catalog(FromCatalogView),
    Forum(FromForumView),
    User(ToApp),
    PresentOptionsForTag(u8, String),
}

struct Toolbox {
    editor: Option<Editor>,
    creator: Option<Creator>,
    selector: Option<Selector>,
    indexer: Option<Indexer>,
    policy_editor: Option<PolicyEditor>,
}
impl Toolbox {
    pub fn empty() -> Self {
        Toolbox {
            editor: None,
            creator: None,
            selector: None,
            indexer: None,
            policy_editor: None,
        }
    }
    pub fn get_tools(
        &mut self,
        manager: Manager,
        config: Configuration,
        get_editor: bool,
        get_creator: bool,
        get_selector: bool,
        get_indexer: bool,
        get_policy_editor: bool,
    ) -> Toolset {
        let editor = if get_editor { self.editor.take() } else { None };

        let creator = if get_creator {
            self.creator.take()
        } else {
            None
        };

        let selector = if get_selector {
            self.selector.take()
        } else {
            None
        };

        let indexer = if get_indexer {
            self.indexer.take()
        } else {
            None
        };
        let policy_editor = if get_policy_editor {
            self.policy_editor.take()
        } else {
            None
        };
        Toolset {
            manager,
            config,
            editor,
            creator,
            selector,
            indexer,
            policy_editor,
        }
    }
    pub fn return_tools(
        &mut self,
        editor: Option<Editor>,
        creator: Option<Creator>,
        selector: Option<Selector>,
        indexer: Option<Indexer>,
        policy_editor: Option<PolicyEditor>,
    ) {
        if editor.is_some() {
            self.editor = editor;
        }
        if creator.is_some() {
            self.creator = creator;
        }
        if selector.is_some() {
            self.selector = selector;
        }
        if indexer.is_some() {
            self.indexer = indexer;
        }
        if policy_editor.is_some() {
            self.policy_editor = policy_editor;
        }
    }
}

struct Toolset {
    manager: Manager,
    config: Configuration,
    editor: Option<Editor>,
    creator: Option<Creator>,
    selector: Option<Selector>,
    indexer: Option<Indexer>,
    policy_editor: Option<PolicyEditor>,
}
impl Toolset {
    pub fn fold(
        manager: Manager,
        config: Configuration,
        editor: Option<Editor>,
        creator: Option<Creator>,
        selector: Option<Selector>,
        indexer: Option<Indexer>,
        policy_editor: Option<PolicyEditor>,
    ) -> Self {
        Toolset {
            manager,
            config,
            editor,
            creator,
            selector,
            indexer,
            policy_editor,
        }
    }
    pub fn unfold(
        self,
    ) -> (
        Manager,
        Configuration,
        Option<Editor>,
        Option<Creator>,
        Option<Selector>,
        Option<Indexer>,
        Option<PolicyEditor>,
    ) {
        (
            self.manager,
            self.config,
            self.editor,
            self.creator,
            self.selector,
            self.indexer,
            self.policy_editor,
        )
    }
    pub fn discard(mut self) {
        if let Some(e) = self.editor {
            e.cleanup(0, &mut self.manager);
        }
        if let Some(e) = self.creator {
            e.cleanup(0, &mut self.manager);
        }
        if let Some(e) = self.selector {
            e.cleanup(0, &mut self.manager);
        }
        if let Some(e) = self.indexer {
            e.cleanup(0, &mut self.manager);
        }
        if let Some(e) = self.policy_editor {
            e.cleanup(0, &mut self.manager);
        }
        self.manager.terminate();
    }
}

// #[async_std::main]
// main! {
// async fn main() {
//
//use async_channel::unbounded;
use easy_parallel::Parallel;
use smol::future;
// use futures_lite::future;
use smol::Executor;

fn main() -> smol::io::Result<()> {
    let dir = if let Some(arg) = args().nth(1) {
        let args = arg.to_string();
        PathBuf::new().join(args)
    } else {
        PathBuf::new()
    };

    block_on(run(dir))
}

async fn run(dir: PathBuf) -> smol::io::Result<()> {
    let (to_application_send, to_application_recv) = achannel::unbounded();
    let (wrapped_sender, wrapped_receiver) = achannel::unbounded();
    let (to_app_mgr_send, to_app_mgr_recv) = achannel::unbounded();
    let (my_name_send, my_name_recv) = achannel::bounded(1);
    let mut config = Configuration::new(&dir).await;
    let storage_neighbors = if config.storage_neighbors.is_empty() {
        vec![]
    } else {
        std::mem::replace(&mut config.storage_neighbors, vec![])
    };

    let local_ex = LocalExecutor::new();
    local_ex
        .spawn(to_user_adapter(to_application_recv, wrapped_sender.clone()))
        .detach();
    // local_ex
    //     .run(run_app(
    //         dir.clone(),
    //         my_name_recv,
    //         config,
    //         to_app_mgr_send.clone(),
    //         wrapped_sender,
    //         wrapped_receiver,
    //     ))
    //     .await;

    // TODO: create an executor for entire dapp-lib and all of it's subtasks
    // TODO: spawn each and every dapp-lib's (sub)task using that executor
    let ex = Arc::new(Executor::new());
    let (signal, shutdown) = achannel::unbounded::<()>();

    let s_ex2 = ex.clone();

    let io_ex = Arc::new(Executor::new());
    let (io_signal, io_shutdown) = achannel::unbounded::<()>();

    let cl_io_ex = io_ex.clone();
    thread::spawn(move || run_io_executor(cl_io_ex, io_shutdown));

    let t_am_s = to_app_mgr_send.clone();
    let d_clone = dir.clone();
    Parallel::new()
        .each(0..4, |_| future::block_on(ex.run(shutdown.recv())))
        // Run the main future on the current thread.
        .finish(|| {
            ex.spawn(initialize(
                my_name_send,
                s_ex2,
                io_ex,
                signal,
                to_application_send,
                t_am_s,
                to_app_mgr_recv,
                d_clone,
                storage_neighbors,
            ))
            .detach();
            future::block_on(async move {
                local_ex
                    .run(run_app(
                        dir.clone(),
                        my_name_recv,
                        config,
                        to_app_mgr_send.clone(),
                        wrapped_sender,
                        wrapped_receiver,
                    ))
                    .await;
            })
        });

    Ok(())
}

async fn run_app(
    dir: PathBuf,
    my_name_recv: achannel::Receiver<SwarmName>,
    config: Configuration,
    to_app_mgr_send: achannel::Sender<ToAppMgr>,
    wrapped_sender: achannel::Sender<InternalMsg>,
    wrapped_receiver: achannel::Receiver<InternalMsg>,
) {
    let tui_mgr = instantiate_tui_mgr();
    eprintln!("Tui manager instantiated");
    let mut toolbox = Toolbox::empty();
    // TODO: When logic.run() is done, it returns Option<AppType>,
    //       and if that option is Some, another logic is started
    //       for that new AppType.
    //       Along with new logic, serve_tui_mgr task should be informed
    //       about user interface switch. Upon receiving that info,
    //       serve_tui_mgr should break it's loop and spawn a new one for AppType.
    //       Also new from_tui_adapter should be spawned,
    //       old one should self-terminate on error receiving FromPresentation.
    // TODO: InternalMessage should serve every defined AppType, and Notification
    let my_name = my_name_recv.recv().await.unwrap();
    eprintln!("Got my name: {}", my_name);
    let toolset = Toolset::fold(tui_mgr, config, None, None, None, None, None);
    let mut next_app = Some((
        Some(AppType::Catalog),
        my_name.clone(),
        wrapped_receiver,
        toolset,
        None,
    ));
    // TODO: Define a Toolbox struct to store all the tools an app might use.
    // Those are Editor, Notifier, Selector etc.
    // Once an app is done, it returns all the tools it was using back to toolbox.
    // When next app is strating, it borrows existing tools from Toolbox
    loop {
        if let Some((app_type, s_name, wrapped_receiver, toolset, clipboard_opt)) = next_app.take()
        {
            if app_type.is_none() {
                eprintln!("Dunno what AppType that is,terminating");
                // TODO: discover AppType
                break;
            }
            let app_type = app_type.unwrap();
            eprintln!("Next app: {} {:?}", s_name, app_type);
            let (mut tui_mgr, config, e_opt, c_opt, s_opt, i_opt, pe_opt) = toolset.unfold();
            toolbox.return_tools(e_opt, c_opt, s_opt, i_opt, pe_opt);
            match app_type {
                AppType::Catalog => {
                    tui_mgr.restore_display(0, false);
                    let c_logic = CatalogLogic::new(
                        s_name,
                        to_app_mgr_send.clone(),
                        &mut tui_mgr,
                        wrapped_sender.clone(),
                        wrapped_receiver,
                    );
                    let get_editor = true;
                    let get_creator = true;
                    let get_selector = true;
                    let get_indexer = true;
                    let get_policy_editor = false;
                    let toolset = toolbox.get_tools(
                        tui_mgr,
                        config,
                        get_editor,
                        get_creator,
                        get_selector,
                        get_indexer,
                        get_policy_editor,
                    );
                    next_app = c_logic.run(dir.clone(), toolset, clipboard_opt).await;
                }
                AppType::Forum => {
                    let f_logic = ForumLogic::new(
                        my_name.founder,
                        tui_mgr.screen_size(),
                        s_name,
                        to_app_mgr_send.clone(),
                        wrapped_sender.clone(),
                        wrapped_receiver,
                    );
                    let get_editor = true;
                    let get_creator = true;
                    let get_selector = true;
                    let get_indexer = false;
                    let get_policy_editor = true;
                    let toolset = toolbox.get_tools(
                        tui_mgr,
                        config,
                        get_editor,
                        get_creator,
                        get_selector,
                        get_indexer,
                        get_policy_editor,
                    );
                    next_app = f_logic
                        .run(my_name.founder, dir.clone(), toolset, clipboard_opt)
                        .await;
                }
                AppType::Other(_x) => {
                    //TODO
                    break;
                }
            }
        } else {
            // let _ = to_app_mgr_send.send(dapp_lib::ToAppMgr::Quit).await;
            break;
        }
    }
    eprintln!("Main loop is done.");
}

fn run_io_executor(io_executor: Arc<Executor<'_>>, shutdown: Receiver<()>) {
    Parallel::new()
        .each(0..2, |_| future::block_on(io_executor.run(shutdown.recv())))
        // Run the main future on the current thread.
        .finish(|| {
            // TODO
            eprintln!("In run io_executor");
        });
}

// async fn to_user_adapter(to_user: AReceiver<ToApp>, wrapped_sender: Sender<InternalMsg>) {
async fn to_user_adapter(
    to_user: achannel::Receiver<ToApp>,
    wrapped_sender: achannel::Sender<InternalMsg>,
) {
    eprintln!("User adapter started");
    while let Ok(to_app) = to_user.recv().await {
        let _ = wrapped_sender.send(InternalMsg::User(to_app)).await;
    }
    eprintln!("User adapter is done.");
}
