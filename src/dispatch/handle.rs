use std::io::{Error, ErrorKind};
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, atomic::{AtomicUsize, Ordering}
};
use crate::tuitty_core::{
    common::enums::{Color::*, Effect, Clear, Style},
};
use crate::internals::ScreenStore;
use super::events::EventEmitter;
use super::enums::{
    Action::{*, self}, Query::{*, self},
    Msg::Response, Reply
};
use super::Term;


type Emitters = Arc<Mutex<HashMap<usize, EventEmitter>>>;


pub fn suspend(id: usize, emitters: &Emitters) -> std::io::Result<()> {
    let err_msg = "Error acquiring emitters lock";
    let mut roster = match emitters.lock() {
        Ok(r) => r,
        Err(e) => return Err(Error::new(
            ErrorKind::Other,
            format!("{} (Cmd::Suspend): {:?}", err_msg, e)))
    };
    roster.entry(id).and_modify(|tx: &mut EventEmitter| {
        tx.is_suspend = true });
    
    Ok(())
}

pub fn transmit(id: usize, emitters: &Emitters) -> std::io::Result<()> {
    let err_msg = "Error acquiring emitters lock";
    let mut roster = match emitters.lock() {
        Ok(r) => r,
        Err(e) => return Err(Error::new(
            ErrorKind::Other,
            format!("{} (Cmd::Transmit): {:?}", err_msg, e)))
    };
    roster.entry(id).and_modify(|tx: &mut EventEmitter| {
        tx.is_suspend = false });

    Ok(())
}

pub fn stop(id: usize, emitters: &Emitters) -> std::io::Result<()> {
    let err_msg = "Error acquiring emitters lock";
    let mut roster = match emitters.lock() {
        Ok(r) => r,
        Err(e) => return Err(Error::new(
            ErrorKind::Other,
            format!("{} (Cmd::Stop): {:?}", err_msg, e)))
    };
    roster.entry(id).and_modify(|tx: &mut EventEmitter| {
        tx.is_running = false });

    Ok(())
}

pub fn lock(id: usize, lock_owner: &Arc<AtomicUsize>) {
    match lock_owner.load(Ordering::SeqCst) {
        0 => lock_owner.store(id, Ordering::SeqCst),
        _ => (),
    }
}

pub fn unlock(lock_owner: &Arc<AtomicUsize>) {
    match lock_owner.load(Ordering::SeqCst) {
        0 => (),
        _ => lock_owner.store(0, Ordering::SeqCst),
    }
}

pub fn signal(
    action: Action, term: &mut Term, store: &mut ScreenStore
) -> std::io::Result<()> {
    execute(action, term, store)
}

#[cfg(unix)]
pub fn request(
    query: Query, term: &mut Term, store: &mut ScreenStore,
    emitters: &Emitters, lock_owner: &Arc<AtomicUsize>
) -> std::io::Result<()> {
    examine(query, term, store, emitters, lock_owner)
}

#[cfg(windows)]
pub fn request(
    query: Query, term: &mut Term, store: &mut ScreenStore,
    emitters: &Emitters
) -> std::io::Result<()> {
    examine(query, term, store, emitters)
}


pub fn execute(
    action: Action, term: &mut Term, store: &mut ScreenStore
) -> std::io::Result<()> {
    match action {
        Goto(col, row) => {
            // Prevent out-of-bounds.
            let (w, h) = store.size();
            let (mut col, mut row) = (col, row);
            if col < 0 { col = 0 }
            if col >= w { col = w - 1 }
            if row < 0 { row = 0 }
            if row >= h { row = h - 1 }
            term.goto(col, row)?;
            store.sync_goto(col, row);
            Ok(())
        }

        Up(n) => {
            let (_, r) = store.coord();
            let mut n = n;
            if r - n < 0 { n = r }
            term.up(n)?;
            store.sync_up(n);
            Ok(())
        },
        
        Down(n) => {
            let (_, h) = store.size();
            let (_, r) = store.coord();
            let mut n = n;
            if r + n >= h { n = (h - r) - 1 }
            term.down(n)?;
            store.sync_down(n);
            Ok(())
        },

        Left(n) => {
            let (c, _) = store.coord();
            let mut n = n;
            if c - n < 0 { n = c }
            term.left(n)?;
            store.sync_left(n);
            Ok(())
        },
        
        Right(n) => {
            let (w, _) = store.size();
            let (c, _) = store.coord();
            let mut n = n;
            if c + n >= w { n = (w - c) - 1 }
            term.right(n)?;
            store.sync_right(n);
            Ok(())
        },

        Clear(clr) => {
            term.clear(clr)?;
            store.sync_clear(clr);
            Ok(())
        },
        
        Resize(w, h) => {
            term.resize(w, h)?;
            store.sync_size(w, h);
            Ok(())
        },
        
        Prints(s) => {
            term.prints(&s)?;
            store.sync_content(&s);
            Ok(())
        },

        Printf(s) => {
            term.printf(&s)?;
            store.sync_content(&s);
            Ok(())
        },

        SetContent(s, c, r) => {
            store.sync_goto(c, r);
            store.sync_content(&s);
            Ok(())
        },

        // #[cfg(windows)]
        // TODO: Paints(buf, size, offset, rect)
 
        Flush => term.flush(),
        Render => store.render(&term),
        Refresh => store.refresh(&term),

        SetFx(fx) => {
            term.set_fx(fx)?;
            store.sync_style(Style::Fx(fx));
            Ok(())
        },

        SetFg(c) => {
            term.set_fg(c)?;
            store.sync_style(Style::Fg(c));
            Ok(())
        },

        SetBg(c) => {
            term.set_bg(c)?;
            store.sync_style(Style::Bg(c));
            Ok(())
        },

        SetStyles(f, b, fx) => {
            term.set_styles(f, b, fx)?;
            store.sync_styles(f, b, fx);
            Ok(())
        },

        ResetStyles => {
            term.reset_styles()?;
            let (c, fx) = (Reset, Effect::Reset as u32);
            store.sync_styles(c, c, fx);
            Ok(())
        },

        // STATEFUL/MODES
        HideCursor => {
            term.hide_cursor()?;
            store.sync_cursor(false);
            Ok(())
        },

        ShowCursor => {
            term.show_cursor()?;
            store.sync_cursor(true);
            Ok(())
        },

        EnableMouse => {
            term.enable_mouse()?;
            store.sync_mouse(true);
            Ok(())
        },

        DisableMouse => {
            term.disable_mouse()?;
            store.sync_mouse(false);
            Ok(())
        },

        // EnableAlt => term.enable_alt(),
        // DisableAlt => term.disable_alt(),

        Raw => {
            term.raw()?;
            store.sync_raw(true);
            Ok(())
        },

        Cook => {
            term.cook()?;
            store.sync_raw(false);
            Ok(())
        },

        // SWITCHING SCREENS
        NewScreen => {
            if store.id() == 0 { term.enable_alt()? }
            else { term.clear(Clear::All)? }
            let (w, h) = store.size();
            store.new_screen(w, h);
            term.cook()?;
            term.disable_mouse()?;
            term.show_cursor()?;
            term.reset_styles()?;
            term.goto(0, 0)?;
            #[cfg(unix)]
            term.flush()?;
            Ok(())
        },

        SwitchTo(id) => {
            let current = store.id();
            // Bounds checking:
            if current == id { return Ok(()) }
            if store.exists(id) { store.set(id)? }
            else { return Ok(()) }
            // Handle screen switch:
            // Disable if you are reverting back to main.
            if id == 0 { term.disable_alt()? }
            else {
                // Enable as you are on the main screen
                // switching to alternate.
                if current == 0 { term.enable_alt()? }
                term.clear(Clear::All)?;
            }
            if id != 0 { store.render(&term)? }
            // Restore settings based on metadata.
            let (raw, mouse, show) = (
                store.is_raw(),
                store.is_mouse(),
                store.is_cursor() );

            if raw { term.raw()? }
            else { term.cook()? }
            if mouse { term.enable_mouse()? }
            else { term.disable_mouse()? }
            if show { term.show_cursor()? }
            else { term.hide_cursor()? }
            #[cfg(unix)]
            term.flush()?;
            Ok(())
        },

        // INTERNAL BUFFER UPDATES
        Resized => {
            let (w, h) = term.size()?;
            store.sync_size(w, h);
            Ok(())
        },
        SyncMarker(c,r) => Ok(store.sync_marker(c,r)),
        Jump => Ok(store.jump()),
        SyncTabSize(n) => Ok(store.sync_tab_size(n)),
    }
}


pub fn examine(
    query: Query, term: &mut Term, store: &ScreenStore, emitters: &Emitters,
    #[cfg(unix)]
    lock_owner: &Arc<AtomicUsize>
) -> std::io::Result<()> {
    match query {
        Size(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::Size): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let (w, h) = store.size();
                match tx.event_tx.send(Response(Reply::Size(w, h))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::Size): {:?}", err_msg, e)))
                }
            }
            Ok(())
        },

        Coord(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::Coord): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let (col, row) = store.coord();
                match tx.event_tx.send(Response(Reply::Coord(col, row))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::Coord): {:?}", err_msg, e)))
                }
            }
            Ok(())
        },

        #[cfg(unix)]
        Pos(id) => {
            // Lock the receiver that requested pos:
            match lock_owner.load(super::Ordering::SeqCst) {
                0 => lock_owner
                    .store(id, super::Ordering::SeqCst),
                _ => return Ok(()),
            }
            term.query_pos()?;
            // Now unlock the receiver after pos call:
            match lock_owner.load(super::Ordering::SeqCst) {
                0 => return Ok(()),
                _ => Ok(lock_owner
                    .store(0, super::Ordering::SeqCst)),
            }
        },

        #[cfg(windows)]
        Pos(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::Pos): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let (col, row) = term.pos()?;
                let err_msg = "Error sending response";
                match tx.event_tx.send(Response(Reply::Pos(col, row))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::Pos): {:?}", err_msg, e)))
                }
            }
            Ok(())
        },

        GetCh(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::GetCh): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let s = store.getch();
                let err_msg = "Error sending response";
                match tx.event_tx.send(Response(Reply::GetCh(s))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::GetCh): {:?}", err_msg, e)))
                }
            }
            Ok(())
        },

        Screen(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::Screen): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let i = store.id();
                let err_msg = "Error sending response";
                match tx.event_tx.send(Response(Reply::Screen(i))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::Screen): {:?}", err_msg, e)))
                }
            }
            Ok(())
        },

        // Internal Use Only
        _IsRaw(id) => {
            let err_msg = "Error acquiring emitters lock";
            let roster = match emitters.lock() {
                Ok(r) => r,
                Err(e) => return Err(Error::new(
                    ErrorKind::Other,
                    format!("{} (Query::_IsRaw): {:?}", err_msg, e)))
            };
            if let Some(tx) = roster.get(&id) {
                let b = store.is_raw();
                let err_msg = "Error sending response";
                match tx.event_tx.send(Response(Reply::_IsRaw(b))) {
                    Ok(_) => (),
                    Err(e) => return Err(Error::new(
                        ErrorKind::Other,
                        format!("{} (Reply::_IsRaw): {:?}", err_msg, e)))
                }
            }
            Ok(())
        }
    }
}