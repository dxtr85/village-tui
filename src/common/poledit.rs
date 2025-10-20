use std::collections::HashMap;

use animaterm::{Animation, Glyph, Graphic, Manager, Timestamp};
use dapp_lib::prelude::{Capabilities, Policy, Requirement};

#[derive(Debug)]
pub enum PolAction {
    SelectPolicy,
    SelectRequirement(ReqTree), //bool indicates if we should
    // include logical reqs (And, Or)
    Store(Policy, ReqTree),
    Run(Policy, ReqTree),
}

pub struct PolicyEditor {
    // policy: Policy,
    // requirement: Requirement,
    display_id: usize,
    blinker_id: usize,
    selection: (usize, usize),
    size: (usize, usize),
    pyramid: Pyramid,
    // TODO: build a policy editor and open it
    // Editor should consist of a working space where you can build
    // Requirement for a Policy using graphical structures similar
    // to Lego bricks.
}
impl PolicyEditor {
    pub fn new(mgr: &mut Manager) -> Self {
        // create own display
        let display_id = mgr.new_display(true);
        let (s_w, s_h) = mgr.screen_size();
        let dim = s_w * s_h;
        let pyramid = Pyramid::new(mgr);
        let frame = vec![Glyph::transparent(); dim];
        let mut library = HashMap::with_capacity(2);
        library.insert(0, frame.clone());
        let mut frame = vec![Glyph::transparent(); dim];
        draw_a_box(s_w, &pyramid.back, 0, Glyph::white(), &mut frame);
        // mgr.swap_frame(self.blinker_id, 1, frame);
        library.insert(1, frame);
        let anim = Animation::new(
            false,
            true,
            vec![(1, Timestamp::new(0, 750)), (0, Timestamp::new(0, 750))],
            Timestamp::now(),
        );
        let mut animations = HashMap::new();
        animations.insert(0, anim);
        let blinker = Graphic::new(s_w, s_h, 0, library, Some(animations));
        let blinker_id = mgr.add_graphic(blinker, 2, (0, 0)).unwrap();

        PolicyEditor {
            display_id,
            blinker_id,
            size: (s_w, s_h),
            selection: (0, 0),

            // policy: Policy::Default,
            // requirement: Requirement::Has(Capabilities::Founder),
            pyramid,
        }
    }
    pub fn present(
        &mut self,
        policy: Policy,
        mut req: ReqTree,
        mgr: &mut Manager,
    ) -> Option<PolAction> {
        mgr.restore_display(self.display_id, true);
        // let mut req = decompose(req);
        self.pyramid.paint(mgr, policy, &req);

        // TODO: allow for selecting and changing blocks.
        // maybe we should have a separate Graphic with
        // two frames covering entire screen.
        // frame 0 would be all transparent Glyphs.
        // frame 1 would also be transparent, but
        // excluding area assigned to selected Block.
        // This Graphic would play an animation switching
        // between those frames in an infinite loop.
        // When we navigate to select different block,
        // we swap frame 1 to a new one that is prepared to
        // blink over new button only.
        mgr.start_animation(self.blinker_id, 0);
        loop {
            if let Some(key) = mgr.read_key() {
                match key {
                    animaterm::Key::Up | animaterm::Key::O => {
                        self.blinker_up(mgr);
                    }
                    animaterm::Key::Down | animaterm::Key::Comma => {
                        self.blinker_down(mgr, &req);
                    }
                    animaterm::Key::Left | animaterm::Key::P => {
                        self.blinker_left(mgr, &req);
                    }
                    animaterm::Key::Right | animaterm::Key::W => {
                        self.blinker_right(mgr, &req);
                    }
                    animaterm::Key::Enter => {
                        //TODO: Add blocks for Cancel & Apply
                        eprintln!("Enter on: {:?}", self.selection);
                        mgr.set_graphic(self.blinker_id, 0, false);
                        match self.selection.0 {
                            0 => match self.selection.1 {
                                1 => return Some(PolAction::SelectPolicy),
                                2 => return Some(PolAction::Run(policy, req)),
                                3 => return Some(PolAction::Store(policy, req)),
                                _o => return None,
                            },

                            // TODO: assert that we need
                            // a req. for current block
                            _o => {
                                req.mark(self.selection);
                                eprintln!("Mark loc: {:?}", req.mark_location());
                                return Some(PolAction::SelectRequirement(req));
                            }
                        }
                    }
                    _other => {
                        break;
                    }
                }
            };
        }
        None
    }
    pub fn cleanup(&self, main_display: usize, mgr: &mut Manager) {
        eprintln!("Editor cleanup");
        mgr.restore_display(self.display_id, true);
        mgr.restore_display(main_display, false);
    }
    fn blinker_up(&mut self, mgr: &mut Manager) {
        if self.selection.0 == 0 {
            return;
        }
        let mut frame = vec![Glyph::transparent(); self.size.0 * self.size.1];
        let mut block = &Block {
            start_x: 0,
            size_x: 0,
            start_y: 0,
            size_y: 0,
        };
        match self.selection.0 {
            1 => {
                block = &self.pyramid.eye;
                self.selection = (0, 1);
            }
            2 => {
                block = &self.pyramid.top;
                self.selection = (1, 0);
            }
            3 => match self.selection.1 {
                0 | 1 => {
                    block = &self.pyramid.second[0];
                    self.selection = (2, 0);
                }
                _o => {
                    block = &self.pyramid.second[1];
                    self.selection = (2, 1);
                }
            },
            4 => match self.selection.1 {
                0 | 1 => {
                    block = &self.pyramid.third[0];
                    self.selection = (3, 0);
                }
                2 | 3 => {
                    block = &self.pyramid.third[1];
                    self.selection = (3, 1);
                }
                4 | 5 => {
                    block = &self.pyramid.third[2];
                    self.selection = (3, 2);
                }
                _o => {
                    block = &self.pyramid.third[3];
                    self.selection = (3, 3);
                }
            },
            _o => {
                eprintln!("Unexpected selection moving blinker UP: {_o}");
            }
        }
        draw_a_box(self.size.0, &block, 0, Glyph::white(), &mut frame);
        mgr.stop_animation(self.blinker_id);
        mgr.set_graphic(self.blinker_id, 0, false);
        mgr.swap_frame(self.blinker_id, 1, frame);
        mgr.start_animation(self.blinker_id, 0);
    }
    fn blinker_down(&mut self, mgr: &mut Manager, req: &ReqTree) {
        if self.selection.0 == 4 {
            return;
        }
        let mut frame = vec![Glyph::transparent(); self.size.0 * self.size.1];
        let block;
        //  = &Block {
        //     start_x: 0,
        //     size_x: 0,
        //     start_y: 0,
        //     size_y: 0,
        // };
        match self.selection.0 {
            0 => {
                block = &self.pyramid.top;
                self.selection = (1, 0);
            }
            1 => {
                // TODO: only allow movement to target
                // location when it makes sense.
                // Here it should be allowed only if
                // top block represents a logical fn.
                if req.req().is_logic_fn() {
                    block = &self.pyramid.second[0];
                    self.selection = (2, 0);
                } else {
                    return;
                }
            }
            2 => match self.selection.1 {
                0 => {
                    if req.left().req().is_logic_fn() {
                        block = &self.pyramid.third[0];
                        self.selection = (3, 0);
                    } else {
                        return;
                    }
                }
                _other => {
                    if req.right().req().is_logic_fn() {
                        block = &self.pyramid.third[2];
                        self.selection = (3, 2);
                    } else {
                        return;
                    }
                }
            },
            3 => match self.selection.1 {
                0 => {
                    if req.left().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[0];
                        self.selection = (4, 0);
                    } else {
                        return;
                    }
                }
                1 => {
                    if req.left().right().req().is_logic_fn() {
                        block = &self.pyramid.fourth[2];
                        self.selection = (4, 2);
                    } else {
                        return;
                    }
                }
                2 => {
                    if req.right().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[4];
                        self.selection = (4, 4);
                    } else {
                        return;
                    }
                }
                _other => {
                    if req.right().right().req().is_logic_fn() {
                        block = &self.pyramid.fourth[6];
                        self.selection = (4, 6);
                    } else {
                        return;
                    }
                }
            },
            _o => {
                eprintln!("Unexp. sel moving blinker down: {_o}");
                return;
            }
        }

        draw_a_box(self.size.0, &block, 0, Glyph::white(), &mut frame);
        mgr.stop_animation(self.blinker_id);
        mgr.set_graphic(self.blinker_id, 0, false);
        mgr.swap_frame(self.blinker_id, 1, frame);
        mgr.start_animation(self.blinker_id, 0);
        // mgr.restart_animation(self.blinker_id, 0, Timestamp::now());
    }
    fn blinker_left(&mut self, mgr: &mut Manager, req: &ReqTree) {
        // TODO: Back: (0,0)
        //       Eye:  (0,1)
        //       Run:  (0,2)
        //       Store:(0,3)
        if self.selection.1 == 0 {
            return;
        }
        let mut frame = vec![Glyph::transparent(); self.size.0 * self.size.1];
        let block;
        //  = &Block {
        //     start_x: 0,
        //     size_x: 0,
        //     start_y: 0,
        //     size_y: 0,
        // };
        match self.selection.0 {
            0 => {
                match self.selection.1 {
                    1 => {
                        //TODO
                        block = &self.pyramid.back;
                        self.selection = (0, 0);
                    }
                    2 => {
                        //TODO
                        block = &self.pyramid.eye;
                        self.selection = (0, 1);
                    }
                    3 => {
                        //TODO
                        block = &self.pyramid.set_running;
                        self.selection = (0, 2);
                    }
                    _o => {
                        //TODO
                        return;
                    }
                }
            }
            1 => {
                // TODO: we could move down if second layer
                // is visible…
                return;
            }
            2 => {
                block = &self.pyramid.second[0];
                self.selection = (2, 0);
            }
            3 => {
                // TODO: conditionally jump left
                match self.selection.1 {
                    2 => {
                        if req.left().req().is_logic_fn() {
                            block = &self.pyramid.third[self.selection.1 - 1];
                            self.selection = (3, self.selection.1 - 1);
                        } else {
                            return;
                        }
                    }
                    _o => {
                        block = &self.pyramid.third[self.selection.1 - 1];
                        self.selection = (3, self.selection.1 - 1);
                    }
                }
            }
            4 => match self.selection.1 {
                2 => {
                    if req.left().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[self.selection.1 - 1];
                        self.selection = (4, self.selection.1 - 1);
                    } else {
                        return;
                    }
                }
                4 => {
                    if req.left().right().req().is_logic_fn() {
                        block = &self.pyramid.fourth[self.selection.1 - 1];
                        self.selection = (4, self.selection.1 - 1);
                    } else if req.left().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[1];
                        self.selection = (4, 1);
                    } else {
                        return;
                    }
                }
                6 => {
                    if req.right().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[self.selection.1 - 1];
                        self.selection = (4, self.selection.1 - 1);
                    } else if req.left().right().req().is_logic_fn() {
                        block = &self.pyramid.fourth[3];
                        self.selection = (4, 3);
                    } else if req.left().left().req().is_logic_fn() {
                        block = &self.pyramid.fourth[1];
                        self.selection = (4, 1);
                    } else {
                        return;
                    }
                }
                _o => {
                    block = &self.pyramid.fourth[self.selection.1 - 1];
                    self.selection = (4, self.selection.1 - 1);
                }
            },
            _o => {
                eprintln!("Unexp. sel moving blinker left: {_o}");
                return;
            }
        }
        draw_a_box(self.size.0, &block, 0, Glyph::white(), &mut frame);
        mgr.stop_animation(self.blinker_id);
        mgr.set_graphic(self.blinker_id, 0, false);
        mgr.swap_frame(self.blinker_id, 1, frame);
        mgr.start_animation(self.blinker_id, 0);
    }
    fn blinker_right(&mut self, mgr: &mut Manager, req: &ReqTree) {
        if self.selection == (0, 3)
            || self.selection == (1, 0)
            || self.selection == (2, 1)
            || self.selection == (3, 3)
            || self.selection == (4, 7)
        {
            return;
        }
        let mut frame = vec![Glyph::transparent(); self.size.0 * self.size.1];
        let mut block = &Block {
            start_x: 0,
            size_x: 0,
            start_y: 0,
            size_y: 0,
        };
        match self.selection.0 {
            0 => {
                match self.selection.1 {
                    0 => {
                        //TODO
                        block = &self.pyramid.eye;
                        self.selection = (0, 1);
                    }
                    1 => {
                        //TODO
                        block = &self.pyramid.set_running;
                        self.selection = (0, 2);
                    }
                    2 => {
                        //TODO
                        block = &self.pyramid.store;
                        self.selection = (0, 3);
                    }

                    _o => {
                        //TODO
                        return;
                    }
                }
            }
            1 => {
                return;
            }
            2 => {
                block = &self.pyramid.second[1];
                self.selection = (2, 1);
            }
            3 => match self.selection.1 {
                1 => {
                    if req.right().req().is_logic_fn() {
                        block = &self.pyramid.third[self.selection.1 + 1];
                        self.selection = (3, self.selection.1 + 1);
                    } else {
                        return;
                    }
                }
                _o => {
                    block = &self.pyramid.third[self.selection.1 + 1];
                    self.selection = (3, self.selection.1 + 1);
                }
            },
            4 => {
                match self.selection.1 {
                    5 => {
                        if req.right().right().req().is_logic_fn() {
                            block = &self.pyramid.fourth[self.selection.1 + 1];
                            self.selection = (4, self.selection.1 + 1);
                        } else {
                            return;
                        }
                    }
                    3 => {
                        if req.right().left().req().is_logic_fn() {
                            block = &self.pyramid.fourth[self.selection.1 + 1];
                            self.selection = (4, self.selection.1 + 1);
                        } else if req.right().right().req().is_logic_fn() {
                            block = &self.pyramid.fourth[6];
                            self.selection = (4, 6);
                        } else {
                            return;
                        }
                    }
                    1 => {
                        if req.left().right().req().is_logic_fn() {
                            block = &self.pyramid.fourth[self.selection.1 + 1];
                            self.selection = (4, self.selection.1 + 1);
                        } else if req.right().left().req().is_logic_fn() {
                            block = &self.pyramid.fourth[4];
                            self.selection = (4, 4);
                        } else if req.right().right().req().is_logic_fn() {
                            block = &self.pyramid.fourth[6];
                            self.selection = (4, 6);
                        } else {
                            return;
                        }
                    }
                    _o => {
                        block = &self.pyramid.fourth[self.selection.1 + 1];
                        self.selection = (4, self.selection.1 + 1);
                    }
                }

                // block = &self.pyramid.fourth[self.selection.1 + 1];
                // self.selection = (4, self.selection.1 + 1);
            }
            _o => {
                eprintln!("Unexp. sel moving blinker left: {_o}");
            }
        }
        draw_a_box(self.size.0, &block, 0, Glyph::white(), &mut frame);
        mgr.stop_animation(self.blinker_id);
        mgr.set_graphic(self.blinker_id, 0, false);
        mgr.swap_frame(self.blinker_id, 1, frame);
        mgr.start_animation(self.blinker_id, 0);
    }
}
#[derive(Clone, Debug)]
pub struct ReqTree {
    r: Req,
    left: Option<Box<ReqTree>>,
    right: Option<Box<ReqTree>>,
}
impl ReqTree {
    pub fn req(&self) -> Req {
        self.r
    }
    pub fn requirement(self) -> Result<Requirement, ()> {
        if self.r.is_logic_fn() {
            let left_r = self.left().requirement();
            let right_r = self.right().requirement();
            if left_r.is_err() {
                return Err(());
            }
            if right_r.is_err() {
                return Err(());
            }
            self.r.compose(left_r.ok(), right_r.ok())
        } else {
            self.r.compose(None, None)
        }
    }

    pub fn none() -> ReqTree {
        ReqTree {
            r: Req::None,
            left: None,
            right: None,
        }
    }
    pub fn left(&self) -> ReqTree {
        if let Some(l) = &self.left {
            *l.clone()
        } else {
            ReqTree::none()
        }
    }
    pub fn right(&self) -> ReqTree {
        if let Some(r) = &self.right {
            *r.clone()
        } else {
            ReqTree::none()
        }
    }
    pub fn replace_mark(&mut self, r: Requirement) -> bool {
        eprintln!("Replacing mark @{:?}", self);
        if self.r.is_marker() {
            let rt = decompose(r);
            self.r = rt.r;
            if !self.r.is_logic_fn() {
                self.left = None;
                self.right = None;
            }
            return true;
        } else {
            let mut left_res = false;
            let mut right_res = false;
            if let Some(mut left) = self.left.take() {
                left_res = left.replace_mark(r.clone());
                self.left = Some(left);
            }
            if !left_res {
                if let Some(mut right) = self.right.take() {
                    right_res = right.replace_mark(r);
                    self.right = Some(right);
                }
            }
            left_res || right_res
        }
    }
    pub fn mark_location(&mut self) -> (usize, usize) {
        if self.req().is_marker() {
            return (1, 0);
        } else if self.left().req().is_marker() {
            return (2, 0);
        } else if self.right().req().is_marker() {
            return (2, 1);
        } else if self.left().left().req().is_marker() {
            return (3, 0);
        } else if self.left().right().req().is_marker() {
            return (3, 1);
        } else if self.right().left().req().is_marker() {
            return (3, 2);
        } else if self.right().right().req().is_marker() {
            return (3, 3);
        } else if self.left().left().left().req().is_marker() {
            return (4, 0);
        } else if self.left().left().left().req().is_marker() {
            return (4, 1);
        } else if self.left().left().right().req().is_marker() {
            return (4, 2);
        } else if self.left().right().left().req().is_marker() {
            return (4, 3);
        } else if self.right().left().left().req().is_marker() {
            return (4, 4);
        } else if self.right().left().left().req().is_marker() {
            return (4, 5);
        } else if self.right().left().right().req().is_marker() {
            return (4, 6);
        } else if self.right().right().left().req().is_marker() {
            return (4, 7);
        } else {
            return (0, 0);
        }
    }
    pub fn mark(&mut self, pos: (usize, usize)) {
        match pos.0 {
            1 => self.r = Req::Marker,
            2 => match pos.1 {
                0 => {
                    let mut rl = self.left();
                    rl.mark((1, 0));
                    self.left = Some(Box::new(rl));
                }
                _o => {
                    eprintln!("Placing marker (2,{_o})");
                    let mut rr = self.right();
                    rr.mark((1, 0));
                    self.right = Some(Box::new(rr));
                }
            },
            3 => match pos.1 {
                0 | 1 => {
                    let mut rl = self.left();
                    rl.mark((2, pos.1));
                    self.left = Some(Box::new(rl));
                }

                _o => {
                    let mut rr = self.right();
                    rr.mark((2, pos.1 - 2));
                    self.right = Some(Box::new(rr));
                }
            },
            4 => match pos.1 {
                0 | 1 | 2 | 3 => {
                    let mut rl = self.left();
                    rl.mark((3, pos.1));
                    self.left = Some(Box::new(rl));
                }

                _o => {
                    let mut rr = self.right();
                    rr.mark((3, pos.1 - 4));
                    self.right = Some(Box::new(rr));
                }
            },
            o => {
                eprintln!("Can not mark: {o}");
            }
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Req {
    And,
    Or,
    Has(Capabilities),
    DataByte2InSet(u8),
    DataByte2Is(u8),
    DataByte2IsNot(u8),
    DataByte3InSet(u8),
    DataByte3Is(u8),
    DataByte3IsNot(u8),
    DataBytes2And3InSet(u8),
    None,
    Marker,
}
impl Req {
    pub fn compose(
        self,
        left: Option<Requirement>,
        right: Option<Requirement>,
    ) -> Result<Requirement, ()> {
        match self {
            Self::And => {
                if let Some(l) = left {
                    if let Some(r) = right {
                        Ok(Requirement::And(Box::new(l), Box::new(r)))
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
            Self::Or => {
                if let Some(l) = left {
                    if let Some(r) = right {
                        Ok(Requirement::Or(Box::new(l), Box::new(r)))
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            }
            Self::Has(c) => Ok(Requirement::Has(c)),
            Self::DataByte2InSet(set) => Ok(Requirement::DataByte2InSet(set)),
            Self::DataByte2Is(byte) => Ok(Requirement::DataByte2Is(byte)),
            Self::DataByte2IsNot(byte) => Ok(Requirement::DataByte2IsNot(byte)),
            Self::DataByte3InSet(set) => Ok(Requirement::DataByte3InSet(set)),
            Self::DataByte3Is(byte) => Ok(Requirement::DataByte3Is(byte)),
            Self::DataByte3IsNot(byte) => Ok(Requirement::DataByte3IsNot(byte)),
            Self::DataBytes2And3InSet(set) => Ok(Requirement::DataBytes2And3InSet(set)),
            Self::None => Ok(Requirement::None),
            Self::Marker => Err(()),
        }
    }
    pub fn not_none(&self) -> bool {
        !matches!(self, Req::None)
    }
    pub fn is_none(&self) -> bool {
        matches!(self, Req::None)
    }
    pub fn is_marker(&self) -> bool {
        matches!(self, Req::Marker)
    }
    pub fn is_logic_fn(&self) -> bool {
        matches!(self, Req::And) || matches!(self, Req::Or)
    }
    pub fn text(&self) -> String {
        match self {
            Req::And => "And".to_string(),
            Req::Or => "Or".to_string(),
            Req::Has(c) => format!("{:?}", c),
            Req::DataByte2InSet(b) => format!("DBt2InSet({})", b),
            Req::DataByte2Is(b) => format!("DBt2={}", b),
            Req::DataByte2IsNot(b) => format!("DBt2!={}", b),
            Req::DataByte3InSet(b) => format!("DBt3InSet({})", b),
            Req::DataByte3Is(b) => format!("DBt3={}", b),
            Req::DataByte3IsNot(b) => format!("DBt3!={}", b),
            Req::DataBytes2And3InSet(b) => format!("DBt2&3InSet({})", b),
            Req::None => "None".to_string(),
            Req::Marker => "Marker".to_string(),
        }
    }
}
pub fn decompose(req: Requirement) -> ReqTree {
    match req {
        Requirement::And(left, right) => {
            let left = decompose(*left);
            let right = decompose(*right);
            ReqTree {
                r: Req::And,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            }
        }
        Requirement::Or(left, right) => {
            let left = decompose(*left);
            let right = decompose(*right);
            ReqTree {
                r: Req::Or,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            }
        }
        Requirement::Has(c) => ReqTree {
            r: Req::Has(c),
            left: None,
            right: None,
        },
        Requirement::DataByte2InSet(s_id) => ReqTree {
            r: Req::DataByte2InSet(s_id),
            left: None,
            right: None,
        },
        Requirement::DataByte2Is(byte) => ReqTree {
            r: Req::DataByte2Is(byte),
            left: None,
            right: None,
        },
        Requirement::DataByte2IsNot(byte) => ReqTree {
            r: Req::DataByte2IsNot(byte),
            left: None,
            right: None,
        },
        Requirement::DataByte3InSet(s_id) => ReqTree {
            r: Req::DataByte3InSet(s_id),
            left: None,
            right: None,
        },
        Requirement::DataByte3Is(byte) => ReqTree {
            r: Req::DataByte3Is(byte),
            left: None,
            right: None,
        },
        Requirement::DataByte3IsNot(byte) => ReqTree {
            r: Req::DataByte3IsNot(byte),
            left: None,
            right: None,
        },
        Requirement::DataBytes2And3InSet(s_id) => ReqTree {
            r: Req::DataBytes2And3InSet(s_id),
            left: None,
            right: None,
        },
        Requirement::None => ReqTree {
            r: Req::None,
            left: None,
            right: None,
        },
    }
}
struct Block {
    start_x: usize,
    size_x: usize,
    start_y: usize,
    size_y: usize,
}
struct Pyramid {
    g_id: usize,
    size: (usize, usize),
    eye: Block,
    back: Block,
    set_running: Block,
    store: Block,
    top: Block,
    second: [Block; 2],
    third: [Block; 4],
    fourth: [Block; 8],
}
impl Pyramid {
    pub fn new(mgr: &mut Manager) -> Self {
        let (s_width, s_height) = mgr.screen_size();
        let s_height = usize::max(25, s_height);
        // TODO: make sure s_height is right

        let f = vec![Glyph::black(); s_width * s_height];
        let mut lib = HashMap::new();
        lib.insert(0, f);
        let canvas = Graphic::new(s_width, s_height, 0, lib, None);

        let g_id = mgr.add_graphic(canvas, 4, (0, 0)).unwrap();
        mgr.move_graphic(g_id, 4, (0, 0));

        let eye = Block {
            start_x: (s_width >> 1) - 12,
            start_y: 1,
            size_x: 24,
            size_y: 3,
        };
        // TODO: add back, set_running, store
        let back = Block {
            start_x: (s_width >> 1) - 22,
            start_y: 1,
            size_x: 8,
            size_y: 3,
        };
        let set_running = Block {
            start_x: (s_width >> 1) + 14,
            start_y: 1,
            size_x: 8,
            size_y: 3,
        };
        let store = Block {
            start_x: (s_width >> 1) + 24,
            start_y: 1,
            size_x: 8,
            size_y: 3,
        };
        let top = Block {
            start_x: (s_width >> 1) - 12,
            start_y: 6,
            size_x: 24,
            size_y: 3,
        };
        let second = [
            Block {
                start_x: (s_width >> 1) - 27,
                start_y: 9,
                size_x: 16,
                size_y: 3,
            },
            Block {
                start_x: (s_width >> 1) + 11,
                start_y: 9,
                size_x: 16,
                size_y: 3,
            },
        ];
        let third = [
            Block {
                start_x: (s_width >> 1) - 36,
                start_y: 12,
                size_x: 12,
                size_y: 3,
            },
            Block {
                start_x: (s_width >> 1) - 16,
                start_y: 12,
                size_x: 12,
                size_y: 3,
            },
            Block {
                start_x: (s_width >> 1) + 4,
                start_y: 12,
                size_x: 12,
                size_y: 3,
            },
            Block {
                start_x: (s_width >> 1) + 24,
                start_y: 12,
                size_x: 12,
                size_y: 3,
            },
        ];
        let fourth = [
            Block {
                start_x: (s_width >> 1) - 36,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) - 27,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) - 16,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) - 7,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) + 4,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) + 13,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) + 24,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
            Block {
                start_x: (s_width >> 1) + 33,
                start_y: 15,
                size_x: 3,
                size_y: 5,
            },
        ];
        Pyramid {
            g_id,
            size: (s_width, s_height),
            eye,
            top,
            back,
            set_running,
            store,
            second,
            third,
            fourth,
        }
    }

    fn paint_top_eye(&self, text: String, color: Glyph, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.eye, 0, color, f);
        type_text(self.size.0, text, &self.eye, color, f);
    }
    fn paint_buttons(&self, color: Glyph, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.back, 0, color, f);
        type_text(self.size.0, format!("Back"), &self.back, color, f);

        draw_a_box(self.size.0, &self.set_running, 0, color, f);
        type_text(self.size.0, format!("Run"), &self.set_running, color, f);

        draw_a_box(self.size.0, &self.store, 0, color, f);
        type_text(self.size.0, format!("Store"), &self.store, color, f);
    }

    fn paint_top_block(&self, text: String, color: Glyph, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.top, 0, color, f);
        type_text(self.size.0, text, &self.top, color, f);
    }

    fn paint_second_layer(&self, texts: (String, String), color: Glyph, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.second[0], 0, color, f);
        type_text(self.size.0, texts.0, &self.second[0], color, f);
        draw_a_box(self.size.0, &self.second[1], 0, color, f);
        type_text(self.size.0, texts.1, &self.second[1], color, f);
    }

    fn paint_third_layer(&self, text: String, idx: usize, color: Glyph, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.third[idx], 0, color, f);
        type_text(self.size.0, text, &self.third[idx], color, f);
    }

    fn paint_fourth_layer(
        &self,
        idx: usize,
        texts: (String, String),
        color: Glyph,
        f: &mut Vec<Glyph>,
    ) {
        draw_a_box(self.size.0, &self.fourth[idx], 0, color, f);
        type_text(self.size.0, texts.0, &self.fourth[idx], color, f);
        draw_a_box(self.size.0, &self.fourth[idx + 1], 0, color, f);
        type_text(self.size.0, texts.1, &self.fourth[idx + 1], color, f);
    }

    fn paint(&self, mgr: &mut Manager, pol: Policy, req_tree: &ReqTree) {
        let f_len = self.size.0 * self.size.1;
        // let g_t = Glyph::transparent();
        let g_t = Glyph::black();
        let mut f = vec![g_t; f_len];

        self.paint_buttons(Glyph::indigo(), &mut f);
        self.paint_top_eye(pol.text(), Glyph::magenta(), &mut f);
        if req_tree.req().is_none() {
            return;
        }
        self.paint_top_block(req_tree.req().text(), Glyph::green(), &mut f);
        // If we want to use only 1 layer and frame
        // first we have to paint wrappings, starting
        // from outer inwards,
        // and then paint blocks.
        let mut teeth_painted = [false; 4];
        let mut left_side_painted = false;
        let mut right_side_painted = false;
        if req_tree.left().left().left().req().not_none() {
            self.wrap_teeth(0, &mut f);
            teeth_painted[0] = true;
            left_side_painted = true;
        }
        if req_tree.left().right().left().req().not_none() {
            self.wrap_teeth(1, &mut f);
            teeth_painted[1] = true;
            left_side_painted = true;
        }
        if req_tree.right().left().left().req().not_none() {
            self.wrap_teeth(2, &mut f);
            teeth_painted[2] = true;
            right_side_painted = true;
        }
        if req_tree.right().right().left().req().not_none() {
            self.wrap_teeth(3, &mut f);
            teeth_painted[3] = true;
            right_side_painted = true;
        }
        if req_tree.left().left().req().not_none() && !teeth_painted[0] {
            self.wrap_l3(0, Glyph::green(), 2, &mut f);
            if !teeth_painted[1] {
                self.wrap_l2(0, Glyph::green(), 1, &mut f);
            }
            self.wrap_l3(0, Glyph::blue(), 1, &mut f);
            //add
            left_side_painted = true;
        }
        if req_tree.left().right().req().not_none() && !teeth_painted[1] {
            self.wrap_l3(1, Glyph::green(), 2, &mut f);
            if !teeth_painted[0] {
                self.wrap_l2(1, Glyph::green(), 1, &mut f);
            }
            self.wrap_l3(1, Glyph::blue(), 1, &mut f);
            //add
            left_side_painted = true;
        }
        if req_tree.right().left().req().not_none() && !teeth_painted[2] {
            self.wrap_l3(2, Glyph::green(), 2, &mut f);
            if !teeth_painted[3] && !right_side_painted {
                self.wrap_l2(1, Glyph::green(), 1, &mut f);
            }
            self.wrap_l3(2, Glyph::blue(), 1, &mut f);
            //add
            right_side_painted = true;
        }
        if req_tree.right().right().req().not_none() && !teeth_painted[3] {
            self.wrap_l3(3, Glyph::green(), 2, &mut f);
            if !teeth_painted[2] && !right_side_painted {
                self.wrap_l2(1, Glyph::green(), 1, &mut f);
            }
            self.wrap_l3(3, Glyph::blue(), 1, &mut f);
            //add
            right_side_painted = true;
        }
        if req_tree.left().req().not_none() && !left_side_painted {
            self.wrap_l2(0, Glyph::green(), 1, &mut f);
        }
        if req_tree.right().req().not_none() && !right_side_painted {
            self.wrap_l2(1, Glyph::green(), 1, &mut f);
        }

        if req_tree.req().is_logic_fn() {
            self.paint_second_layer(
                (req_tree.left().req().text(), req_tree.right().req().text()),
                Glyph::blue(),
                &mut f,
            );
        }
        if req_tree.left().req().is_logic_fn() {
            self.paint_third_layer(
                req_tree.left().left().req().text(),
                0,
                Glyph::orange(),
                &mut f,
            );
            self.paint_third_layer(
                req_tree.left().right().req().text(),
                1,
                Glyph::orange(),
                &mut f,
            );
        }
        if req_tree.right().req().is_logic_fn() {
            self.paint_third_layer(
                req_tree.right().left().req().text(),
                2,
                Glyph::orange(),
                &mut f,
            );
            self.paint_third_layer(
                req_tree.right().right().req().text(),
                3,
                Glyph::orange(),
                &mut f,
            );
        }
        if req_tree.left().left().req().is_logic_fn() {
            self.paint_fourth_layer(
                0,
                (
                    req_tree.left().left().left().req().text(),
                    req_tree.left().left().right().req().text(),
                ),
                Glyph::red(),
                &mut f,
            );
        }
        if req_tree.left().right().req().is_logic_fn() {
            self.paint_fourth_layer(
                2,
                (
                    req_tree.left().right().left().req().text(),
                    req_tree.left().right().right().req().text(),
                ),
                Glyph::red(),
                &mut f,
            );
        }
        if req_tree.right().left().req().is_logic_fn() {
            self.paint_fourth_layer(
                4,
                (
                    req_tree.right().left().left().req().text(),
                    req_tree.right().left().right().req().text(),
                ),
                Glyph::red(),
                &mut f,
            );
        }
        if req_tree.right().right().req().is_logic_fn() {
            self.paint_fourth_layer(
                6,
                (
                    req_tree.right().right().left().req().text(),
                    req_tree.right().right().right().req().text(),
                ),
                Glyph::red(),
                &mut f,
            );
        }
        mgr.swap_frame(self.g_id, 0, f);
        mgr.move_graphic(self.g_id, 1, (0, 0));
    }

    fn wrap_teeth(&self, t_id: usize, f: &mut Vec<Glyph>) {
        self.wrap_pillar(t_id * 2, Glyph::green(), 3, f);
        self.wrap_pillar(1 + t_id * 2, Glyph::green(), 3, f);
        self.wrap_l3(t_id, Glyph::green(), 2, f);
        self.wrap_l2(t_id >> 1, Glyph::green(), 1, f);
        self.wrap_pillar(t_id * 2, Glyph::blue(), 2, f);
        self.wrap_pillar(1 + t_id * 2, Glyph::blue(), 2, f);
        self.wrap_l3(t_id, Glyph::blue(), 1, f);
        self.wrap_pillar(t_id * 2, Glyph::orange(), 1, f);
        self.wrap_pillar(1 + t_id * 2, Glyph::orange(), 1, f);
    }

    fn wrap_pillar(&self, p_id: usize, color: Glyph, frame_thickness: usize, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.fourth[p_id], frame_thickness, color, f);
    }

    fn wrap_l3(&self, p_id: usize, color: Glyph, frame_thickness: usize, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.third[p_id], frame_thickness, color, f);
    }

    fn wrap_l2(&self, p_id: usize, color: Glyph, frame_thickness: usize, f: &mut Vec<Glyph>) {
        draw_a_box(self.size.0, &self.second[p_id], frame_thickness, color, f);
    }
}

fn draw_a_box(
    s_width: usize,
    dims: &Block,
    add_thickness: usize,
    color: Glyph,
    frame: &mut Vec<Glyph>,
) {
    for y in dims.start_y - add_thickness..dims.start_y + dims.size_y + add_thickness {
        for x in dims.start_x - add_thickness..dims.start_x + dims.size_x + add_thickness {
            frame[y * s_width + x] = color;
        }
    }
}

fn type_text(s_width: usize, text: String, dims: &Block, mut color: Glyph, frame: &mut Vec<Glyph>) {
    let mut chars = text.chars();
    let len = text.len();
    if dims.size_x > dims.size_y {
        let (start, end) = if len > dims.size_x {
            (
                (s_width * (dims.start_y + 1)) + dims.start_x,
                (s_width * (dims.start_y + 1)) + dims.start_x + dims.size_x,
            )
        } else {
            (
                (s_width * (dims.start_y + 1)) + dims.start_x + ((dims.size_x - len) >> 1),
                (s_width * (dims.start_y + 1)) + dims.start_x + ((dims.size_x - len) >> 1) + len,
            )
        };

        for x in start..end {
            if let Some(char) = chars.next() {
                color.set_char(char);
                frame[x] = color;
            } else {
                break;
            }
        }
    } else {
        let start = (s_width * (dims.start_y - 1)) + dims.start_x;
        let mut add_lines = 0;

        for i in 0..len {
            let i_mod = i % dims.size_x;
            if i_mod == 0 {
                add_lines = add_lines + s_width;
            }
            if let Some(char) = chars.next() {
                color.set_char(char);
                frame[start + add_lines + i_mod] = color;
            } else {
                break;
            }
        }
    }
}
