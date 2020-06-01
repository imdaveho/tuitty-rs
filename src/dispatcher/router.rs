use crate::tuitty::terminal::Term;
use crate::tuitty::common::enums::{Color::*, Effect};
use crate::store::Store;
use super::message::Action::{*, self};


// pub fn match_signal(action: Action, term: &mut Term, store: &mut Store) {
pub fn handle_action(action: Action, term: &mut Term, store: &mut Store) {
    match action {
        Goto(col, row) => {
            // Prevent out-of-bounds.
            // let (w, h) = store.size();
            // let (mut col, mut row) = (col, row);
            // if col < 0 { col = 0 }
            // if col >= w { col = w - 1 }
            // if row < 0 { row = 0 }
            // if row >= h { row = h - 1 }
            #[cfg(unix)]
            term.goto(col, row);

            #[cfg(windows)]
            match term.goto(col, row) {
                Ok(_) => (),
                Err(e) => {}
            }
            store.sync_goto(col, row);
        }

        Up(n) => {
            #[cfg(unix)]
            term.up(n);

            #[cfg(windows)] {
                // let (_, r) = store.coord();
                // let mut n = n;
                // if r - n < 0 { n = r }
                match term.up(n) {
                    Ok(_) => (),
                    Err(e) => {}
                }
            }
            // store.sync_up(n);
        },
        
        Down(n) => {
            #[cfg(unix)]
            term.down(n);

            #[cfg(windows)] {
                // let (_, h) = store.size();
                // let (_, r) = store.coord();
                // let mut n = n;
                // if r + n >= h { n = (h - r) - 1 }
                match term.down(n) {
                    Ok(_) => (),
                    Err(e) => {}
                }
            }
            // store.sync_down(n);
        },

        Left(n) => {
            #[cfg(unix)]
            term.left(n);

            #[cfg(windows)] {
                // let (c, _) = store.coord();
                // let mut n = n;
                // if c - n < 0 { n = c }
                match term.left(n) {
                    Ok(_) => (),
                    Err(e) => {}
                }
            }
            // store.sync_left(n);
        },
        
        Right(n) => {
            #[cfg(unix)]
            term.right(n);
            
            #[cfg(windows)] {
            //     let (w, _) = store.size();
            //     let (c, _) = store.coord();
            //     let mut n = n;
            //     if c + n >= w { n = (w - c) - 1 }
                match term.right(n) {
                    Ok(_) => (),
                    Err(e) => {}
                }
            }
            // store.sync_right(n);
        },

        // TODO: CursorPos

        Clear(clr) => {
            #[cfg(unix)]
            term.clear(clr);

            #[cfg(windows)]
            match term.clear(clr) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_clear(clr);
        },
        
        Resize(w, h) => {
            #[cfg(unix)]
            term.resize(w, h);

            #[cfg(windows)]
            match term.resize(w, h) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_size(w, h);
        },
        
        Prints(s) => {
            #[cfg(unix)]
            term.prints(&s);
            
            #[cfg(windows)]
            match term.prints(&s) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_content(&s);
        },
        
        Printf(s) => {
            #[cfg(unix)]
            term.printf(&s);

            #[cfg(windows)]
            match term.printf(&s) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_content(&s);
        },

        Flush => term.flush(),

        SetFx(fx) => {
            #[cfg(unix)]
            term.set_fx(fx);

            #[cfg(windows)]
            match term.set_fx(fx) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_style(Style::Fx(fx));
        },

        SetFg(c) => {
            #[cfg(unix)]
            term.set_fg(c);
            #[cfg(windows)]
            match term.set_fg(c) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_style(Style::Fg(c));
        },

        SetBg(c) => {
            #[cfg(unix)]
            term.set_bg(c);
            #[cfg(windows)]
            match term.set_bg(c) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_style(Style::Bg(c));
        },

        SetStyles(f, b, fx) => {
            #[cfg(unix)]
            term.set_styles(f, b, fx);
            #[cfg(windows)]
            match term.set_styles(f, b, fx) {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_styles(f, b, fx);
        },

        ResetStyles => {
            #[cfg(unix)]
            term.reset_styles();
            #[cfg(windows)]
            match term.reset_styles() {
                Ok(_) => (),
                Err(e) => {}
            }
            let (c, fx) = (Reset, Effect::Reset as u32);
            // store.sync_styles(c, c, fx);
        },

        // STATEFUL/MODES
        HideCursor => {
            #[cfg(unix)]
            term.hide_cursor();

            #[cfg(windows)]
            match term.hide_cursor() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_cursor(false);
        },

        ShowCursor => {
            #[cfg(unix)]
            term.show_cursor();

            #[cfg(windows)]
            match term.show_cursor() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_cursor(true);
        },

        EnableMouse => {
            #[cfg(unix)]
            term.enable_mouse();

            #[cfg(windows)]
            match term.enable_mouse() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_mouse(true);
        },

        DisableMouse => {
            #[cfg(unix)]
            term.disable_mouse();

            #[cfg(windows)]
            match term.disable_mouse() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_mouse(false);
        },

        EnableAlt => {
            #[cfg(unix)]
            term.enable_alt();

            #[cfg(windows)]
            match term.enable_alt() {
                Ok(_) => (),
                Err(e) => {}
            }
        },

        DisableAlt => {
            #[cfg(unix)]
            term.disable_alt();

            #[cfg(windows)]
            match term.disable_alt() {
                Ok(_) => (),
                Err(e) => {}
            }
        },

        Raw => {
            #[cfg(unix)]
            term.raw();

            #[cfg(windows)]
            match term.raw() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_raw(true);
        },

        Cook => {
            #[cfg(unix)]
            term.cook(&initial);

            #[cfg(windows)]
            match term.cook() {
                Ok(_) => (),
                Err(e) => {}
            }
            // store.sync_raw(false);
        },

        // SWITCHING SCREENS
        #[cfg(unix)]
        Switch => {
            // if store.id() == 0 {
            //     posix::enable_alt();
            // } else {
            //     posix::clear(Clear::All);
            // }
            // store.new_screen();
            // posix::cook(&initial);
            // posix::disable_mouse();
            // posix::show_cursor();
            // posix::reset_styles();
            // posix::goto(0, 0);
            // posix::flush();
        },
        #[cfg(windows)]
        Switch => {
            // if store.id() == 0 {
            //     win32::enable_alt(
            //         &screen, &initial, vte);
            //     win32::clear(Clear::All, vte);
            // } else {
            //     win32::clear(Clear::All, vte);
            // }
            // store.new_screen();
            // win32::cook();
            // win32::disable_mouse(vte);
            // win32::show_cursor(vte);
            // win32::reset_styles(default, vte);
            // win32::goto(0, 0, vte);
        },
        #[cfg(unix)]
        SwitchTo(id) => {
            // let current = store.id();
            // // Bounds checking:
            // if current == id { continue }
            // if store.exists(id) { store.set(id) }
            // else { continue }
            // // Handle screen switch:
            // // Disable if you are reverting back to main.
            // if id == 0 { posix::disable_alt() }
            // else {
            //     // Enable as you are on the main screen
            //     // switching to alternate.
            //     if current == 0 { posix::enable_alt() }
            //     posix::clear(Clear::All);
            // }
            // store.render();
            // // Restore settings based on metadata.
            // let (raw, mouse, show) = (
            //     store.is_raw(),
            //     store.is_mouse(),
            //     store.is_cursor() );

            // if raw { posix::raw() }
            // else { posix::cook(&initial) }
            // if mouse { posix::enable_mouse() }
            // else { posix::disable_mouse() }
            // if show { posix::show_cursor() }
            // else { posix::hide_cursor() }

            // posix::flush();
        },
        #[cfg(windows)]
        SwitchTo(id) => {
            // let current = store.id();
            // // Bounds checking:
            // if current == id { continue }
            // if store.exists(id) { store.set(id) }
            // else { continue }
            // // Handle screen switch:
            // // Disable if you are reverting back to main.
            // if id == 0 { win32::disable_alt(vte).expect("TODO") }
            // else {
            //     // Enable as you are on the main screen
            //     // switching to alternate.
            //     if vte { if current == 0 {
            //         win32::enable_alt(
            //             &screen, &initial, true).expect("TODO")
            //     }} else {
            //         // (imdaveho) NOTE: For unknown reason
            //         // that needs to be investigated TODO,
            //         // we need to activate the alternate
            //         // console handle as active every time
            //         // on Windows Console.
            //         win32::enable_alt(
            //             &screen, &initial, false).expect("TODO")
            //     }
            //     win32::clear(Clear::All, vte);
            // }
            // store.render(default, vte);
            // // Restore settings based on metadata.
            // let (raw, mouse, show) = (
            //     store.is_raw(),
            //     store.is_mouse(),
            //     store.is_cursor() );

            // if raw { win32::raw().expect("TODO") } else { win32::cook().expect("TODO") }
            // if mouse { win32::enable_mouse(vte).expect("TODO") }
            // else { win32::disable_mouse(vte).expect("TODO") }
            // if show { win32::show_cursor(vte).expect("TODO") }
            // else { win32::hide_cursor(vte).expect("TODO") }
        },

        // INTERNAL BUFFER UPDATES
        Resized => {
            #[cfg(unix)]
            let (w, h) = term.size();
            #[cfg(windows)]
            let (w, h) = match term.size() {
                Ok(s) => (s.0, s.1),
                Err(e) => {
                    (0, 0)
                }
            };
            // store.sync_size(w, h);
        },

        SyncMarker(c,r) => {}, // store.sync_marker(c,r),
        Jump => {}, // store.jump(),
        SyncTabSize(n) => {}, // store.sync_tab_size(n),
    }
}
