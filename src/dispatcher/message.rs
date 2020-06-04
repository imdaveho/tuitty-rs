use crate::tuitty_core::common::enums::*;

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
    // SCREEN/OUTPUT
    Clear(Clear),
    Prints(String),
    SetContent(String, i16, i16),
    Flush,
    Render,
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
    Raw,
    Cook,
    // INTERNAL BUFFER
    NewScreen,
    SwitchTo(usize),
    Resized,
    SyncMarker(i16, i16),
    Jump,
    SyncTabSize(usize),
}

pub enum Query {
    Size(usize),
    Coord(usize),
    Pos(usize),
    GetCh(usize),
    Screen(usize),
    IsRaw(usize),
}

#[derive(Clone)]
pub enum Reply {
    Size(i16, i16),
    Coord(i16, i16),
    Pos(i16, i16),
    GetCh(String),
    Screen(usize),
    IsRaw(bool),
    Empty
}
