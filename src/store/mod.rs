mod buffer;
use buffer::ScreenBuffer;

use std::io::{ Result, Error, ErrorKind };
use crate::tuitty::terminal::Term;
use crate::tuitty::common::enums::{ Clear, Color, Style };


struct Screen {
    // Screen mode settings
    is_raw_enabled: bool,
    is_mouse_enabled: bool,
    is_cursor_visible: bool,
    // Screen buffer
    buffer: ScreenBuffer,
}

impl Screen {
    pub fn new(w: i16, h: i16) -> Screen {
        Screen {
            is_raw_enabled: false,
            is_mouse_enabled: false,
            is_cursor_visible: true,
            buffer: ScreenBuffer::new(w, h),
        }
    }
}


pub struct Store {
    id: usize,
    data: Vec<Screen>,
}

impl Store {
    pub fn new(w: i16, h: i16) -> Store {
        Store { id: 0, data: vec![Screen::new(w, h)] }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn exists(&self, id: usize) -> bool {
        self.data.get(id).is_some()
    }

    pub fn set(&mut self, id: usize) -> Result<()> {
        if let Some(_) = self.data.get(id) {
            self.id = id;
            return Ok(());
        }

        Err(Error::new(ErrorKind::Other,
            format!("Error: Screen ({}) does not exist", id)))
    }

    pub fn new_screen(&mut self, w: i16, h: i16) {
        self.data.push(Screen::new(w, h));
        self.id = self.data.len() - 1;
    }

    pub fn coord(&self) -> (i16, i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.coord()
    }

    pub fn size(&self) -> (i16, i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.size()
    }

    pub fn getch(&self) -> String {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.getch()
    }

    pub fn is_raw(&self) -> bool {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_raw_enabled
    }

    pub fn sync_raw(&mut self, state: bool) {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_raw_enabled = state;
    }

    pub fn is_cursor(&self) -> bool {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_cursor_visible
    }

    pub fn sync_cursor(&mut self, state: bool) {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_cursor_visible = state;
    }

    pub fn is_mouse(&self) -> bool {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_mouse_enabled
    }

    pub fn sync_mouse(&mut self, state: bool) {
        // self.set() ensures that there is a valid id
        self.data[self.id].is_mouse_enabled = state;
    }

    pub fn sync_goto(&mut self, col: i16, row: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_coord(col, row);
    }

    pub fn sync_left(&mut self, n: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_left(n);
    }

    pub fn sync_right(&mut self, n: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_right(n);
    }

    pub fn sync_up(&mut self, n: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_up(n);
    }

    pub fn sync_down(&mut self, n: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_down(n);
    }

    pub fn jump(&mut self) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.jump();
    }

    pub fn sync_marker(&mut self, col: i16, row: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_marker(col, row);
    }

    pub fn sync_size(&mut self, w: i16, h: i16) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_size(w, h);
    }

    pub fn sync_tab_size(&mut self, n: usize) {
        // TODO: include a process Command into tabs
        // to ensure that system tabs is aligned.
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_tab_size(n);
    }

    pub fn sync_content(&mut self, content: &str) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_content(content);
    }

    pub fn sync_style(&mut self, style: Style) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_style(style);
    }

    pub fn sync_styles(&mut self, f: Color, b: Color, fx: u32) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_styles(f, b, fx);
    }

    pub fn sync_clear(&mut self, clr: Clear) {
        // self.set() ensures that there is a valid id
        self.data[self.id].buffer.sync_clear(clr);
    }

    #[cfg(unix)]
    pub fn render(&self) {
        self.data[self.id].buffer.render();
    }

    #[cfg(windows)]
    pub fn render(&self, term: &Term) -> Result<()> {
        self.data[self.id].buffer.render(term)
    }
}