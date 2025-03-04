use std::collections::HashMap;
use std::time::Duration;

use animaterm::Glyph;
use animaterm::Graphic;
use animaterm::Manager;
use async_std::channel::Receiver;
use async_std::channel::Sender;
use async_std::task::sleep;
use async_std::task::spawn;
use std::sync::mpsc::Sender as SyncSender;

use super::ToPresentation;

enum NotifierState {
    SlidingIn(u8),
    Presenting,
    SlidingOut(u8),
    OffScreen,
}
pub struct Notifier {
    //TODO
    pub id: usize,
    cols: usize,
    rows: usize,
    state: NotifierState,
    buffer: Vec<String>,
    sender: Sender<Option<String>>,
    receiver: Receiver<Option<String>>,
    tui_sender: SyncSender<ToPresentation>,
}
//TODO: Create an async channel for Notifier to receive notifications.
// We need also a buffer to store queued notifications, if we receive multiple
// notifications at once.
// Then we need to display those one by one.
impl Notifier {
    pub fn new(
        offset: (isize, isize),
        mgr: &mut Manager,
        (sender, receiver): (Sender<Option<String>>, Receiver<Option<String>>),
        tui_sender: SyncSender<ToPresentation>,
    ) -> Self {
        let cols = 30;
        let rows = 3;
        let g = Glyph::red();
        let frame = vec![g; cols * rows];
        let mut library = HashMap::new();
        library.insert(0, frame);
        let n_graphic = Graphic::new(cols, rows, 0, library, None);
        let id = mgr.add_graphic(n_graphic, 4, offset).unwrap();
        mgr.set_graphic(id, 0, true);
        Notifier {
            id,
            cols,
            rows,
            state: NotifierState::OffScreen,
            buffer: Vec::with_capacity(16),
            sender,
            receiver,
            tui_sender,
        }
    }
    //TODO: we can not modify screen contents from Notifier,
    // since we do not have acces to TUIManager
    // Hence we need to send a ToPresentation messages
    // in order for screen to change
    // We need to define two new messages:
    // 1. SetNotification(usize, Vec<Glyph>) // usize being graphic's id
    // 2. MoveNotification(usize, (isize,isize))
    //
    // Notifier should have a state indicating which phase he is currently in.
    pub async fn serve(mut self) {
        loop {
            // eprintln!("Notifier waiting…");
            let recv_result = self.receiver.recv().await;
            // eprintln!("Notifier ggot: {:?}", recv_result);
            match recv_result {
                Ok(Some(new_note)) => {
                    //TODO: display a new_note or add it to buffer
                    match self.state {
                        NotifierState::OffScreen => {
                            self.state = NotifierState::SlidingIn(self.cols as u8);
                            let note_frame = self.prepare_note(new_note);
                            //TODO: spawn a timer
                            spawn(timer(self.sender.clone(), self.cols));
                            // eprintln!("Timer spawned");
                            let _res = self
                                .tui_sender
                                .send(ToPresentation::SetNotification(self.id, note_frame));
                            eprintln!("SetNote result: {:?}", _res);
                        }
                        _ => {
                            self.buffer.push(new_note);
                        }
                    }
                }
                Ok(None) => match self.state {
                    NotifierState::SlidingIn(step) => {
                        let next_step = step.wrapping_sub(1);
                        self.state = if next_step == 0 {
                            NotifierState::Presenting
                        } else {
                            NotifierState::SlidingIn(next_step)
                        };
                        let _ = self
                            .tui_sender
                            .send(ToPresentation::MoveNotification(self.id, (-1, 0)));
                    }
                    NotifierState::Presenting => {
                        self.state = NotifierState::SlidingOut(self.cols as u8);
                    }
                    NotifierState::SlidingOut(step) => {
                        let next_step = step.wrapping_sub(1);
                        self.state = if next_step == 0 {
                            NotifierState::OffScreen
                        } else {
                            NotifierState::SlidingOut(next_step)
                        };
                        let _ = self
                            .tui_sender
                            .send(ToPresentation::MoveNotification(self.id, (1, 0)));
                        if !self.buffer.is_empty() {
                            let next_note = self.buffer.remove(0);
                            let _ = self.sender.send(Some(next_note)).await;
                        }
                    }
                    NotifierState::OffScreen => {
                        eprintln!("Did not expect a timeout");
                    }
                },
                Err(err) => {
                    eprintln!("Notifier terminating ({:?})", err);
                    break;
                }
            }
        }
    }
    fn prepare_note(&self, text: String) -> Vec<Glyph> {
        let mut frame = Vec::with_capacity(self.cols * self.rows);
        let mut red = Glyph::red();
        for _i in 0..self.cols {
            frame.push(red);
        }
        let mut remaining_glyphs = self.cols;
        for (i, char) in text.chars().enumerate() {
            if i >= self.cols {
                break;
            }
            remaining_glyphs -= 1;
            red.set_char(char);
            frame.push(red);
        }
        red.set_char(' ');
        for _i in 0..self.cols + remaining_glyphs {
            frame.push(red);
        }
        frame
    }
}

async fn timer(sender: Sender<Option<String>>, counter: usize) {
    let step = Duration::from_millis(500 / counter as u64);
    for _i in 0..counter {
        sleep(step).await;
        let _ = sender.send(None).await;
    }
    sleep(Duration::from_secs(3)).await;
    let _ = sender.send(None).await;
    for _i in 0..counter {
        sleep(step).await;
        let _ = sender.send(None).await;
    }
}
