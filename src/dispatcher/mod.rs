mod router;
use router::handle_action;

use std::{
    thread, collections::HashMap,
    time::{ SystemTime, UNIX_EPOCH, Duration },
    sync::{
        mpsc::{ channel, Sender, Receiver, SendError, TryRecvError },
        Arc, Mutex, atomic::{ AtomicBool, AtomicUsize, Ordering },
    },
};
use crate::store::Store;
use message::{
    Action, Cmd::{*, self},
    Msg::{*, self}, Query::*, Reply,
};
use crate::tuitty_core::terminal::Term;
use crate::tuitty_core::common::enums::InputEvent;

#[cfg(unix)]
use crate::tuitty_core::parser::unix;
#[cfg(windows)]
use crate::tuitty::parser::windows;

pub mod message;

const DELAY: u64 = 3;

pub struct EventHandle {
    id: usize,
    event_rx: Receiver<Msg>,
    signal_tx: Sender<Cmd>,
}

impl EventHandle {
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
                let mut iter = self.event_rx.iter();
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        return Ok(r)
                    }
                }
            },
            "pos" => {
                // Determine if the current screen is in raw mode.
                self.signal_tx.send(Request(IsRaw(self.id)))?;
                let mut iter = self.event_rx.iter();
                let is_raw: bool;
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        if let Reply::IsRaw(b) = r {
                            is_raw = b;
                            break
                        }
                    }
                }
                // Set it to raw temporarily, if not in raw mode.
                if !is_raw { self.signal_tx.send(Signal(Action::Raw))? }
                // Request the cursor position and 
                self.signal_tx.send(Request(Pos(self.id)))?;
                let mut iter = self.event_rx.iter();
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
            "getch" => {
                self.signal_tx.send(Request(GetCh(self.id)))?;
                let mut iter = self.event_rx.iter();
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        return Ok(r)
                    }
                }
            },
            "size" => {
                self.signal_tx.send(Request(Size(self.id)))?;
                let mut iter = self.event_rx.iter();
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        return Ok(r)
                    }
                }
            },
            "screen" => {
                self.signal_tx.send(Request(Screen(self.id)))?;
                let mut iter = self.event_rx.iter();
                loop {
                    if let Some(Msg::Response(r)) = iter.next() {
                        return Ok(r)
                    }
                }
            },
            _ => Ok(Reply::Empty)
        }
    }

    // TODO: CursorPos
    // pub fn syspos(&self) -> (i16, i16) {
    //     let _ = self.signal_tx.send(
    //         Request(State::SysPos(self.id)));
    //     let mut iter = self.event_rx.iter();
    //     loop {
    //         if let Some(Dispatch(SysPos(col, row))) = iter.next() {
    //             return (col, row)
    //         }
    //     }
    // }
}


struct EventEmitter {
    event_tx: Sender<Msg>,
    is_suspend: bool,
    is_running: bool,
}


pub struct Dispatcher {
    // Thread handle to send keyboard and mouse events
    // through each emitter's event_tx to the owner of
    // the event_rx handle (single produce/ single consumer)
    input_handle: Option<thread::JoinHandle<()>>,
    emitters: Arc<Mutex<HashMap<usize, EventEmitter>>>,
    // Broadcast to select owner(s) of the lock.
    lock_owner: Arc<AtomicUsize>,
    // The Dispatcher can also signal commands that is handled
    // by the singal thread handle's signal_rx. This implements
    // the mpsc pattern to allow for multithreaded use cases.
    // 
    // The signal_tx is also cloned into each EventHandle (MP),
    // however, the signal_rx is kept within the signal thread's
    // inner loop (SC).
    signal_tx: Sender<Cmd>,
    signal_handle: Option<thread::JoinHandle<()>>,
    // Handle graceful shutdown and clean up.
    is_running: Arc<AtomicBool>
}

impl Dispatcher {
    pub fn init() -> Self {
        // Initialize struct fields.
        let emitters = Arc::new(Mutex::new(HashMap::with_capacity(8)));
        let is_running = Arc::new(AtomicBool::new(true));
        let lock_owner = Arc::new(AtomicUsize::new(0));

        // Setup Atomic References to move into thread.
        let emitters_ref = emitters.clone();
        let is_running_ref = is_running.clone();
        let lock_owner_ref = lock_owner.clone();

        // Fetch terminal default state in main thread.
        #[cfg(unix)]
        let (col, row, tab_size) = match fetch_defaults() {
            Ok((col, row, tab_size)) =>
                (col, row, tab_size),
            Err(e) => panic!("Error fetching terminal defaults: {:?}", e)
        };

        #[cfg(windows)]
        let (mode, reset, ansi, col, row, tab_size) = match fetch_defaults() {
            Ok((mode, reset, ansi, col, row, tab_size)) =>
                (mode, reset, ansi, col, row, tab_size),
            Err(e) => panic!("Error fetching terminal defaults: {:?}", e)
        };

         // Start signal loop.
        let (signal_tx, signal_rx) = channel();
        let signal_handle = thread::spawn(move || {
            let mut term = Term::new()
                .expect("Error initializing the Term struct.");
            #[cfg(windows)]
            term.with(mode, reset, ansi);
            // Initialize the internal buffer.
            let (w, h) = term.size().expect("Error fetching terminal size.");
            let mut store = Store::new(w, h);
            store.sync_tab_size(tab_size);
            store.sync_goto(col, row);

            loop {
                // Include minor delay so the thread isn't blindly using CPU.
                thread::sleep(Duration::from_millis(DELAY));
                // Handle signal commands.
                match signal_rx.try_recv() {
                    Ok(cmd) => match cmd {
                        Continue => (),

                        Suspend(id) => {
                            let mut roster = match emitters_ref.lock() {
                                Ok(r) => r,
                                Err(_) => match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => continue
                                },
                            };
                            roster.entry(id)
                                .and_modify(|tx: &mut EventEmitter| {
                                    tx.is_suspend = true 
                                });
                        },

                        Transmit(id) => {
                            let mut roster = match emitters_ref.lock() {
                                Ok(r) => r,
                                Err(_) => match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => continue
                                },
                            };
                            roster.entry(id)
                                .and_modify(|tx: &mut EventEmitter| {
                                    tx.is_suspend = false
                                });
                        },

                        Stop(id) => {
                            let mut roster = match emitters_ref.lock() {
                                Ok(r) => r,
                                Err(_) => match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => continue
                                },
                            };
                            roster.entry(id)
                                .and_modify(|tx: &mut EventEmitter| {
                                    tx.is_running = false
                                });
                        },

                        Lock(id) => {
                            match lock_owner_ref.load(Ordering::SeqCst) {
                                0 => lock_owner_ref
                                    .store(id, Ordering::SeqCst),
                                _ => continue,
                            }
                        },

                        Unlock => {
                            match lock_owner_ref.load(Ordering::SeqCst) {
                                0 => continue,
                                _ => lock_owner_ref
                                    .store(0, Ordering::SeqCst),
                            }
                        },

                        Signal(action) => {
                            match handle_action(action, &mut term, &mut store) {
                                Ok(_) => (),
                                Err(e) => {}
                            }
                        },

                        Request(query) => match query {
                            Size(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let (w, h) = store.size();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::Size(w, h)));
                                }
                            },

                            Coord(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let (col, row) = store.coord();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::Coord(col, row)));
                                }
                            },

                            #[cfg(unix)]
                            Pos(id) => {
                                // Lock the receiver that requested pos:
                                match lock_owner_ref.load(Ordering::SeqCst) {
                                    0 => lock_owner_ref
                                        .store(id, Ordering::SeqCst),
                                    _ => continue,
                                }
                                match term.query_pos() {
                                    Ok(_) => (),
                                    Err(_) => match term.query_pos() {
                                        Ok(_) => (),
                                        Err(_) => {
                                            is_running_ref.store(false, 
                                                Ordering::SeqCst);
                                            break
                                        }
                                    }
                                }
                                // Now unlock the receiver after pos call:
                                match lock_owner_ref.load(Ordering::SeqCst) {
                                    0 => continue,
                                    _ => lock_owner_ref
                                        .store(0, Ordering::SeqCst),
                                }
                            },

                            #[cfg(windows)]
                            Pos(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let (col, row) = term.pos();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::Pos(col, row)));
                                }
                            },

                            GetCh(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let s = store.getch();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::GetCh(s)));
                                }
                            },

                            Screen(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let i = store.id();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::Screen(i)));
                                }
                            },

                            IsRaw(id) => {
                                let roster = match emitters_ref.lock() {
                                    Ok(r) => r,
                                    Err(_) => match emitters_ref.lock() {
                                        Ok(r) => r,
                                        Err(_) => continue
                                    },
                                };
                                if let Some(tx) = roster.get(&id) {
                                    let b = store.is_raw();
                                    let _ = tx.event_tx.send(Response(
                                        Reply::IsRaw(b)));
                                }
                            }
                        }
                    },

                    Err(e) => match e {
                        TryRecvError::Empty => if !is_running_ref
                            .load(Ordering::SeqCst) { break },
                        TryRecvError::Disconnected => is_running_ref
                            .store(false, Ordering::SeqCst),
                    }
                } // End match.
            } // End loop.
        }); // End signal thread.
        
        Self {
            input_handle: None,
            emitters, lock_owner,
            signal_tx, is_running,
            signal_handle: Some(signal_handle),
        }
    }

    pub fn listen(&mut self) -> EventHandle {
        // Do not duplicate threads.
        // If input handle exists, spawn another event handle.
        if self.input_handle.is_some() { return self.spawn() }

        // Setup input channel and Arc's to move to thread.
        let is_running = self.is_running.clone();
        let lock_owner = self.lock_owner.clone();
        let emitters_ref = self.emitters.clone();

        // Begin reading user input.
        #[cfg(unix)] {
        self.input_handle = Some(thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                let tty = match std::fs::OpenOptions::new()
                    .read(true).write(true).open("/dev/tty")
                {
                    Ok(f) => std::io::BufReader::new(f),
                    Err(_) => continue
                };
                let (mut input, mut taken) = (
                    [0; 12], std::io::Read::take(tty, 12));
                let _ = std::io::Read::read(&mut taken, &mut input);

                // Emitters clean up.
                let mut roster = match emitters_ref.lock() {
                    Ok(r) => r,
                    Err(_) => match emitters_ref.lock() {
                        Ok(r) => r,
                        Err(_) => continue
                    },
                };
                if !roster.is_empty() {
                    roster.retain( |_, tx: &mut EventEmitter| {
                        tx.is_running
                    })
                }
                // Parse the user input from /dev/tty.
                let item = input[0];
                let mut rest = input[1..].to_vec().into_iter();
                let evt = unix::parse_event(item, &mut rest);
                // Push user input event.
                match lock_owner.load(Ordering::SeqCst) {
                    0 => {
                        for (_, tx) in roster.iter() {
                            if tx.is_suspend { continue }
                            let _ = tx.event_tx.send(
                                Received(evt.clone()));
                        }
                    },
                    id => match roster.get(&id) {
                        Some(tx) => {
                            let _ = tx.event_tx.send(
                                Received(evt.clone())); 
                        },
                        None => lock_owner.store(0, Ordering::SeqCst),
                    }
                }
                thread::sleep(Duration::from_millis(DELAY));
            }
        }))}

        #[cfg(windows)] {
        self.input_handle = Some(thread::spawn(move || {
            while is_running.load(Ordering::SeqCst) {
                let (_, evts) = windows::read_input_events();
                for evt in evts {
                    // Emitters clean up.
                    let mut roster = match emitters_ref.lock() {
                        Ok(r) => r,
                        Err(_) => match emitters_ref.lock() {
                            Ok(r) => r,
                            Err(_) => continue
                        },
                    };
                    if !roster.is_empty() {
                        roster.retain( |_, tx: &mut EventEmitter| {
                            tx.is_running
                        })
                    }
                    // Push user input event.
                    match lock_owner.load(Ordering::SeqCst) {
                        0 => {
                            for (_, tx) in roster.iter() {
                                if tx.is_suspend { continue }
                                let _ = tx.event_tx.send(
                                    Received(evt.clone()));
                            }
                        },
                        id => match roster.get(&id) {
                            Some(tx) => { 
                                let _ = tx.event_tx.send(
                                    Received(evt.clone()));
                            },
                            None => lock_owner.store(0, Ordering::SeqCst),
                        }
                    }
                }
                thread::sleep(Duration::from_millis(DELAY));
            }
        }))}

        self.spawn()
    }

    fn randomish(&self) -> usize {
        match self.emitters.lock() {
            Ok(senders) => {
                let mut key: usize;
                loop {
                    key = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Error fetching duration since 1970")
                        .subsec_nanos() as usize;
                    if key == 0 { continue }
                    if !senders.contains_key(&key) { break }
                }
                key
            },
            Err(_) => {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Error fetching duration since 1970")
                    .subsec_nanos() as usize
            }
        }
    }

    pub fn spawn(&self) -> EventHandle {
        // let err_msg = "Error obtaining emitter registry lock";
        let (event_tx, event_rx) = channel();
        let id = self.randomish();
        let (is_suspend, is_running) = (false, true);
        match self.emitters.lock() {
            Ok(mut roster) => {
                roster.insert(id, EventEmitter{
                    event_tx, is_suspend, is_running
                });
            },
            Err(_) => match self.emitters.lock() {
                Ok(mut roster) => {
                    roster.insert(id, EventEmitter{
                        event_tx, is_suspend, is_running 
                    });
                },
                Err(e) => {}
            },
        };
        let signal_tx = self.signal_tx.clone(); 
        EventHandle { id, event_rx, signal_tx }
    }

    pub fn signal(&self, action: Action) -> Result<(), SendError<Cmd>> {
        self.signal_tx.send(Signal(action))
    }

    fn shutdown(&mut self) -> std::thread::Result<()> {
        self.is_running.store(false, Ordering::SeqCst);
        // (imdaveho) TODO: Since reading /dev/tty is blocking
        // we ignore this for now as it will clean up when the
        // program ends (and Dispatcher is dropped).
        // if let Some(t) = self.input_handle.take() { t.join()? }

        // Clear the emitters registery.
        // let lock_err = "Error obtaining emitters lock";
        // let mut roster = self.emitters.lock().expect(lock_err);
        // roster.clear();
        match self.emitters.lock() {
            Ok(mut roster) => roster.clear(),
            Err(_) => match self.emitters.lock() {
                Ok(mut roster) => roster.clear(),
                Err(e) => {}
            },
        }
        if let Some(t) = self.signal_handle.take() { t.join()? }
        // (imdaveho) NOTE: `term` should have drop/closed when the
        // signal_handle joined/finished.

        println!("\r\n");

        Ok(())
    }
}

impl Drop for Dispatcher {
    fn drop(&mut self) {
        self.shutdown().expect("Error on shutdown")
    }
}


#[cfg(unix)]
fn fetch_defaults() -> std::io::Result<(i16, i16, usize)> {
    let term = Term::new()?;
    term.raw()?;
    let (col, row) = term.raw_pos()?;
    term.printf("\t")?;
    let (tab_col, _) = term.raw_pos()?;
    term.cook()?;
    let tab_size = (tab_col - col) as usize;
    term.printf("\r")?;
    Ok((col, row, tab_size))
}


#[cfg(windows)]
fn fetch_defaults() -> std::io::Result<(u32, u16, bool, i16, i16, usize)> {
    let term = Term::new()?;
    let (mode, reset, ansi) = term.init_data();
    let (col, row) = term.pos()?;
    term.printf("\t")?;
    let (tab_col, _) = term.pos()?;
    let tab_size = (tab_col - col) as usize;
    term.printf("\r")?;
    Ok((mode, reset, ansi, col, row, tab_size))
}
