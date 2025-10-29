use animaterm::prelude::Key;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::ByteSet;
use dapp_lib::prelude::Capabilities;
use dapp_lib::prelude::ContentID;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::Policy;
// use dapp_lib::prelude::Requirement;
use dapp_lib::prelude::SwarmName;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use crate::catalog::tui::button::Button;
use crate::catalog::tui::CreatorResult;
use crate::catalog::tui::EditorResult;
use crate::common::poledit::PolAction;
use crate::common::poledit::PolicyEditor;
use crate::common::poledit::ReqTree;
use crate::forum::logic::TopicContext;
use crate::Toolset;
pub struct EditorParams {
    pub title: String,
    pub initial_text: Option<String>,
    pub allow_newlines: bool,
    pub chars_limit: Option<Vec<char>>,
    pub text_limit: Option<u16>,
    pub read_only: bool,
}

#[derive(Debug)]
pub enum Action {
    // Generic actions
    AddNew(bool),
    Delete(u16),
    Edit(u16),
    NextPage,
    PreviousPage,
    FirstPage,
    LastPage,
    Filter(String),
    Query(u16),
    Run(Option<usize>),
    // Specific actions
    MainMenu,   // inform of viewing Topics & ask for first page
    Posts(u16), // inform & ask for first page of given type
    Settings,
    RunningPolicies,     // inform & ask for first page of given type
    StoredPolicies,      // inform & ask for first page of given type
    RunningCapabilities, // inform & ask for first page of given type
    StoredCapabilities,  // inform & ask for first page of given type
    ByteSets(bool),      // inform & ask for first page of given type
    // StoredByteSets,        // inform & ask for first page of given type
    PolicyAction(PolAction),
    Selected(Vec<usize>),
    EditorResult(EditorResult),
    CreatorResult(CreatorResult),
    FollowLink(SwarmName, ContentID, u16), //last is page id
}
pub enum ToForumView {
    TopicsPage(u16, Vec<(u16, String)>),
    PostsPage(u16, Vec<(u16, String)>),
    Request(Vec<(u16, String)>),
    RunningPoliciesPage(u16, Vec<(u16, String)>),
    StoredPoliciesPage(u16, Vec<(u16, String)>),
    RunningCapabilitiesPage(u16, Vec<(u16, String)>),
    StoredCapabilitiesPage(u16, Vec<(u16, String)>),
    RunningByteSetsPage(u16, Vec<(u16, String)>),
    StoredByteSetsPage(u16, Vec<(u16, String)>),
    ShowPolicy(Policy, ReqTree),
    ShowCapability(Capabilities, Vec<(u16, String)>),
    ShowByteSet(u8, ByteSet),
    Select(bool, Vec<String>, Vec<usize>), // bool indicates if only one can be selected
    OpenEditor(EditorParams),
    OpenCreator(TopicContext),
    Finish,
}
pub enum FromForumView {
    Act(Action),
    // RunningPolicies,
    // StoredPolicies,
    SwitchTo(AppType, SwarmName),
    CopyToClipboard(u16),
    Quit,
}
// TODO: make an elastic menu mgmt system.
// There should be a set of available button configurations.
// And every time a user presses a button then a button config
// can be optionally changed to a different one as defined by
// action under that pressed button.
// There can also be some message being sent to logic in order
// to retrieve or update something.
struct ButtonsLogic {
    menu_buttons: [(Button, bool); 7],
    entry_buttons: Vec<(Button, bool, u16)>,
    configs: HashMap<MenuType, MenuConfig>,
    current_config_id: MenuType,
    is_menu_active: bool,
    selected_menu_button: usize,
    selected_entry_button: usize,
}
impl ButtonsLogic {
    pub fn new(tui_mgr: &mut Manager) -> Self {
        // TODO:
        // 1. Category filter
        // 2. Filter by text
        // 3. + Topic
        // 4. Options
        // 5. →Village
        //
        // 6. 7. unocuppied
        // (in options +Category)
        let button_1 = Button::new((10, 3), 2, (1, 0), " CatFltr", None, tui_mgr);
        button_1.show(tui_mgr);
        button_1.select(tui_mgr, false);
        let button_2 = Button::new((10, 3), 2, (12, 0), "Filter", None, tui_mgr);
        button_2.show(tui_mgr);
        let button_3 = Button::new((10, 3), 2, (23, 0), "+ Topic", None, tui_mgr);
        button_3.show(tui_mgr);
        let button_4 = Button::new((10, 3), 2, (34, 0), "Options", None, tui_mgr);
        button_4.show(tui_mgr);
        let button_5 = Button::new((10, 3), 2, (45, 0), "→Village", None, tui_mgr);
        button_5.show(tui_mgr);
        let button_6 = Button::new((10, 3), 2, (56, 0), "…More", None, tui_mgr);
        button_6.show(tui_mgr);
        let button_7 = Button::new((10, 3), 2, (67, 0), "…Actions", None, tui_mgr);
        button_7.show(tui_mgr);
        let menu_buttons = [
            (button_1, true),
            (button_2, true),
            (button_3, true),
            (button_4, true),
            (button_5, true),
            (button_6, true),
            (button_7, true),
        ];
        let mut configs: HashMap<MenuType, MenuConfig> = HashMap::new();
        // TODO: A Button should have a function to become grayed-out
        // so that when we switch from menu to entries we know what
        // we are operating on
        // TODO: There is 9 MenuConfigs total:
        // 0 – Main menu: here you can see all topics
        // 1 – Topic menu: here you can see all posts in a topic
        //     buttons to be determined
        // 2 – Settings menu
        //     only one button to go back to main menu
        //     and six menu items to jump to other submenus
        // 3 – Running Policies menu
        // 4 – Running Capabilities menu
        // 5 – Running ByteSets menu
        // 6 – Stored Policies menu
        // 7 – Stored Capabilities menu
        // 8 – Stored ByteSets menu
        configs.insert(
            MenuType::Main,
            MenuConfig::new(
                [
                    ButtonState::Show("CatFlter".to_string()),
                    ButtonState::Show("Filter".to_string()),
                    ButtonState::Show("+ Topic".to_string()),
                    ButtonState::Show("Options".to_string()),
                    ButtonState::Show("→ Village".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::AllTopics),
            ),
        );
        configs.insert(
            MenuType::Topic,
            MenuConfig::new(
                [
                    ButtonState::Show("New post".to_string()),
                    ButtonState::Show("Edit".to_string()),
                    ButtonState::Hide,
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::AllPosts),
            ),
        );
        configs.insert(
            MenuType::Settings,
            MenuConfig::new(
                [
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Show("Requests".to_string()),
                    ButtonState::Show("+Categry".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::Fixed(vec![
                    (
                        ButtonState::Show("Running Policies".to_string()),
                        EntryAction::RunningPolicies,
                    ),
                    (
                        ButtonState::Show("Running Capabilities".to_string()),
                        EntryAction::RunningCapabilities,
                    ),
                    (
                        ButtonState::Show("Running Byte Sets".to_string()),
                        EntryAction::RunningByteSets,
                    ),
                    (
                        ButtonState::Show("Stored Policies".to_string()),
                        EntryAction::StoredPolicies,
                    ),
                    (
                        ButtonState::Show("Stored Capabilities".to_string()),
                        EntryAction::StoredCapabilities,
                    ),
                    (
                        ButtonState::Show("Stored Byte Sets".to_string()),
                        EntryAction::StoredByteSets,
                    ),
                    (
                        ButtonState::Show("Change Forum's description".to_string()),
                        EntryAction::EditDescription,
                    ),
                ]),
            ),
        );
        configs.insert(
            MenuType::Requests,
            MenuConfig::new(
                [
                    ButtonState::Show("Approve".to_string()),
                    ButtonState::Show("Reject".to_string()),
                    ButtonState::Show("← Settings".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::AllTopics),
            ),
        );
        configs.insert(
            MenuType::RunningPolicies,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActivePolicy),
            ),
        );
        configs.insert(
            MenuType::RunningCapabilities,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Show("Add Cap".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActiveCapability),
            ),
        );
        configs.insert(
            MenuType::RunningByteSets,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Show("New 1Bte".to_string()),
                    ButtonState::Show("New 2Bts".to_string()),
                    ButtonState::Show("Run".to_string()),
                    ButtonState::Show("Store".to_string()),
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActiveByteSet),
            ),
        );
        configs.insert(
            MenuType::StoredPolicies,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::StoredPolicy),
            ),
        );
        configs.insert(
            MenuType::StoredCapabilities,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::StoredCapability),
            ),
        );
        configs.insert(
            MenuType::StoredByteSets,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::StoredByteSet),
            ),
        );
        // 1 - Back
        // 2 - Filter
        // 3 - Add
        // 4 - Modify
        // 5 - Delete
        // 6 - Run
        // 7 - Store
        configs.insert(
            MenuType::Capability,
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("Filter".to_string()),
                    ButtonState::Show("Add".to_string()),
                    ButtonState::Show("Modify".to_string()),
                    ButtonState::Show("Delete".to_string()),
                    ButtonState::Show("Run".to_string()),
                    ButtonState::Show("Store".to_string()),
                ],
                EntriesState::QueryLogic(QueryType::StoredByteSet),
            ),
        );

        let (cols, rows) = tui_mgr.screen_size();
        let mut entry_buttons = Vec::with_capacity((rows - 4) >> 1);
        for i in 0..(rows - 4) >> 1 {
            let e_button_1 = Button::new(
                (cols - 2, 1),
                2,
                (1, 4 + (i << 1) as isize),
                "Empty",
                None,
                tui_mgr,
            );
            e_button_1.show(tui_mgr);
            entry_buttons.push((e_button_1, true, 255));
        }
        ButtonsLogic {
            menu_buttons,
            entry_buttons,
            configs,
            current_config_id: MenuType::Main,
            is_menu_active: true,
            selected_menu_button: 0,
            selected_entry_button: 0,
        }
    }
    pub fn is_current_config_equal(&self, conf_id: MenuType) -> bool {
        self.current_config_id == conf_id
    }
    pub fn activate(&mut self, tui_mgr: &mut Manager) -> Option<Action> {
        if self.is_menu_active {
            match self.current_config_id {
                MenuType::Main => {
                    match self.selected_menu_button {
                        0 => {
                            //TODO: Category Filter
                            // Read all Categories(Tags)
                            // from Manifest
                            // Open up Selector and let
                            // user choose categories.
                            // Filter Topics to only include
                            // Topics marked with at least
                            // one of selected Catagories.
                            None
                        }
                        1 => {
                            //TODO: Filter
                            // Open Editor and read user
                            // input.
                            // Filter Topics to only include
                            // those containing at least one
                            // word from user-defined Filter
                            None
                        }
                        2 => {
                            //TODO: Add new topic
                            // Open Creator and allow user
                            // to define a new topic.
                            Some(Action::AddNew(false))
                        }
                        3 => {
                            //TODO: Options
                            // Allow definition of new
                            // Category (Tag)
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        4 => {
                            // TODO: Village
                            // Improve this logic.
                            Some(Action::FollowLink(
                                SwarmName::new(GnomeId::any(), format!("/")).unwrap(),
                                0,
                                0,
                            ))
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                // We need a way to CRUD Policies, Capabilities, ByteSets.
                // There are stored Policies, Caps & BSets,
                // and there are active ones.
                // Active settings are read from Swarm's Configuration.
                // Based on active settings we make decision of which
                // actions should be available to the user.
                // By default a user is only allowed to Read settings,
                // both active & stored.
                // Creation, Update & Deletion of any setting should
                // be disabled by default.
                //
                //
                MenuType::Topic => {
                    // TODO: define buttons
                    match self.selected_menu_button {
                        0 => {
                            // Add Post
                            Some(Action::AddNew(false))
                        }
                        1 => {
                            // Test button for UserDefined SyncMessage posting
                            Some(Action::Edit(self.selected_entry_button as u16))
                        }
                        2 => {
                            // self.activate_menu(4, tui_mgr);
                            // self.activate_menu(MenuType::RunningCapabilities, tui_mgr);
                            None
                        }
                        3 => {
                            // Back to main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::Settings => {
                    // TODO: maybe some more buttons?
                    match self.selected_menu_button {
                        0 => {
                            // Back to menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        1 => {
                            // Show Menu for 2-steps Requests
                            self.activate_menu(MenuType::Requests, tui_mgr);
                            Some(Action::Edit(0))
                        }
                        2 => {
                            // Open Editor for a new Category/Tag
                            Some(Action::Edit(1))
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::Requests => {
                    match self.selected_menu_button {
                        0 => {
                            // TODO: Approve
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Edit(0))
                        }
                        1 => {
                            // TODO: Reject
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::Delete(0))
                        }
                        2 => {
                            // Back to Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        _o => {
                            eprintln!("Menu Button id {_o} not supported in Requests ");
                            None
                        }
                    }
                }
                MenuType::RunningPolicies => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            //hidden
                            None
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::RunningCapabilities => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            // Add new Capability
                            Some(Action::AddNew(false))
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::RunningByteSets => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            //New 1 Byte
                            Some(Action::AddNew(false))
                        }
                        3 => {
                            //New 2 Bytes
                            Some(Action::AddNew(true))
                        }
                        4 => {
                            //Run selected
                            Some(Action::Run(Some(self.selected_entry_button)))
                        }
                        5 => {
                            //Store selected
                            // Some(Action::Store)
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::StoredPolicies => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            //hidden
                            None
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::StoredCapabilities => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            //hidden
                            None
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::StoredByteSets => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(MenuType::Main, tui_mgr);
                            Some(Action::MainMenu)
                        }
                        2 => {
                            //hidden
                            None
                        }
                        3 => {
                            //hidden
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                MenuType::Capability => {
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(MenuType::Settings, tui_mgr);
                            Some(Action::Settings)
                        }
                        1 => None,
                        2 => Some(Action::AddNew(false)),
                        3 => Some(Action::Query(
                            self.entry_buttons
                                .get(self.selected_entry_button)
                                .unwrap()
                                .2,
                        )),
                        4 => Some(Action::Delete(
                            self.entry_buttons
                                .get(self.selected_entry_button)
                                .unwrap()
                                .2,
                        )),
                        5 => Some(Action::Run(None)),
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
            }
        } else {
            // logic when entry was selected
            // there can be three types of entries selected
            // (two, after bugfix):
            // – a fixed entry from settings menu
            // – a query logic entry
            // – a hidden entry that should not be selected…
            let menu = self.configs.get(&self.current_config_id).unwrap();
            let action = menu.entry_action(self.selected_entry_button);
            match action {
                EntryAction::Query => {
                    // TODO:first we have to figure out which menu should get
                    // activated
                    let action = Action::Query(
                        self.entry_buttons
                            .get(self.selected_entry_button)
                            .unwrap()
                            .2,
                    );
                    match self.current_config_id {
                        MenuType::Main => {
                            // TODO: switch to Topic
                        }
                        MenuType::Topic => {
                            // TODO: switch to Post
                        }
                        MenuType::Settings => {
                            // DONE: switch to Settings (dedicated logic)
                        }
                        MenuType::Requests => {
                            // TODO/DONE ?
                        }
                        MenuType::RunningPolicies => {
                            // DONE: switch to Pyramid (dedicated logic)
                        }
                        MenuType::RunningCapabilities => {
                            // TODO: switch to Capability
                            self.activate_menu(MenuType::Capability, tui_mgr);
                        }
                        MenuType::RunningByteSets => {
                            // TODO: switch to ByteSet
                        }
                        MenuType::StoredPolicies => {
                            // TODO: switch to Stored Policy
                        }
                        MenuType::StoredCapabilities => {
                            // TODO: switch to Stored Capability
                        }
                        MenuType::StoredByteSets => {
                            // TODO: switch to Stored Byte Set
                        }
                        MenuType::Capability => {
                            // TODO: switch to Edit selected GnomeId
                        }
                    }
                    Some(action)
                }
                EntryAction::RunningPolicies => {
                    self.activate_menu(MenuType::RunningPolicies, tui_mgr);
                    Some(Action::RunningPolicies)
                }
                EntryAction::RunningCapabilities => {
                    self.activate_menu(MenuType::RunningCapabilities, tui_mgr);
                    Some(Action::RunningCapabilities)
                }
                EntryAction::RunningByteSets => {
                    self.activate_menu(MenuType::RunningByteSets, tui_mgr);
                    Some(Action::ByteSets(true))
                }
                EntryAction::StoredPolicies => {
                    self.activate_menu(MenuType::StoredPolicies, tui_mgr);
                    Some(Action::StoredPolicies)
                }
                EntryAction::StoredCapabilities => {
                    self.activate_menu(MenuType::StoredCapabilities, tui_mgr);
                    Some(Action::StoredCapabilities)
                }
                EntryAction::StoredByteSets => {
                    self.activate_menu(MenuType::StoredByteSets, tui_mgr);
                    Some(Action::ByteSets(false))
                }
                EntryAction::EditDescription => {
                    self.activate_menu(MenuType::Main, tui_mgr);
                    Some(Action::Query(0))
                }
                EntryAction::NoAction => None,
            }
        }
    }
    pub fn update_entries(&mut self, mut new_list: Vec<(u16, String)>, tui_mgr: &mut Manager) {
        // TODO:
        eprintln!("New list len: {:?}", new_list);
        for butn in &mut self.entry_buttons {
            if !new_list.is_empty() {
                let (id, text) = new_list.remove(0);
                butn.0.hide(tui_mgr);
                butn.0.rename(tui_mgr, &text);
                butn.0.show(tui_mgr);
                butn.1 = true;
                butn.2 = id;
            } else {
                butn.0.hide(tui_mgr);
                butn.1 = false;
            }
        }
        if !new_list.is_empty() {
            eprintln!("Received overflowing entries page!");
        }
    }

    fn up(&mut self, tui_mgr: &mut Manager) -> bool {
        let mut prev_page = false;
        if self.is_menu_active {
            self.is_menu_active = false;
            self.menu_buttons[self.selected_menu_button]
                .0
                .deselect(tui_mgr, false);
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        } else {
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, false);

            prev_page = self.select_prev_entry();
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        }
        prev_page
    }
    fn down(&mut self, tui_mgr: &mut Manager) -> bool {
        let mut next_page = false;
        if self.is_menu_active {
            self.is_menu_active = false;
            self.menu_buttons[self.selected_menu_button]
                .0
                .deselect(tui_mgr, false);
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        } else {
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, false);

            next_page = self.select_next_entry();
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        }
        next_page
    }
    fn left(&mut self, tui_mgr: &mut Manager) {
        if self.is_menu_active {
            self.menu_buttons[self.selected_menu_button]
                .0
                .deselect(tui_mgr, false);
            self.select_prev_menu();
            self.menu_buttons[self.selected_menu_button]
                .0
                .select(tui_mgr, false);
        } else {
            self.is_menu_active = true;
            let set_to_alt = self.current_config_id == MenuType::Capability
                || self.current_config_id == MenuType::RunningByteSets;
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, set_to_alt);
            self.menu_buttons[self.selected_menu_button]
                .0
                .select(tui_mgr, false);
        }
    }
    fn right(&mut self, tui_mgr: &mut Manager) {
        if self.is_menu_active {
            self.menu_buttons[self.selected_menu_button]
                .0
                .deselect(tui_mgr, false);
            self.select_next_menu();
            self.menu_buttons[self.selected_menu_button]
                .0
                .select(tui_mgr, false);
        } else {
            self.is_menu_active = true;
            let set_to_alt = self.current_config_id == MenuType::Capability
                || self.current_config_id == MenuType::RunningByteSets;
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, set_to_alt);
            self.menu_buttons[self.selected_menu_button]
                .0
                .select(tui_mgr, false);
        }
    }
    fn select_next_menu(&mut self) {
        for i in 1..=self.menu_buttons.len() {
            let next_id = (self.selected_menu_button + i) % self.menu_buttons.len();
            if self.menu_buttons[next_id].1 {
                self.selected_menu_button = next_id;
                break;
            }
        }
    }
    fn select_prev_menu(&mut self) {
        for i in 1..=self.menu_buttons.len() {
            let next_id =
                (self.selected_menu_button + self.menu_buttons.len() - i) % self.menu_buttons.len();
            if self.menu_buttons[next_id].1 {
                self.selected_menu_button = next_id;
                break;
            }
        }
    }
    fn select_next_entry(&mut self) -> bool {
        for i in 1..=self.entry_buttons.len() {
            let next_id = (self.selected_entry_button + i) % self.entry_buttons.len();
            if self.entry_buttons[next_id].1 {
                self.selected_entry_button = next_id;
                break;
            }
        }
        self.selected_entry_button == 0
    }
    fn select_prev_entry(&mut self) -> bool {
        for i in 1..=self.entry_buttons.len() {
            let prev_id = (self.selected_entry_button + self.entry_buttons.len() - i)
                % self.entry_buttons.len();
            if self.entry_buttons[prev_id].1 {
                self.selected_entry_button = prev_id;
                break;
            }
        }
        self.selected_entry_button == self.last_visible_button_idx()
    }

    fn last_visible_button_idx(&self) -> usize {
        for i in (0..self.entry_buttons.len()).rev() {
            if self.entry_buttons[i].1 {
                return i;
            }
        }
        return 0;
    }
    fn activate_menu(&mut self, cfg_idx: MenuType, tui_mgr: &mut Manager) {
        self.is_menu_active = true;
        if let Some(ms) = self.configs.get(&cfg_idx) {
            self.current_config_id = cfg_idx;
            self.selected_menu_button = 0;
            self.selected_entry_button = 0;

            for i in 0..7 {
                match &ms.menu[i] {
                    ButtonState::Hide => {
                        self.menu_buttons[i].0.hide(tui_mgr);
                        self.menu_buttons[i].1 = false;
                    }
                    ButtonState::Show(text) => {
                        self.menu_buttons[i].1 = true;
                        self.menu_buttons[i].0.hide(tui_mgr);
                        self.menu_buttons[i].0.rename(tui_mgr, text);
                        self.menu_buttons[i].0.show(tui_mgr);
                    }
                }
            }
            self.menu_buttons[0].0.select(tui_mgr, false);
            match &ms.entries {
                EntriesState::Fixed(new_states) => {
                    //TODO
                    let mut i = 0;
                    for b in &mut self.entry_buttons {
                        if let Some(new_state) = new_states.get(i) {
                            match new_state {
                                (ButtonState::Hide, _a) => {
                                    b.1 = false;
                                    b.0.hide(tui_mgr);
                                }
                                (ButtonState::Show(text), _a) => {
                                    b.1 = true;
                                    b.0.hide(tui_mgr);
                                    b.0.rename(tui_mgr, text);
                                    b.0.show(tui_mgr);
                                }
                            }
                        } else {
                            b.1 = false;
                            b.0.hide(tui_mgr);
                        }
                        i = i + 1;
                    }
                }
                EntriesState::QueryLogic(_qt) => {
                    for b in &mut self.entry_buttons {
                        b.1 = false;
                        b.0.hide(tui_mgr);
                    }
                }
                EntriesState::HideAll => {
                    for b in &mut self.entry_buttons {
                        b.1 = false;
                        b.0.hide(tui_mgr);
                    }
                }
            }

            //TODO
        } else {
            eprintln!("Could not activate menu: {}", self.selected_menu_button);
        }
    }
}

enum ButtonState {
    Hide,
    Show(String),
}
enum EntriesState {
    HideAll,
    Fixed(Vec<(ButtonState, EntryAction)>),
    QueryLogic(QueryType),
}
#[derive(Clone, Copy)]
enum EntryAction {
    NoAction,
    RunningPolicies,
    RunningCapabilities,
    RunningByteSets,
    StoredPolicies,
    StoredCapabilities,
    StoredByteSets,
    EditDescription,
    Query,
}
#[derive(Clone, Copy, Debug)]
pub enum QueryType {
    AllTopics,
    Topic(u8),
    AllPosts,
    Post(u8),
    ActivePolicy,
    ActiveCapability,
    ActiveByteSet,
    StoredPolicy,
    StoredCapability,
    StoredByteSet,
}

#[derive(Eq, PartialEq, Debug, Hash)]
enum MenuType {
    Main,
    Topic,
    Settings,
    Requests,
    RunningPolicies,
    RunningCapabilities,
    RunningByteSets,
    StoredPolicies,
    StoredCapabilities,
    StoredByteSets,
    Capability,
}
struct MenuConfig {
    menu: [ButtonState; 7],
    entries: EntriesState,
    // button_to_id
}

impl MenuConfig {
    pub fn new(menu: [ButtonState; 7], entries: EntriesState) -> Self {
        MenuConfig { menu, entries }
    }
    pub fn entry_action(&self, id: usize) -> EntryAction {
        match &self.entries {
            EntriesState::HideAll => EntryAction::NoAction,
            EntriesState::QueryLogic(_qt) => EntryAction::Query,
            EntriesState::Fixed(list) => {
                if let Some((_b, a)) = list.get(id) {
                    *a
                } else {
                    EntryAction::NoAction
                }
            }
        }
    }
}
pub fn serve_forum_tui(
    _my_id: GnomeId,
    toolset: Toolset,
    // mut tui_mgr: Manager,
    to_app: Sender<FromForumView>,
    // to_tui_send: Sender<ToPresentation>,
    to_tui_recv: Receiver<ToForumView>,
    // config: Configuration,
    // ) -> (Manager, Configuration) {
) -> Toolset {
    let (mut tui_mgr, config, e_opt, c_opt, s_opt, _i_opt, pe_opt) = toolset.unfold();
    let mut creator = c_opt.unwrap();
    let mut selector = s_opt.unwrap();

    // TODO: PEditor should be created once upon
    // startup & should be passed to an app
    // together with other tools
    let mut editor = e_opt.unwrap();
    let mut pedit = if let Some(pe) = pe_opt {
        pe
    } else {
        PolicyEditor::new(&mut tui_mgr)
    };
    // TODO: do not create a new display every time Forum App is opened
    let main_display = tui_mgr.new_display(true);
    let (cols, rows) = tui_mgr.screen_size();
    let frame = vec![Glyph::blue(); cols * rows];
    // Forum
    let mut buttons_logic = ButtonsLogic::new(&mut tui_mgr);
    let mut action = buttons_logic.activate(&mut tui_mgr);
    // let mut active_button = 0;
    // let mut entry_buttons = Vec::with_capacity((rows - 4) >> 1);
    // let mut active_entry = 0;
    // let mut menu_active = true;
    // TODO: maybe use main menu for additional tasks such as:
    // – Policy
    // – Capabilities
    // – ByteSets
    // TODO: We have two separate sets of above:
    // – one is Swarm's running Configuration,
    // – the other being what is stored in Manifest's Data blocks.
    // So we need to be able to present both of these.
    // User could define a new Policy, but not set it to active.
    // User could also activate a temporary Policy that will not
    // be stored in Manifest's Data blocks.
    // For example some users may not have Capabilities to modify
    // CID=0 (Manifest), but they are allowed to send Reconfigure
    // packages. An Admin might be an example.
    //
    // So we need our Menu to be able to do all of the above.
    // This could show up once user presses Options menu button.
    // Then all the Buttons would change, as well as contents of Entries
    // Once user presses e.g. Policy button everything changes once again.
    // Now entry buttons list all Policies that have been defined and are
    // active.
    // If a user chooses a Policy by pressing it's button, then a new
    // window opens to present given Policy and it's Requirements.
    // From that menu user

    // let menu_id = 0;
    // activate_menu(
    //     menu_id,
    //     &mut menu_sets,
    //     &mut tui_mgr,
    //     &mut menu_buttons,
    //     &mut entry_buttons,
    // );
    let mut library = HashMap::new();
    library.insert(0, frame);
    let bg = Graphic::new(cols, rows, 0, library, None);
    let bg_idx = tui_mgr.add_graphic(bg, 1, (0, 0)).unwrap();
    tui_mgr.set_graphic(bg_idx, 0, true);
    loop {
        if let Some(act) = action.take() {
            eprintln!("Some action: {:?}", act);
            let _ = to_app.send(FromForumView::Act(act));
        }
        if let Some(key) = tui_mgr.read_key() {
            match key {
                Key::Enter => {
                    action = buttons_logic.activate(&mut tui_mgr);
                }
                Key::ShiftQ => {
                    eprintln!("Forum ShiftQ");
                    let _ = to_app.send(FromForumView::Quit);
                    break;
                }
                Key::C => {
                    let s_name = SwarmName::new(GnomeId::any(), "/".to_string()).unwrap();
                    eprintln!("Forum C");
                    let _ = to_app.send(FromForumView::SwitchTo(AppType::Catalog, s_name));
                    break;
                }
                Key::Right | Key::W => {
                    let _action = buttons_logic.right(&mut tui_mgr);
                }
                Key::Left | Key::P => {
                    let _action = buttons_logic.left(&mut tui_mgr);
                }
                Key::Up | Key::O => {
                    if buttons_logic.up(&mut tui_mgr) {
                        // TODO: open prev page
                        action = Some(Action::PreviousPage);
                    }
                }
                Key::Down | Key::Comma => {
                    if buttons_logic.down(&mut tui_mgr) {
                        // TODO: open next page
                        action = Some(Action::NextPage);
                    }
                }
                Key::Home => {
                    action = Some(Action::FirstPage);
                }
                Key::PgUp => {
                    action = Some(Action::PreviousPage);
                }
                Key::PgDn => {
                    action = Some(Action::NextPage);
                }
                Key::End => {
                    action = Some(Action::LastPage);
                }
                Key::F5 => {
                    let _ = to_app.send(FromForumView::CopyToClipboard(
                        buttons_logic.selected_entry_button as u16,
                    ));
                }
                _other => {
                    //TODO
                }
            }
        }
        let to_tui_res = to_tui_recv.try_recv();
        if let Ok(msg) = to_tui_res {
            //TODO
            match msg {
                ToForumView::TopicsPage(_pg_id, topics) => {
                    buttons_logic.activate_menu(MenuType::Main, &mut tui_mgr);
                    buttons_logic.update_entries(topics, &mut tui_mgr);
                }
                ToForumView::PostsPage(_pg_id, topics) => {
                    buttons_logic.activate_menu(MenuType::Topic, &mut tui_mgr);
                    buttons_logic.update_entries(topics, &mut tui_mgr);
                }
                ToForumView::Request(v_req) => {
                    buttons_logic.activate_menu(MenuType::Requests, &mut tui_mgr);
                    buttons_logic.update_entries(v_req, &mut tui_mgr);
                }
                ToForumView::RunningPoliciesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::RunningPolicies) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                    }
                }
                ToForumView::StoredPoliciesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::StoredPolicies) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                        // TODO: present them to user if in right menu
                        // we might be showing stored config by this time…
                    }
                }
                ToForumView::RunningCapabilitiesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::RunningCapabilities) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                    }
                }
                ToForumView::StoredCapabilitiesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::StoredCapabilities) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                        // TODO: present them to user if in right menu
                        // we might be showing stored config by this time…
                    }
                }
                ToForumView::RunningByteSetsPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::RunningByteSets) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                    }
                }
                ToForumView::StoredByteSetsPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(MenuType::StoredByteSets) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                        // TODO: present them to user if in right menu
                        // we might be showing stored config by this time…
                    }
                }
                ToForumView::ShowPolicy(pol, req) => {
                    eprintln!("Forum TUI Policy: {:?} – {:?} to present", pol, req);

                    if let Some(p_action) = pedit.present(pol, req, &mut tui_mgr) {
                        // TODO
                        action = Some(Action::PolicyAction(p_action));
                    } else {
                        action = Some(Action::Settings);
                    }
                    tui_mgr.restore_display(main_display, true);
                    buttons_logic.activate_menu(MenuType::Settings, &mut tui_mgr);
                }
                ToForumView::ShowCapability(_cap, v_gids) => {
                    // TODO: First approach is to reuse existing Selector.
                    // One item could be for adding new GnomeId into this Cap.
                    // But we can not currently apply dedicated logic for
                    // single selector's item…
                    // Maybe we should reuse Forum's menu logic instead?
                    // Top buttons could allow for Addition, Deletion and
                    // Edition of an entry.
                    // We should only slightly modify existing logic so that
                    // when we are navigating through top Menu buttons,
                    // element on main list would need to stay selected,
                    // maybe grayed out.
                    // This way when we want to delete/modify an item
                    // we see which one is going to be affected.
                    // And when we select one of those actions then Editor
                    // shows up and allow us to type in a one liner with
                    // exactly 16 chars all of them from a restricted set
                    // of characters that GnomeId can have.
                    // Once we're done, we exit Editor & an updated item
                    // shows up on Capability Menu.
                    // Menu should have up to 7 Buttons:
                    // 1 - Back
                    // 2 - Filter
                    // 3 - Add
                    // 4 - Modify
                    // 5 - Delete
                    // 6 - Run
                    // 7 - Store
                    if buttons_logic.is_current_config_equal(MenuType::Capability) {
                        buttons_logic.update_entries(v_gids, &mut tui_mgr);
                        // TODO: present them to user if in right menu
                        // we might be showing stored config by this time…
                    }
                    eprintln!("ForumTUI should present cap",);
                }
                ToForumView::ShowByteSet(bs_id, _bs) => {
                    // TODO
                    eprintln!("ForumTUI should present ByteSet({bs_id})");
                }
                ToForumView::Select(only_one, list, preselected) => {
                    let text = if only_one {
                        "Pick one & press Enter"
                    } else {
                        "Pick many with Enter, Escape to finish"
                    };
                    let selected =
                        selector.select(text, &list, preselected, &mut tui_mgr, only_one);
                    // TODO: bring up selector with
                    // a given list & reply back to logic.
                    eprintln!("Selected: {:?}", selected);
                    tui_mgr.restore_display(main_display, true);
                    if !selected.is_empty() {
                        action = Some(Action::Selected(selected));
                    } else {
                        //TODO:
                        eprintln!("Failed to select one!");
                    }
                }
                ToForumView::OpenEditor(e_p) => {
                    // let _ = editor.take_text(&mut tui_mgr);
                    editor.show(&mut tui_mgr);
                    editor.set_title(&mut tui_mgr, &e_p.title);
                    editor.set_limit(e_p.text_limit);
                    editor.allow_newlines(e_p.allow_newlines);

                    if let Some(text) = e_p.initial_text {
                        editor.set_text(&mut tui_mgr, &text);
                    }
                    editor.set_mode((e_p.read_only, e_p.read_only));
                    editor.move_to_line_end(&mut tui_mgr);
                    let e_res = editor.run(&mut tui_mgr);
                    action = Some(Action::EditorResult(e_res));
                    tui_mgr.restore_display(main_display, true);
                }
                ToForumView::OpenCreator(t_ctx) => {
                    eprintln!("Forum should open creator");
                    let res = creator.show(
                        main_display,
                        &mut tui_mgr,
                        false,
                        format!("Topic"),
                        format!(""),
                        t_ctx.description,
                    );
                    action = Some(Action::CreatorResult(res));
                }
                ToForumView::Finish => {
                    eprintln!("Forum is finished");
                    break;
                }
            }
            eprintln!("Forum TUI recv");
        } else {
            let err = to_tui_res.err().unwrap();
            match err {
                std::sync::mpsc::TryRecvError::Empty => {}
                std::sync::mpsc::TryRecvError::Disconnected => {
                    eprintln!("Forum TUI got Disconnected error.");
                    break;
                }
            }
        }
    }
    eprintln!("serve_forum_tui is done");
    // (tui_mgr, config)
    Toolset::fold(tui_mgr, config, None, None, None, None, Some(pedit))
}

// fn take_action(to_app: Sender<FromForumView>, action: Action) {
//     match action {
//         Action::RunningPolicies => {
//             let _ = to_app.send(FromForumView::RunningPolicies);
//         }
//         Action::StoredPolicies => {
//             let _ = to_app.send(FromForumView::StoredPolicies);
//         }
//         Action::RunningCapabilities => {
//             let _ = to_app.send(FromForumView::RunningPolicies);
//         }
//         Action::StoredCapabilities => {
//             let _ = to_app.send(FromForumView::StoredPolicies);
//         }
//         Action::RunningByteSets => {
//             let _ = to_app.send(FromForumView::RunningPolicies);
//         }
//         Action::StoredByteSets => {
//             let _ = to_app.send(FromForumView::StoredPolicies);
//         }
//         Action::NextPage => {
//             //TODO
//         }
//         Action::PreviousPage => {
//             //TODO
//         }
//         Action::FirstPage => {
//             //TODO
//         }
//         Action::LastPage => {
//             //TODO
//         }
//         Action::Filter(filter) => {
//             //TODO
//         }
//         Action::Query(qt) => {
//             //TODO: put logic here
//             // TODO: do we need to parameterize this action?
//             // Logic knows current state of presentation layer,
//             // so sending an index should be enough to retrieve
//             // desired data.
//         }
//         Action::Topics => {
//             //TODO
//         }
//         Action::Posts(topic_id) => {
//             //TODO
//         }
//     }
// }
