use animaterm::prelude::Key;
use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Manager;
use dapp_lib::prelude::AppType;
use dapp_lib::prelude::GnomeId;
use dapp_lib::prelude::Policy;
use dapp_lib::prelude::Requirement;
use dapp_lib::prelude::SwarmName;
use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use crate::catalog::tui::button::Button;
use crate::common::poledit::PolAction;
use crate::common::poledit::PolicyEditor;
use crate::common::poledit::ReqTree;
use crate::Toolset;

#[derive(Debug)]
pub enum Action {
    // Generic actions
    NextPage,
    PreviousPage,
    FirstPage,
    LastPage,
    Filter(String),
    Query(u16),
    // Specific actions
    Topics,              // inform of viewing Topics & ask for first page
    Posts(u16),          // inform & ask for first page of given type
    RunningPolicies,     // inform & ask for first page of given type
    StoredPolicies,      // inform & ask for first page of given type
    RunningCapabilities, // inform & ask for first page of given type
    StoredCapabilities,  // inform & ask for first page of given type
    RunningByteSets,     // inform & ask for first page of given type
    StoredByteSets,      // inform & ask for first page of given type
    PolicyAction(PolAction),
    OneSelected(usize),
}
pub enum ToForumView {
    RunningPoliciesPage(u16, Vec<(u16, String)>),
    StoredPoliciesPage(u16, Vec<(u16, String)>),
    ShowPolicy(Policy, ReqTree),
    SelectOne(Vec<String>),
}
pub enum FromForumView {
    Act(Action),
    // RunningPolicies,
    // StoredPolicies,
    SwitchTo(AppType, SwarmName),
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
    menu_buttons: [(Button, bool); 4],
    entry_buttons: Vec<(Button, bool, u16)>,
    configs: HashMap<u8, MenuConfig>,
    current_config_id: u8,
    is_menu_active: bool,
    selected_menu_button: usize,
    selected_entry_button: usize,
}
impl ButtonsLogic {
    pub fn new(tui_mgr: &mut Manager) -> Self {
        // TODO
        let button_1 = Button::new((10, 3), 2, (1, 0), " Filter", None, tui_mgr);
        button_1.show(tui_mgr);
        button_1.select(tui_mgr, false);
        let button_2 = Button::new((10, 3), 2, (12, 0), "Add new", None, tui_mgr);
        button_2.show(tui_mgr);
        let button_3 = Button::new((10, 3), 2, (23, 0), "Options", None, tui_mgr);
        button_3.show(tui_mgr);
        let button_4 = Button::new((10, 3), 2, (34, 0), "→Village", None, tui_mgr);
        button_4.show(tui_mgr);
        let menu_buttons = [
            (button_1, true),
            (button_2, true),
            (button_3, true),
            (button_4, true),
        ];
        let mut configs: HashMap<u8, MenuConfig> = HashMap::new();
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
            0,
            MenuConfig::new(
                [
                    ButtonState::Show("Fylter".to_string()),
                    ButtonState::Show("Add new".to_string()),
                    // ButtonState::Hide,
                    ButtonState::Show("Options".to_string()),
                    ButtonState::Show("→ Village".to_string()),
                ],
                EntriesState::QueryLogic(QueryType::AllTopics),
            ),
        );
        configs.insert(
            1, // Topic menu
            MenuConfig::new(
                [
                    ButtonState::Show("New post".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Show("← Forum".to_string()),
                ],
                EntriesState::QueryLogic(QueryType::AllPosts),
            ),
        );
        configs.insert(
            2, // Settings menu
            MenuConfig::new(
                [
                    // ButtonState::Show("Active".to_string()),
                    // ButtonState::Show("Stored".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                    ButtonState::Hide,
                    // ButtonState::Show("ByteSets".to_string()),
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
                ]),
            ),
        );

        configs.insert(
            3, // Running Policy
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActivePolicy),
            ),
        );
        configs.insert(
            4, // Running Capabilities
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActiveCapability),
            ),
        );
        configs.insert(
            5, // Running Byte Sets
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::ActiveByteSet),
            ),
        );
        configs.insert(
            6, // Stored Policy
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::StoredPolicy),
            ),
        );
        configs.insert(
            7, // Stored Capabilities
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
                ],
                EntriesState::QueryLogic(QueryType::StoredCapability),
            ),
        );
        configs.insert(
            8, // Stored Byte Sets
            MenuConfig::new(
                [
                    ButtonState::Show("←Setings".to_string()),
                    ButtonState::Show("← Forum".to_string()),
                    ButtonState::Hide,
                    ButtonState::Hide,
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
            current_config_id: 0,
            is_menu_active: true,
            selected_menu_button: 0,
            selected_entry_button: 0,
        }
    }
    pub fn is_current_config_equal(&self, conf_id: u8) -> bool {
        self.current_config_id == conf_id
    }
    pub fn activate(&mut self, tui_mgr: &mut Manager) -> Option<Action> {
        if self.is_menu_active {
            match self.current_config_id {
                0 => {
                    //Main menu
                    match self.selected_menu_button {
                        0 => {
                            //TODO: Filter
                            None
                        }
                        1 => {
                            //TODO: Add new
                            None
                        }
                        2 => {
                            //TODO: Options
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        3 => {
                            // TODO: Village
                            None
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
                1 => {
                    //Topic Menu
                    // TODO: define buttons
                    match self.selected_menu_button {
                        0 => {
                            //TODO: Policy menu
                            // We should request a list of active policies
                            // from logic to present on screen
                            // self.activate_menu(2, tui_mgr);
                            // Some(Action::ProvideActivePolicies)
                            None
                        }
                        1 => {
                            // self.activate_menu(3, tui_mgr);
                            None
                        }
                        2 => {
                            self.activate_menu(4, tui_mgr);
                            None
                        }
                        3 => {
                            // Back to main menu
                            self.activate_menu(0, tui_mgr);
                            None
                        }
                        _o => {
                            // this should not happen
                            None
                        }
                    }
                }
                2 => {
                    //Settings Menu
                    // TODO: maybe some more buttons?
                    match self.selected_menu_button {
                        0 => {
                            // Back to menu
                            self.activate_menu(0, tui_mgr);
                            None
                        }
                        1 => {
                            //hidden
                            None
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
                3 => {
                    //Running Policies Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                4 => {
                    //Running Capabilities Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                5 => {
                    //Running Byte Sets Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                6 => {
                    //Stored Policies Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                7 => {
                    //Stored Capabilities Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                8 => {
                    //Stored Byte Sets Menu
                    match self.selected_menu_button {
                        0 => {
                            // Back to  Settings menu
                            self.activate_menu(2, tui_mgr);
                            None
                        }
                        1 => {
                            // Back to Main menu
                            self.activate_menu(0, tui_mgr);
                            None
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
                other => {
                    eprintln!("Unexpected button config id: {other}");
                    None
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
                EntryAction::Query => Some(Action::Query(
                    self.entry_buttons
                        .get(self.selected_entry_button)
                        .unwrap()
                        .2,
                )),
                EntryAction::RunningPolicies => {
                    self.activate_menu(3, tui_mgr);
                    Some(Action::RunningPolicies)
                }
                EntryAction::RunningCapabilities => {
                    self.activate_menu(4, tui_mgr);
                    Some(Action::RunningCapabilities)
                }
                EntryAction::RunningByteSets => {
                    self.activate_menu(5, tui_mgr);
                    Some(Action::RunningByteSets)
                }
                EntryAction::StoredPolicies => {
                    self.activate_menu(6, tui_mgr);
                    Some(Action::StoredPolicies)
                }
                EntryAction::StoredCapabilities => {
                    self.activate_menu(7, tui_mgr);
                    Some(Action::StoredCapabilities)
                }
                EntryAction::StoredByteSets => {
                    self.activate_menu(8, tui_mgr);
                    Some(Action::StoredByteSets)
                }
                EntryAction::NoAction => None,
            }
        }
    }
    pub fn update_entries(&mut self, mut new_list: Vec<(u16, String)>, tui_mgr: &mut Manager) {
        // TODO:
        for butn in &mut self.entry_buttons {
            if !new_list.is_empty() {
                let (id, text) = new_list.remove(0);
                butn.0.hide(tui_mgr);
                butn.0.rename(tui_mgr, &text);
                butn.0.show(tui_mgr);
                butn.1 = true;
                butn.2 = id;
            } else {
                break;
            }
        }
        if !new_list.is_empty() {
            eprintln!("Received overflowing entrines page!");
        }
    }

    fn up(&mut self, tui_mgr: &mut Manager) {
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

            self.select_prev_entry();
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        }
    }
    fn down(&mut self, tui_mgr: &mut Manager) {
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

            self.select_next_entry();
            self.entry_buttons[self.selected_entry_button]
                .0
                .select(tui_mgr, false);
        }
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
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, false);
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
            self.entry_buttons[self.selected_entry_button]
                .0
                .deselect(tui_mgr, false);
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
    fn select_next_entry(&mut self) {
        for i in 1..=self.entry_buttons.len() {
            let next_id = (self.selected_entry_button + i) % self.entry_buttons.len();
            if self.entry_buttons[next_id].1 {
                self.selected_entry_button = next_id;
                break;
            }
        }
    }
    fn select_prev_entry(&mut self) {
        for i in 1..=self.entry_buttons.len() {
            let next_id = (self.selected_entry_button + self.entry_buttons.len() - i)
                % self.entry_buttons.len();
            if self.entry_buttons[next_id].1 {
                self.selected_entry_button = next_id;
                break;
            }
        }
    }
    fn activate_menu(&mut self, cfg_idx: u8, tui_mgr: &mut Manager) {
        self.is_menu_active = true;
        if let Some(ms) = self.configs.get(&cfg_idx) {
            self.current_config_id = cfg_idx;
            self.selected_menu_button = 0;
            self.selected_entry_button = 0;

            for i in 0..4 {
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
                EntriesState::QueryLogic(qt) => {
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

struct MenuConfig {
    menu: [ButtonState; 4],
    entries: EntriesState,
    // button_to_id
}

impl MenuConfig {
    pub fn new(menu: [ButtonState; 4], entries: EntriesState) -> Self {
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
    my_id: GnomeId,
    toolset: Toolset,
    // mut tui_mgr: Manager,
    to_app: Sender<FromForumView>,
    // to_tui_send: Sender<ToPresentation>,
    to_tui_recv: Receiver<ToForumView>,
    // config: Configuration,
    // ) -> (Manager, Configuration) {
) -> Toolset {
    let (mut tui_mgr, config, e_opt, c_opt, s_opt, i_opt, pe_opt) = toolset.unfold();
    let mut creator = c_opt.unwrap();
    let mut selector = s_opt.unwrap();

    // TODO: PEditor should be created once upon
    // startup & should be passed to an app
    // together with other tools
    let mut pedit = if let Some(pe) = pe_opt {
        pe
    } else {
        PolicyEditor::new(&mut tui_mgr)
    };
    // TODO: do not create a new display every time Forum App is opened
    let main_display = tui_mgr.new_display(true);
    let (cols, rows) = tui_mgr.screen_size();
    let mut frame = vec![Glyph::blue(); cols * rows];
    // Forum
    let mut buttons_logic = ButtonsLogic::new(&mut tui_mgr);
    let mut action = None;
    action = buttons_logic.activate(&mut tui_mgr);
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
                    let _action = buttons_logic.up(&mut tui_mgr);
                }
                Key::Down | Key::Comma => {
                    let _action = buttons_logic.down(&mut tui_mgr);
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
                ToForumView::RunningPoliciesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(3) {
                        buttons_logic.update_entries(plcs, &mut tui_mgr);
                    }
                }
                ToForumView::StoredPoliciesPage(_pg_no, plcs) => {
                    if buttons_logic.is_current_config_equal(6) {
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
                    }
                    tui_mgr.restore_display(main_display, true);
                }
                ToForumView::SelectOne(list) => {
                    let selected = selector.select("Pick one", &list, vec![], &mut tui_mgr, true);
                    // TODO: bring up selector with
                    // a given list & reply back to logic.
                    eprintln!("Selected: {:?}", selected);
                    tui_mgr.restore_display(main_display, true);
                    if !selected.is_empty() {
                        action = Some(Action::OneSelected(selected[0]));
                    } else {
                        //TODO:
                        eprintln!("Failed to select one!");
                    }
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
