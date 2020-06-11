use std::sync::mpsc::{Sender, Receiver, SendError};
#[cfg(unix)]
use super::InputEvent;
use super::enums::{
    Cmd::{*, self}, Query::*,
    Msg, Action, Reply
};


pub struct EventEmitter {
    pub event_tx: Sender<Msg>,
    pub is_suspend: bool,
    pub is_running: bool,
}


pub struct EventReceiver {
    pub id: usize,
    pub event_rx: Receiver<Msg>,
    pub signal_tx: Sender<Cmd>,
}

impl EventReceiver {
    pub fn poll_async(&self) -> Option<Msg> {
        let mut iterator = self.event_rx.try_iter();
        iterator.next()
    }

    pub fn poll_latest_async(&self) -> Option<Msg> {
        let iterator = self.event_rx.try_iter();
        iterator.last()
    }

    pub fn poll_sync(&self) -> Option<Msg> {
        let mut iterator = self.event_rx.iter();
        iterator.next()
    }

    pub fn suspend(&self) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Suspend(self.id))
    }

    pub fn transmit(&self) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Transmit(self.id))
    }

    pub fn stop(&self) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Stop(self.id))
    }

    pub fn lock(&self) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Lock(self.id))
    }

    pub fn unlock(&self) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Unlock)
    }

    pub fn signal(&self, action: Action) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Signal(action))
    }

    pub fn request(&self, query: &str) -> Result<Reply, SendError<Cmd>> {
        match query {
            "coord" => {
                self.signal_tx.send(Request(Coord(self.id)))?;
            },
            #[cfg(unix)]
            "raw_pos" => {
                // Determine if the current screen is in raw mode.
                self.signal_tx.send(Request(_IsRaw(self.id)))?;
                let mut iter = self.event_rx.iter();
                let is_raw: bool;
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        if let Reply::_IsRaw(b) = r {
                            is_raw = b;
                            break
                        }
                    }
                }
                // Set it to raw temporarily, if not in raw mode.
                if !is_raw { self.signal_tx.send(Signal(Action::Raw))? }
                // Request the cursor position and 
                self.signal_tx.send(Request(Pos(self.id)))?;
                loop {
                    if let Some(Msg::Received(iv)) = iter.next() {
                        if let InputEvent::CursorPos(col, row) = iv {
                            // Revert back to cooked mode.
                            if !is_raw { 
                                self.signal_tx.send(Signal(Action::Cook))?
                            }
                            return Ok(Reply::Pos(col, row));
                        }
                    }
                }
            },
            #[cfg(windows)]
            "raw_pos" => {
                self.signal_tx.send(Request(Pos(self.id)))?;
            },
            "getch" => {
                self.signal_tx.send(Request(GetCh(self.id)))?;
            },
            "size" => {
                self.signal_tx.send(Request(Size(self.id)))?;
            },
            "screen" => {
                self.signal_tx.send(Request(Screen(self.id)))?;
            },
            _ => return Ok(Reply::Empty)
        }
        let mut iter = self.event_rx.iter();
        loop {
            if let Some(Msg::Response(r)) = iter.next() {
                return Ok(r)
            }
        }
    }
}
