use animaterm::Manager;

pub struct PolicyEditor {
    display_id: usize,
    // TODO: build a policy editor and open it
}
impl PolicyEditor {
    pub fn new(mgr: &mut Manager) -> Self {
        PolicyEditor { display_id: 19 }
    }
    pub fn cleanup(&self, main_display: usize, mgr: &mut Manager) {
        eprintln!("Editor cleanup");
        mgr.restore_display(self.display_id, true);
        mgr.restore_display(main_display, false);
    }
}
