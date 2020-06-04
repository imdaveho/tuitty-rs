use tuitty_rs::dispatcher::Dispatcher;
use tuitty_rs::dispatcher::message::{*, Action::*};
use std::thread;
use std::time::Duration;
use tuitty_core::common::enums::{
    Clear as ClearKind, InputEvent::*, Color, Effect,
    MouseEvent::*, KeyEvent as Kv, MouseButton as Mb
};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};


fn main() {
    let mut dispatch = Dispatcher::init();
    // dispatch.signal(Printf("Hello, World".to_string()));
    // thread::sleep(Duration::from_millis(1000));

    let main_input = dispatch.listen();
    let background = dispatch.spawn();

    // dispatch.signal(EnableAlt);
    dispatch.signal(NewScreen);
    dispatch.signal(Raw);
    dispatch.signal(HideCursor);
    dispatch.signal(EnableMouse);

    let breaker = Arc::new(AtomicBool::new(false));
    let breakrf = breaker.clone();

    let cycle_th = thread::spawn(move || {
        let mut alpha = "abcdefghijklmnopqrstuvwxyz".chars().cycle();
        let mut corners = ["up", "right", "down", "left"].iter().cycle();

        while !breaker.load(Ordering::SeqCst) {
            background.signal(Goto(12, 12));
            let s = format!("{}", alpha.next().unwrap());
            match *corners.next().unwrap() {
                "up" => {
                    background.signal(Up(4));
                },
                "right" => {
                    background.signal(Up(4));
                    background.signal(Right(4));
                },
                "down" => {
                    background.signal(Right(4));
                },
                _ => (),
            }
            // let (x, y) = match background.request("coord") {
            //     Ok(Reply::Coord(col, row)) => (col, row),
            //     _ => (0, 0)
            // };
            let (x, y) = match background.request("pos") {
                Ok(Reply::Pos(col, row)) => (col, row),
                _ => (0, 0)
            };
            // background.signal(Prints(s));
            // background.signal(Goto(12, 14));
            // background.signal(Prints(
                // format!("col: {}, row: {}", x, y)));
            background.signal(SetContent(s, x, y));
            let p = format!("col: {}, row: {}", x, y);
            background.signal(SetContent(p, 12, 14));
            background.signal(Clear(ClearKind::NewLn));
            thread::sleep(Duration::from_millis(100));
            // background.signal(Flush);
        }
    });

    loop {
        let event = {
                if let Some(evt) = main_input.poll_latest_async() {
                    if let Msg::Received(iv) = evt { match iv {
                        Keyboard(kv) => match kv {
                            Kv::Left => "←".to_string(),
                            Kv::Right => "→".to_string(),
                            Kv::Up => "↑".to_string(),
                            Kv::Down => "↓".to_string(),
                            Kv::Esc => "Esc".to_string(),
                            Kv::Alt(c) => format!("Alt+{}", c),
                            Kv::Ctrl(c) => {
                                if c == 'g' {
                                    breakrf.store(true, Ordering::SeqCst);
                                    break
                                }
                                format!("Ctrl+{}", c)
                            },
                            Kv::Char(c) => format!("Key: ({})", c),
                            Kv::Tab => "Tab".to_string(),
                            Kv::Backspace => "Bksp".to_string(),
                            Kv::CtrlLeft => "Ctrl+←".to_string(),
                            Kv::CtrlRight => "Ctrl+→".to_string(),
                            Kv::CtrlUp => "Ctrl+↑".to_string(),
                            Kv::CtrlDown => "Ctrl+↓".to_string(),
                            Kv::ShiftLeft => "Shift+←".to_string(),
                            Kv::ShiftRight => "Shift+→".to_string(),
                            Kv::ShiftUp => "Shift+↑".to_string(),
                            Kv::ShiftDown => "Shift+↓".to_string(),
                            Kv::F(n) => format!("F: ({})", n),
                            _ => "".to_string(),
                        },
                        Mouse(mv) => match mv {
                            Press(mb, x, y) => match mb {
                                Mb::Left => format!("Left: ({}, {})", x, y),
                                Mb::Right => format!("Right: ({}, {})", x, y),
                                Mb::Middle => format!("Middle: ({}, {})", x, y),
                                Mb::WheelUp => format!("WheelUp: ({}, {})", x, y),
                                Mb::WheelDown => format!("WheelDown: ({}, {})", x, y),
                            },
                            _ => "".to_string()
                        },
                        _ => "".to_string()
                    }} else { "".to_string() }
                } else { "".to_string() }
            };

        if &event != "" {
            // main_input.signal(Goto(0, 0));
            // main_input.signal(Prints(event));
            main_input.signal(SetFg(Color::Red));
            main_input.signal(SetContent(event, 0, 0));
            main_input.signal(ResetStyles);
            main_input.signal(Clear(ClearKind::NewLn));
        }
        thread::sleep(Duration::from_millis(10));
        // main_input.signal(Flush);
        main_input.signal(Render);
    }

    let _ = cycle_th.join();

    dispatch.signal(DisableMouse);
    dispatch.signal(ShowCursor);
    dispatch.signal(Cook);
    // dispatch.signal(DisableAlt);
    dispatch.signal(SwitchTo(0));

    // thread::sleep(Duration::from_millis(1000));

    // dispatch.signal(NewScreen);
    // dispatch.signal(SetFg(Color::Yellow));
    // dispatch.signal(Goto(5, 5));
    // let sid = match main_input.request("screen") {
    //     Ok(Reply::Screen(n)) => n,
    //     _ => 0,
    // };
    // dispatch.signal(Printf(format!("Hello Screen #{}", sid)));
    // thread::sleep(Duration::from_millis(1000));
    // dispatch.signal(Switch(0));
    // thread::sleep(Duration::from_millis(1000));

    // dispatch.signal(NewScreen);
    // dispatch.signal(SetFg(Color::Cyan));
    // dispatch.signal(Goto(5, 7));
    // let sid = match main_input.request("screen") {
    //     Ok(Reply::Screen(n)) => n,
    //     _ => 0,
    // };
    // dispatch.signal(Printf(format!("Hello Screen #{}", sid)));
    // thread::sleep(Duration::from_millis(1000));
    // dispatch.signal(Switch(1));
    // thread::sleep(Duration::from_millis(1000));
    // dispatch.signal(Switch(2));
    // thread::sleep(Duration::from_millis(1000));
    // dispatch.signal(Switch(0));
    // thread::sleep(Duration::from_millis(1000));
    // dispatch.signal(Switch(2));
    // thread::sleep(Duration::from_millis(1000));


    // dispatch.signal(DisableAlt);
    // dispatch.signal(DisableMouse);
    // dispatch.signal(ShowCursor);
    // dispatch.signal(Cook);
}
