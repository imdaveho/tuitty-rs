use crate::tuitty::common::enums::*;

#[derive(Clone)]
pub enum Msg {
    Received(InputEvent),
    Response(Reply),
    Unsupported,
}

pub enum Cmd {
    Continue,
    Suspend(usize),
    Transmit(usize),
    Stop(usize),
    Lock(usize),
    Unlock,
    Signal(Action),
    Request(Query)
}

pub enum Action {
    // CURSOR
    Goto(i16, i16),
    Up(i16),
    Down(i16),
    Left(i16),
    Right(i16),
    // TODO: CursorPos
    // SCREEN/OUTPUT
    Clear(Clear),
    Prints(String),
    Printf(String),
    Flush,
    Resize(i16, i16),
    // STYLE
    SetFx(u32),
    SetFg(Color),
    SetBg(Color),
    SetStyles(Color, Color, u32),
    ResetStyles,
    // STATEFUL/MODES
    HideCursor,
    ShowCursor,
    EnableMouse,
    DisableMouse,
    EnableAlt,
    DisableAlt,
    Raw,
    Cook,
    // INTERNAL BUFFER
    Switch,
    SwitchTo(usize),
    Resized,
    SyncMarker(i16, i16),
    Jump,
    SyncTabSize(usize),
}

pub enum Query {
    Size(usize),
    Coord(usize),
    GetCh(usize),
    Screen(usize),
}

#[derive(Clone)]
pub enum Reply {
    Size(i16, i16),
    Coord(i16, i16),
    GetCh(String),
    Screen(usize),
    Empty
}