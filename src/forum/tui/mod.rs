use animaterm::prelude::Key;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::GnomeId;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use crate::catalog::tui::button::Button;
use crate::config::Configuration;
use crate::Toolset;

pub enum ToForumView {}
pub enum FromForumView {
    SwitchTo(AppType),
    Quit,
}
pub fn serve_forum_tui(
    my_id: GnomeId,
    toolset: Toolset,
    // mut tui_mgr: Manager,
    to_app: Sender<FromForumView>,
    // to_tui_send: Sender<ToPresentation>,
    to_tui_recv: Receiver<ToForumView>,
    // config: Configuration,
    // ) -> (Manager, Configuration) {
) -> Toolset {
    let (mut tui_mgr, config, e_opt, c_opt, s_opt, i_opt) = toolset.unfold();
    // let mut editor = e_opt.unwrap();
    let mut creator = c_opt.unwrap();
    // TODO: do not create a new display every time Forum App is opened
    let main_display = tui_mgr.new_display(true);
    let (cols, rows) = tui_mgr.screen_size();
    let mut frame = vec![Glyph::blue(); cols * rows];
    // Forum
    let button_1 = Button::new((10, 3), 2, (1, 0), " Filter", None, &mut tui_mgr);
    button_1.show(&mut tui_mgr);
    button_1.select(&mut tui_mgr, false);
    let button_2 = Button::new((10, 3), 2, (12, 0), "Add new", None, &mut tui_mgr);
    button_2.show(&mut tui_mgr);
    let button_3 = Button::new((10, 3), 2, (23, 0), "Options", None, &mut tui_mgr);
    button_3.show(&mut tui_mgr);
    let button_4 = Button::new((10, 3), 2, (34, 0), "→Village", None, &mut tui_mgr);
    button_4.show(&mut tui_mgr);
    let buttons = vec![button_1, button_2, button_3, button_4];
    let mut active_button = 0;
    let mut entry_buttons = Vec::with_capacity((rows - 4) >> 1);
    let mut active_entry = 0;
    let mut menu_active = true;
    for i in 0..(rows - 4) >> 1 {
        let e_button_1 = Button::new(
            (cols - 2, 1),
            2,
            (1, 4 + (i << 1) as isize),
            "No entry at the moment",
            None,
            &mut tui_mgr,
        );
        e_button_1.show(&mut tui_mgr);
        entry_buttons.push(e_button_1);
    }

    let mut library = HashMap::new();
    library.insert(0, frame);
    let bg = Graphic::new(cols, rows, 0, library, None);
    let bg_idx = tui_mgr.add_graphic(bg, 1, (0, 0)).unwrap();
    tui_mgr.set_graphic(bg_idx, 0, true);
    loop {
        if let Some(key) = tui_mgr.read_key() {
            match key {
                Key::Enter => {
                    if menu_active {
                        if active_button == 1 {
                            eprintln!("Trying to invoke Creator from Forum");
                            let read_only = false;
                            let d_type = "DT".to_string();
                            let tags = "TAGs".to_string();
                            let description = "descr".to_string();
                            let _result = creator.show(
                                main_display,
                                &mut tui_mgr,
                                read_only,
                                d_type,
                                tags,
                                description,
                            );
                            match _result {
                                crate::catalog::tui::CreatorResult::Create => {
                                    eprintln!("Creator wants to Create");
                                }
                                crate::catalog::tui::CreatorResult::Cancel => {
                                    eprintln!("Creator wants to Cancel");
                                }
                                crate::catalog::tui::CreatorResult::SelectDType => {
                                    eprintln!("Creator wants to SelectDType");
                                }
                                crate::catalog::tui::CreatorResult::SelectTags => {
                                    eprintln!("Creator wants to SelectTags");
                                }
                                crate::catalog::tui::CreatorResult::SelectDescription => {
                                    eprintln!("Creator wants to SelectDescription");
                                }
                            }
                            // let header = "Nagłówek";
                            // let initial_text = None;
                            // let allow_new_lines = true;
                            // let byte_limit = Some(1024);
                            // editor.set_mode((false, false));
                            // let _result = editor.serve(
                            //     // input_display,
                            //     main_display,
                            //     // &mut editor,
                            //     &header,
                            //     initial_text,
                            //     allow_new_lines,
                            //     limit,
                            //     &mut tui_mgr,
                            // );
                            // if let Some(text) = _result {
                            //     eprintln!("Forum Create result: {text}");
                            // }
                        } else if active_button == 3 {
                            let _ = to_app.send(FromForumView::SwitchTo(AppType::Catalog));
                            break;
                        } else {
                            print!("\x1b[2;60H Selected  menu: {active_button}     ");
                        }
                    } else {
                        print!("\x1b[2;60H Selected entry: {active_entry}     ");
                    }
                }
                Key::ShiftQ => {
                    eprintln!("Forum ShiftQ");
                    let _ = to_app.send(FromForumView::Quit);
                    break;
                }
                Key::C => {
                    eprintln!("Forum C");
                    let _ = to_app.send(FromForumView::SwitchTo(AppType::Catalog));
                    break;
                }
                Key::Right => {
                    if menu_active {
                        buttons[active_button].deselect(&mut tui_mgr, false);
                        active_button = (active_button + 1) % buttons.len();
                        buttons[active_button].select(&mut tui_mgr, false);
                    } else {
                        menu_active = true;
                        entry_buttons[active_entry].deselect(&mut tui_mgr, false);
                        buttons[active_button].select(&mut tui_mgr, false);
                    }
                }
                Key::Left => {
                    if menu_active {
                        buttons[active_button].deselect(&mut tui_mgr, false);
                        active_button = if active_button == 0 {
                            buttons.len() - 1
                        } else {
                            (active_button - 1) % buttons.len()
                        };
                        buttons[active_button].select(&mut tui_mgr, false);
                    } else {
                        menu_active = true;
                        entry_buttons[active_entry].deselect(&mut tui_mgr, false);
                        buttons[active_button].select(&mut tui_mgr, false);
                    }
                }
                Key::Up => {
                    if menu_active {
                        menu_active = false;
                        buttons[active_button].deselect(&mut tui_mgr, false);
                        entry_buttons[active_entry].select(&mut tui_mgr, false);
                    } else {
                        entry_buttons[active_entry].deselect(&mut tui_mgr, false);

                        active_entry = if active_entry == 0 {
                            entry_buttons.len() - 1
                        } else {
                            active_entry - 1
                        };
                        entry_buttons[active_entry].select(&mut tui_mgr, false);
                    }
                }
                Key::Down => {
                    if menu_active {
                        menu_active = false;
                        buttons[active_button].deselect(&mut tui_mgr, false);
                        entry_buttons[active_entry].select(&mut tui_mgr, false);
                    } else {
                        entry_buttons[active_entry].deselect(&mut tui_mgr, false);

                        active_entry = (active_entry + 1) % entry_buttons.len();
                        entry_buttons[active_entry].select(&mut tui_mgr, false);
                    }
                }
                _other => {
                    //TODO
                }
            }
        }
    }
    eprintln!("serve_forum_tui is done");
    // (tui_mgr, config)
    Toolset::fold(tui_mgr, config, None, None, None, None)
}
