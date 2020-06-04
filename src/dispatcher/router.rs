use crate::tuitty_core::terminal::Term;
use crate::tuitty_core::common::enums::{
    Color::*, Effect, Clear, Style
};
use crate::store::Store;
use super::message::Action::{*, self};


// pub fn match_signal(action: Action, term: &mut Term, store: &mut Store) {
pub fn handle_action(
    action: Action, term: &mut Term, store: &mut Store
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

        // TODO: CursorPos

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
        
        // Prints(s) => {
        //     term.prints(&s)?;
        //     store.sync_content(&s);
        //     Ok(())
        // },
        
        // Printf(s) => {
        //     term.printf(&s)?;
        //     store.sync_content(&s);
        //     Ok(())
        // },

        SetContent(s, c, r) => {
            store.sync_goto(c, r);
            store.sync_content(&s);
            Ok(())
        },
 
        Flush => term.flush(),
        Render => store.render(&term),

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
