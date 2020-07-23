// This module provides an internal representation of the contents that
// make up the terminal screen.
use crate::tuitty_core::common::unicode::{grapheme::*, wcwidth::*};
use super::{Term, Color::{*, self}, Style, Clear};

use crate::tuitty_core::common::enums::Effect;
#[cfg(unix)]
use crate::tuitty_core::system::ansi::{style::{set_style, reset}, cursor::goto};
#[cfg(windows)]
use crate::tuitty_core::system::wincon::style::into_attr;
#[cfg(windows)]
use crate::tuitty_core::system::wincon::output::{CHAR_INFO, COORD, SMALL_RECT};


#[derive(Clone)]
pub struct Cell {
    glyph: Vec<char>,
    is_wide: bool,
    is_part: bool,
    style: (Color, Color, u32),
}


pub struct ScreenBuffer {
    cursor: usize,
    marker: usize,
    inner_buf: Vec<Option<Cell>>,
    #[cfg(unix)]
    front_buf: Vec<Option<Cell>>,
    #[cfg(windows)]
    front_buf: Vec<CHAR_INFO>,
    capacity: usize,
    window: (i16, i16),
    tab_size: usize,
    active_style: (Color, Color, u32),
    placeholder: char,
}

impl ScreenBuffer {
    pub fn new(w: i16, h: i16) -> Self {
        let capacity = (w * h) as usize;
        let inner_buf = vec![None; capacity];
        #[cfg(unix)]
        let front_buf = vec![None; capacity];
        #[cfg(windows)]
        let front_buf: Vec<CHAR_INFO> = vec![
            unsafe {std::mem::zeroed()};capacity]
            .iter_mut().map(|x: &mut CHAR_INFO| {
                unsafe { *x.Char.UnicodeChar_mut() = 32 };
                x.Attributes = 7;
                return *x
            }).collect();
            
        Self {
            cursor: 0,
            marker: 0,
            inner_buf,
            front_buf,
            capacity,
            window: (w, h),
            tab_size: 8,
            active_style: (Reset, Reset, Effect::Reset as u32),
            placeholder: '🚧',
        }
    }

    #[allow(clippy::comparison_chain)]
    fn cursor(&mut self) -> usize {
        let mut index = self.cursor;
        // Start at the origin cell.
        match self.inner_buf.get(index) {
            Some(Some(cell)) => if cell.is_part {
                // With only 2-cell wide chars,
                // if is_part is true, we shift left
                // by one to ensure cursor is pointing
                // a the origin cell.
                self.cursor -= 1;
            },
            Some(None) => (),
            None => {
                // Out-of-Bounds
                let length = self.inner_buf.len();
                // Scenario A: cell buffer length < capacity:
                if length < self.capacity {
                    // Pop from extra back into cells to get
                    // back to len == capacity.
                    let cycles = self.capacity - length;
                    for _ in 0..cycles { self.inner_buf.push(None); }
                }
                // Scenario B: cell buffer length > capacity:
                else if length > self.capacity {
                    // Pop from cells into extra to get back
                    // to len == capacity.
                    let cycles = length - self.capacity;
                    for _ in 0..cycles { self.inner_buf.pop(); }
                }
                // No issues with buffer; cursor index just out of bounds.
                // Set cursor to last Cell in buffer:
                index = self.capacity - 1;
                if let Some(cell) = &self.inner_buf[index] {
                    if cell.is_part { index -= 1; }
                }
                self.cursor = index;
            },
        }
        index
    }

    pub fn coord(&self) -> (i16, i16) {
        let index = self.cursor as i16;
        let width = self.window.0;
        ((index % width), (index / width))
    }

    // fn col(&self) -> i16 {
    //     (self.cursor as i16) % self.window.0
    // }

    fn row(&self) -> i16 {
        (self.cursor as i16) / self.window.0
    }
    
    pub fn sync_coord(&mut self, col: i16, row: i16) {
        let (mut col, mut row) = (col, row);
        if col < 0 { col = col.abs() }
        if row < 0 { row = row.abs() }
        self.cursor = ((row * self.window.0) + col) as usize;
        self.cursor();
    }

    pub fn sync_left(&mut self, n: i16) {
        let mut n = n;
        if n < 0 { n = n.abs() }
        let (mut col, row) = self.coord();
        if n >= col { col = 0 } else { col -= n }

        self.sync_coord(col, row);
    }

    pub fn sync_right(&mut self, n: i16) {
        let mut n = n;
        if n < 0 { n = n.abs() }
        if let Some(cell) = &self.inner_buf[self.cursor] {
            if cell.is_wide { n += 1 }
        }
        let (mut col, row) = self.coord();
        let width = self.width() - 1;
        if col + n >= width { col = width } else { col += n }

        self.sync_coord(col, row);
    }

    pub fn sync_up(&mut self, n: i16) {
        let mut n = n;
        if n < 0 { n = n.abs() }
        let (col, mut row) = self.coord();
        if n >= row { row = 0 } else { row -= n }

        self.sync_coord(col, row);
    }

    pub fn sync_down(&mut self, n: i16) {
        let mut n = n;
        if n < 0 { n = n.abs() }
        let (col, mut row) = self.coord();
        let height = self.height() - 1;
        if row + n >= height { row = height } else { row += n }

        self.sync_coord(col, row);
    }

    pub fn jump(&mut self) {
        let index = self.cursor;
        let marker = self.marker;
        self.cursor = marker;
        self.cursor();
        self.marker = index;
    }

    pub fn sync_marker(&mut self, col: i16, row: i16) {
        let (mut col, mut row) = (col, row);
        if col < 0 { col = col.abs() }
        if row < 0 { row = row.abs() }
        self.marker = ((row * self.window.0) + col) as usize;
    }

    pub fn size(&self) -> (i16, i16) {
        self.window
    }

    fn width(&self) -> i16 {
        self.window.0
    }

    fn height(&self) -> i16 {
        self.window.1
    }

    pub fn sync_tab_size(&mut self, n: usize) {
        self.tab_size = n;
    }

    pub fn sync_placeholder(&mut self, ch: char) {
        self.placeholder = ch;
    }

    pub fn sync_size(&mut self, w: i16, h: i16) {
        self.window = (w, h);
        self.capacity = (w * h) as usize;
        self.inner_buf.resize(self.capacity, None);
    }

    // pub fn sync_window(&mut self, w: i16, h: i16) {
    //     self.sync_size(w, h);
    // }

    pub fn getch(&self) -> String {
        let index = self.cursor;
        match &self.inner_buf[index] {
            Some(cell) => if cell.is_part {
                match &self.inner_buf[index - 1] {
                    Some(cell) => cell.glyph.iter().collect(),
                    None => " ".to_string(),
                }
            } else { cell.glyph.iter().collect() },
            None => " ".to_string(),
        }
    }

    pub fn delch(&mut self) {
        // Eg. Backspace moves cursor left 1 cell. This should have called
        // something that updated the cursor to the starting cell. Therefore
        // the index would be in-bounds and at a starting point.
        let index = self.cursor;
        match &self.inner_buf[index] {
            Some(cell) => if cell.is_part {
                // Technically, impossible to hit since self.cursor()
                // should always land on a normal cell (vs a partial one).
                // However, in the case that somehow the index is a
                // partial cell, we remove the normal cell left of it,
                // and once it is deleted, the partial cell is now in
                // index - 1 and ready for deletion as well.
                for _ in 0..2 {
                    self.inner_buf.remove(index - 1);
                    self.inner_buf.push(None);
                }
                self.cursor = index - 1;
            } else {
                // In this case, we delete the normal cell under the cursor,
                // and when the vec shifts to the left, the existing index
                // will remove the partial cell that has shifted into position.
                if cell.is_wide {
                    for _ in 0..2 {
                        self.inner_buf.remove(index);
                        self.inner_buf.push(None);
                    }
                } else {
                    self.inner_buf.remove(index);
                    self.inner_buf.push(None);
                }
            },
            None => {
                self.inner_buf.remove(index);
                self.inner_buf.push(None);
            }
        };
    }

    pub fn sync_style(&mut self, style: Style) {
        match style {
            Style::Fg(c) => self.active_style.0 = c,
            Style::Bg(c) => self.active_style.1 = c,
            Style::Fx(f) => self.active_style.2 = f,
        }
    }

    pub fn sync_styles(&mut self, f: Color, b: Color, fx: u32) {
        self.active_style = (f, b, fx);
    }

    fn set_cell(&mut self, ch: Vec<char>, is_wide: bool) {
        let mut index = self.cursor;
        if index >= self.capacity { index = self.capacity - 1 }
        if is_wide {
            self.inner_buf.remove(index);
            self.inner_buf.insert(index, Some(Cell {
                glyph: ch,
                is_wide: true,
                is_part: false,
                style: self.active_style,
            }));
            self.inner_buf.remove(index + 1);
            self.inner_buf.insert(index + 1, Some(Cell {
                glyph: vec![],
                is_wide: true,
                is_part: true,
                style: self.active_style,
            }));
            self.cursor = index + 2;
        } else {
            let mut from_wide = false;
            // If cell below is wide and new cell is single,
            // we would need to clear out the partial cell.
            if let Some(cell) = &self.inner_buf[index] {
                if cell.is_wide { from_wide = true }
            }
            self.inner_buf.remove(index);
            self.inner_buf.insert(index, Some(Cell {
                glyph: ch,
                is_wide: false,
                is_part: false,
                style: self.active_style,
            }));
            self.cursor = index + 1;
            if from_wide {
                self.inner_buf.remove(index + 1);
                self.inner_buf.insert(index + 1, None);
                // Keep the spacing from 2 cells occupied to
                // the first cell updated and the second cell blank.
                self.cursor = index + 2;
            }
        }
    }

    fn set_ascii(&mut self, s: &str) {
        match s {
            "\x00" => (),
            "\r" => self.sync_coord(0, self.row()),
            "\n" => {
                // (imdaveho) NOTE: Windows handles \n as if it were
                // \r\n on Unix systems. This is regardless of ConPTY
                // or classic (cmd.exe) consoles. (Eg. behavior is the
                // same on git-bash, powershell, and Windows Terminal)
                #[cfg(unix)] { self.sync_down(1) }
                #[cfg(windows)] {
                    let (row, height) = (self.row() + 1, self.height());
                    if height > row { self.sync_coord(0, row) }
                    else { self.sync_coord(0, height - 1) }
                }
            },
            "\r\n" => {
                let (row, height) = (self.row() + 1, self.height());
                if height > row { self.sync_coord(0, row) }
                else { self.sync_coord(0, height - 1) }
            },
            "\t" => {
                let (col, row) = self.coord();
                let prev_tab =
                    (col as usize / self.tab_size)
                    * self.tab_size;
                let mut new_tab = prev_tab + self.tab_size;
                let width = self.width() as usize - 1;
                if new_tab > width { new_tab = width }
                self.sync_coord(new_tab as i16, row)
            },
            _ => {
                let ch = if s == "\x1B" { vec!['^'] }
                else { s.chars().collect() };
                self.set_cell(ch, false)
            }
        }
    }

    pub fn sync_content(&mut self, content: &str) {
        let segments: Vec<&str> = UnicodeGraphemes
            ::graphemes(content, true).collect();

        for s in segments {
            if s.is_ascii() { self.set_ascii(s) }
            else {
                match s.width() {
                    1 => {
                        if s.contains("\u{fe0f}") {
                            self.set_cell(s.chars().collect(), true)
                        } else {
                            self.set_cell(s.chars().collect(), false)
                        }
                    },
                    2 => self.set_cell(s.chars().collect(), true),
                    // (imdaveho) NOTE: We are not going to handle complex
                    // combiner chars until there is a better way to detect
                    // how many cells is going to be taken up or until there
                    // is a consistent font / handling across terminal emulators
                    // for these complex characters.
                    // Instead, we will render all complex combiner characters
                    // with this placeholder glyph: 🚧
                    _ => self.set_cell(vec![self.placeholder,], true),
                }
            }
        }
    }

    pub fn sync_clear(&mut self, clr: Clear) {
        match clr {
            Clear::All => {
                self.inner_buf = vec![None; self.capacity];
                self.cursor = 0;
            }
            Clear::NewLn => {
                let (w, (col, row)) = (self.width(), self.coord());
                let (start, stop) = (
                    ((row * w) + col) as usize,
                    ((row + 1) * w) as usize );
                for i in start..stop { self.inner_buf[i] = None }
            }
            Clear::CurrentLn => {
                let (w, (_, row)) = (self.width(), self.coord());
                let (start, stop) = (
                    (row * w) as usize,
                    ((row + 1) * w) as usize );
                for i in start..stop { self.inner_buf[i] = None }
                self.sync_coord(0, row);
            }
            Clear::CursorUp => {
                let (w, (col, row)) = (self.width(), self.coord());
                let stop = ((row * w) + col) as usize;
                for i in 0..stop { self.inner_buf[i] = None }
            }
            Clear::CursorDn => {
                let ((w, h), (col, row)) = (self.size(), self.coord());
                let (start, stop) = (
                    ((row * w) + col) as usize,
                    (w * h) as usize );
                for i in start..stop { self.inner_buf[i] = None }
            }
        }
    }

   
    #[cfg(unix)]
    pub fn render(&mut self, term: &Term) -> std::io::Result<()> {
        let (col, row) = self.coord();
        term.goto(0, 0)?;
        let default = (Reset, Reset, Effect::Reset as u32);
        let mut style = (Reset, Reset, Effect::Reset as u32);
        let mut contents = String::with_capacity(self.capacity);
        for (i, cell) in self.inner_buf.iter().enumerate() {
            self.front_buf[i] = cell.clone();
            match cell {
                Some(c) => {
                    if c.is_part { continue }
                    // Complete reset.
                    if style != c.style && c.style == default {
                        contents.push_str(&reset());
                        style = default;
                        for ch in &c.glyph { contents.push(*ch) }
                    }
                    // Some styles are different.
                    else if style != c.style {
                        // Different Fg.
                        if style.0 != c.style.0 {
                            contents.push_str(
                                &set_style(Style::Fg(c.style.0)));
                            style.0 = c.style.0;
                        }
                        // Different Bg.
                        if style.1 != c.style.1 {
                            contents.push_str(
                                &set_style(Style::Bg(c.style.1)));
                            style.1 = c.style.1;
                        }
                        // Different Fx.
                        if style.2 != c.style.2 {
                            contents.push_str(
                                &set_style(Style::Fx(c.style.2)));
                            style.2 = c.style.2;
                        }
                        for ch in &c.glyph { contents.push(*ch) }
                    }
                    // Current style remains. Expand contents.
                    else { for ch in &c.glyph { contents.push(*ch) }}
                },
                None => {
                    // Already default style.
                    if style == default { contents.push(' ') }
                    // Reset the previous style.
                    else {
                        contents.push_str(&reset());
                        style = default;
                        contents.push(' ');
                    }
                }
            }
        }
        if !contents.is_empty() { term.prints(&contents)? }
        term.goto(col, row)?;
        term.flush()?;
        Ok(())
    }

    #[cfg(unix)]
    pub fn refresh(&mut self, term: &Term) -> std::io::Result<()> {
        let (col, row) = self.coord();
        let default = (Reset, Reset, Effect::Reset as u32);
        // TODO: let mut style = (Reset, Reset, Effect::Reset as u32);
        let mut prev = 0;
        let mut contents = String::with_capacity(self.capacity);
        for (i, cell) in self.inner_buf.iter().enumerate() {
            match cell {
                Some(c) => {
                    if c.is_part {
                        self.front_buf[i] = Some(c.clone());
                        continue
                    }

                    if let Some(f) = &self.front_buf[i] {
                        // The cells are identical. Skip.
                        if f.style == c.style && f.glyph == c.glyph {
                            self.front_buf[i] = Some(c.clone());
                            continue
                        }

                        // If you are here, it must mean a diff.
                        if i != (prev + 1) {
                            contents.push_str(&goto(
                                i as i16 % self.window.0,
                                i as i16 / self.window.0))
                        }

                        // Complete reset.
                        if f.style != c.style && c.style == default {
                            contents.push_str(&reset());
                        }
                        // Different styles between front and inner bufs.
                        else if f.style != c.style {
                            // Different Fg.
                            if f.style.0 != c.style.0 {
                                contents.push_str(
                                    &set_style(Style::Fg(c.style.0)));
                            }
                            // Different Bg.
                            if f.style.1 != c.style.1 {
                                contents.push_str(
                                    &set_style(Style::Bg(c.style.1)));
                            }
                            // Different Fx.
                            // if f.style.2 != c.style.2 {
                            //     contents.push_str(
                            //         &set_style(Style::Fx(c.style.2)));
                            // }
                        }
                        // Different chars between front and inner bufs.
                        for ch in &c.glyph { contents.push(*ch) }
                        contents.push_str(&reset());
                        prev = i;
                    }
                    // Different: frontbuf is None; innerbuf is Some(c).
                    else {
                        if i != (prev + 1) {
                            contents.push_str(&goto(
                                i as i16 % self.window.0,
                                i as i16 / self.window.0))
                        }
                        if c.style == default {
                            for ch in &c.glyph { contents.push(*ch) }
                        }
                        else {
                            // contents.push_str(&format!(
                            //     "{}{}{}",
                            //     set_style(Style::Fg(c.style.0)),
                            //     set_style(Style::Bg(c.style.1)),
                            //     set_style(Style::Fx(c.style.2))));
                            contents.push_str(&format!(
                                "{}{}",
                                set_style(Style::Fg(c.style.0)),
                                set_style(Style::Bg(c.style.1))));
                            for ch in &c.glyph { contents.push(*ch) }
                            contents.push_str(&reset());
                        }
                        prev = i;
                    }

                    self.front_buf[i] = Some(c.clone());
                },
                None => {
                    if self.front_buf[i].is_some() {
                        if i != (prev + 1) {
                            contents.push_str(&goto(
                                i as i16 % self.window.0,
                                i as i16 / self.window.0));
                            contents.push_str(&reset());
                        }
                        contents.push(' ');
                        prev = i;
                    }
                    self.front_buf[i] = None;
                }
            }
        }
        if !contents.is_empty() { term.prints(&contents)? }
        term.goto(col, row)?;
        term.flush()?;
        Ok(())
    }

    #[cfg(windows)]
    pub fn render(&mut self, term: &Term) -> std::io::Result<()> {
        let reset = term.init_data().1;
        let mut index = 0;
        for cell in &self.inner_buf {
            match cell {
                Some(c) => {
                    let (f, b, fx) = c.style;
                    let style = into_attr(f, b, fx, reset);
                    for ch in &c.glyph {
                        unsafe {
                            *self.front_buf[index]
                                .Char.UnicodeChar_mut() = *ch as u16;
                        }
                        self.front_buf[index].Attributes = style;
                        index += 1
                    }
                },
                None => {
                    unsafe {
                        *self.front_buf[index]
                            .Char.UnicodeChar_mut() = ' ' as u16;
                    }
                    self.front_buf[index].Attributes = reset;
                    index +=1
                }
            };
        }
        let (w, h) = self.size();
        let coord = COORD {X: 0, Y: 0};
        let size = COORD {X: w, Y: h};
        let mut dest = SMALL_RECT {Top: 0, Left: 0, Bottom: h, Right: w};
        term.paints(self.front_buf.as_ptr(), size, coord, &mut dest)?;
        Ok(())
    }

    #[cfg(windows)]
    pub fn refresh(&mut self, term: &Term) -> std::io::Result<()> {
        let reset = term.init_data().1;
        let width = self.window.0;
        let mut start: Option<i16> = None;
        let mut left = 0; let mut top = 0;
        let mut right = 0; let mut bottom = 0;
        let mut index = 0;
        for cell in &self.inner_buf {
            match cell {
                Some(c) => {
                    for ch in &c.glyph {
                        let ich = *ch as u16;
                        let (f, b, fx) = c.style;
                        let iattr = into_attr(f, b, fx, reset);
                        let fch = unsafe {
                            *self.front_buf[index].Char.UnicodeChar()
                        };
                        let fattr = self.front_buf[index].Attributes;
                        if (ich != fch) || (iattr != fattr) {
                            let (col, row) = (
                                (index as i16 % width), 
                                (index as i16 / width)
                            );
                            if start.is_none() { 
                                start = Some(index as i16);
                                left = col; top = row;
                            }
                            if col < left { left = col }
                            if col > right { right = col }
                            if row < top { top = row }
                            if row > bottom { bottom = row }
                            unsafe {
                                *self.front_buf[index]
                                    .Char.UnicodeChar_mut() = ich;
                            }
                            self.front_buf[index].Attributes = iattr;
                        }
                        index += 1;
                    }
                },
                None => {
                    let ich = ' ' as u16;
                    let fch = unsafe {
                        *self.front_buf[index].Char.UnicodeChar()
                    };
                    let fattr = self.front_buf[index].Attributes;
                    if (ich != fch) || (fattr != reset) {
                        let (col, row) = (
                            (index as i16 % width), 
                            (index as i16 / width)
                        );
                        if start.is_none() { 
                            start = Some(index as i16);
                            left = col; top = row;
                        }
                        if col < left { left = col }
                        if col > right { right = col }
                        if row < top { top = row }
                        if row > bottom { bottom = row }
                        unsafe {
                            *self.front_buf[index]
                                .Char.UnicodeChar_mut() = ich;
                        }
                        self.front_buf[index].Attributes = reset;
                    }
                    index += 1;
                }
            }
        }
        let size = COORD {
            X: self.window.0,
            Y: bottom + 1
        };
        let offset = COORD {
            X: left, 
            Y: top
        };
        let mut dest = SMALL_RECT {
            Top: top, Left: left, Bottom: bottom, Right: right
        };
        term.paints(self.front_buf.as_ptr(), size, offset, &mut dest)?;
        Ok(())
    }

    #[cfg(test)]
    fn check_contents(&self) -> String {
        let mut chars: Vec<char> = Vec::with_capacity(self.capacity);
        let mut length = 0;
        for c in self.inner_buf.iter() {
            match c {
                Some(cell) => {
                    if cell.is_part { continue }
                    let width = if cell.is_wide { 2 } else { 1 };
                    if length + width > self.capacity { break }
                    for c in &cell.glyph {
                        chars.push(*c)
                    }
                    length += width;
                },
                None => {
                    if length + 1 > self.capacity { break }
                    chars.push(' ');
                    length += 1;
                },
            }
        }
        while length < self.capacity {
            chars.push(' ');
            length += 1;
        }
        chars.iter().collect()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_wide_char_content() {
        let mut buffer = ScreenBuffer::new(5, 2);
        // buffer.sync_size(5, 2);
        // Check default output:
        let output = buffer.check_contents();
        assert_eq!(output, " ".repeat(10));

        // Insert wide char:
        buffer.sync_content("a㓘z");
        assert_eq!(buffer.inner_buf.len(), 10);
        let output = buffer.check_contents();
        assert_eq!(output, format!("a㓘z{}", " ".repeat(6)));
        assert_eq!(output.width(), 10);
        // Overwrite wide char:
        buffer.sync_coord(0, 0);
        buffer.sync_content("a$z");
        let output = buffer.check_contents();
        assert_eq!(output, format!("a$ z{}", " ".repeat(6)));
    }

    #[test]
    fn test_buffer_newline_content() {
        let mut buffer = ScreenBuffer::new(5, 2);
        // buffer.sync_size(5, 2);
        // Check default output:
        let output = buffer.check_contents();
        assert_eq!(output, " ".repeat(10));

        // Insert \n char:
        // NOTE: Difference between Unix and Windows \n handling.
        buffer.sync_content("a\n㓘z");
        assert_eq!(buffer.inner_buf.len(), 10);
        let output = buffer.check_contents();
        #[cfg(unix)]
        assert_eq!(output, format!(
            "a{}{}{}",
            // (1, 0) => (1, 1) = (1 * 5 + 1)
            // | a | x | x | x | x |
            // | x |   㓘  | z | x |
            " ".repeat(5),
            "㓘z",
            " ".repeat(1)));
        #[cfg(windows)]
        assert_eq!(output, format!(
            "a{}{}{}",
            // (1, 0) => (1, 1) = (1 * 5 + 1)
            // | a | x | x | x | x |
            // |   㓘  | z | x | x |
            " ".repeat(4),
            "㓘z",
            " ".repeat(2)));

        assert_eq!(output.width(), 10);

        // Overwrite \n char:
        buffer.sync_coord(0, 0);
        buffer.sync_content("a\n$z");
        let output = buffer.check_contents();
        #[cfg(unix)]
        assert_eq!(output, format!(
            "a{}{}{}",
            " ".repeat(5),
            "$ z",
            " ".repeat(1)));
        #[cfg(windows)]
        assert_eq!(output, format!(
            "a{}{}{}",
            " ".repeat(4),
            "$ z",
            " ".repeat(2)));
        // Clear cursor to new line.

        // Unix \n shifts row down. So $ is at (1, 1)
        // Since Clear::NewLn would remove "$ z  ",
        // check Clear:NewLn @ (1, 0).
        #[cfg(unix)]
        buffer.sync_coord(1, 0);

        // Windows \n is like \r\n '$' would be at (0, 1)
        // So Clear::NewLn would just remove " z  ".
        #[cfg(windows)]
        buffer.sync_coord(1, 1);

        buffer.sync_clear(Clear::NewLn);
        let output = buffer.check_contents();

        #[cfg(unix)]
        assert_eq!(output, format!("a{}$ z{}", " ".repeat(5), " ".repeat(1)));
        #[cfg(windows)]
        assert_eq!(output, format!("a{}${}", " ".repeat(4), " ".repeat(4)));

        // Clear current line
        buffer.sync_coord(1, 1);
        buffer.sync_clear(Clear::CurrentLn);
        let output = buffer.check_contents();
        assert_eq!(output, format!("a{}", " ".repeat(9)));
    }

    #[test]
    fn test_buffer_tabbed_content() {
        let mut buffer = ScreenBuffer::new(15, 2);
        // buffer.sync_size(15, 2);
        // Check default output:
        let output = buffer.check_contents();
        assert_eq!(output, " ".repeat(30));

        // Insert tabs char:
        // NOTE: when tab_size = 4;
        buffer.tab_size = 4;
        buffer.sync_content("a\t㓘\tzebra\t\t\t&");
        assert_eq!(buffer.inner_buf.len(), 30);
        let output = buffer.check_contents();
        assert_eq!(output, format!(
            "a{tab1}㓘{tab2}zebra{tab3}&{rest}",
            tab1=" ".repeat(3),
            tab2=" ".repeat(2),
            tab3=" ",
            // 1 + 3 + 2 + 2 + 5 + 1 + 1 = 15
            rest=" ".repeat(15)));
        assert_eq!(output.width(), 30);

        // NOTE: when tab_size = 8 (default on most platforms);
        buffer.tab_size = 8;
        buffer.sync_clear(Clear::All);
        buffer.sync_content("a\t㓘\tzebra\t\t\t&");
        assert_eq!(buffer.inner_buf.len(), 30);
        let output = buffer.check_contents();
        assert_eq!(output, format!(
            "a{tab1}㓘{tab2}zebra{tab3}&{rest}",
            tab1=" ".repeat(7),
            tab2=" ".repeat(4), // NOTE: why is this 4?
            // despite 3 \t chars, after zebra, there is only
            // enough space for 2 tabstops ending at capacity
            // and & would end content because of how cursor
            // and the cell buffer works (where it will continue)
            // to overwrite the last character until done. 
            tab3=" ".repeat(8 + 2),
            // 1 + 7 + 2 + 5 + 5 + 8 + 2 = 30
            rest=""));
        assert_eq!(output.width(), 30);
    }

    #[test]
    fn test_buffer_movement() {
        let mut buffer = ScreenBuffer::new(5, 5);
        // buffer.sync_size(5, 5);
        buffer.sync_content(&"-".repeat(25));
        buffer.sync_coord(2, 2);
        buffer.sync_content("0");
        buffer.sync_coord(2, 2);
        buffer.sync_up(2);
        buffer.sync_content("N");
        buffer.sync_coord(2, 2);
        buffer.sync_right(2);
        buffer.sync_content("E");
        buffer.sync_coord(2, 2);
        buffer.sync_down(2);
        buffer.sync_content("S");
        buffer.sync_coord(2, 2);
        buffer.sync_left(2);
        buffer.sync_content("W");

        // ┌───┬───┬───┬───┬───┐
        // │ 0 │ 1 │ N │ 3 │ 4 │
        // ├───┼───┼───┼───┼───┤
        // │ 5 │ 6 │ 7 │ 8 │ 9 │
        // ├───┼───┼───┼───┼───┤
        // │ W │ 11│ 0 │ 13│ E │
        // ├───┼───┼───┼───┼───┤
        // │ 15│ 16│ 17│ 18│ 19│
        // ├───┼───┼───┼───┼───┤
        // │ 20│ 21│ S │ 23│ 24│
        // └───┴───┴───┴───┴───┘

        let output = buffer.check_contents();
        assert_eq!(&output[0..3], "--N");
        assert_eq!(&output[10..13], "W-0");
        assert_eq!(&output[12..15], "0-E");
        assert_eq!(&output[20..23], "--S");
    }

    #[test]
    fn test_buffer_getch() {
        let mut buffer = ScreenBuffer::new(5, 2);
        // buffer.sync_size(5, 2);
        buffer.sync_content("He㓘o, क्‍ष");
        buffer.sync_coord(3, 0);
        assert_eq!(buffer.getch(), "㓘");
        buffer.sync_coord(0, 1);
        assert_eq!(buffer.getch(), ",");
        buffer.sync_coord(2, 1);
        assert_eq!(buffer.getch(), "क\u{94d}\u{200d}");
        buffer.sync_coord(3, 1);
        assert_eq!(buffer.getch(), "ष");
        buffer.sync_coord(4, 1);
        assert_eq!(buffer.getch(), " ");
        let output = buffer.check_contents();
        assert_eq!(output, "He㓘o, क्‍ष ");
        assert_eq!(output.width(), 10);
    }

    #[test]
    fn test_buffer_delch() {
        let mut buffer = ScreenBuffer::new(5, 2);
        // buffer.sync_size(5, 2);
        buffer.sync_content("He㓘o, क्‍ष");
        // Check contents right after entry:
        let output = buffer.check_contents();
        let length = output.len();
        // End should have single whitespace char:
        for (i, c) in output.chars().enumerate() {
            if i == length - 2 {
                assert_eq!(c, 'ष')
            }
            if i == length - 1 {
                assert_eq!(c, ' ')
            }
        }

        // Remove 㓘 with 2 or 3:
        // buffer.sync_coord(3, 0); // forcing an index on partial cell
        buffer.cursor = 3;
        assert_eq!(buffer.getch(), "㓘");
        buffer.delch();
        assert_eq!(buffer.getch(), "o");

        // Check contents after deletion:
        let output = buffer.check_contents();
        let length = output.len();
        // Should result in 2 more whitespace at the end:
        for (i, c) in output.chars().enumerate() {
            if i == length - 4 {
                assert_eq!(c, 'ष')
            }
            if length - 3 <= i && i < length {
                assert_eq!(c, ' ')
            }
        }
        buffer.sync_coord(0, 1);
        assert_eq!(buffer.getch(), "क\u{94d}\u{200d}");

        // Remove whitespace char, which appends another at end:
        buffer.sync_coord(3, 1);
        buffer.delch();
        let output = buffer.check_contents();
        assert_eq!(output, "Heo, क्‍ष   ");
        assert_eq!(output.width(), 10);
    }

    #[test]
    fn test_win_newline() {
        let mut buffer_a = ScreenBuffer::new(5, 2);
        // buffer_a.sync_size(5, 2);
        buffer_a.sync_content("a\r\n㓘z");
        let output_a = buffer_a.check_contents();

        let mut buffer_b = ScreenBuffer::new(5, 2);
        // buffer_b.sync_size(5, 2);
        buffer_b.sync_content("a\n㓘z");
        let output_b = buffer_b.check_contents();

        // NOTE: This demonstrates the difference
        // in OS specific ways of handling \n and \r\n.

        #[cfg(unix)]
        assert_ne!(output_a, output_b);

        #[cfg(windows)]
        assert_eq!(output_a, output_b);
    }

    #[test]
    fn test_win_grapheme() {
        let content_a = "a\r\n㓘z";
        let content_b = "a\n㓘z";

        let segments_a: Vec<&str> = UnicodeGraphemes
            ::graphemes(content_a, true).collect();

        let segments_b: Vec<&str> = UnicodeGraphemes
            ::graphemes(content_b, true).collect();

        assert_ne!(segments_a, segments_b);
    }

    #[test]
    fn test_complex_char_content() {
        let mut buffer = ScreenBuffer::new(5, 2);
        // buffer.sync_size(5, 2);
        // Check default output:
        let output = buffer.check_contents();
        assert_eq!(output, " ".repeat(10));

        // Insert wide char:
        buffer.sync_content("a⚠️ 👨‍👩‍👧 ❤️z");
        assert_eq!(buffer.inner_buf.len(), 10);
        let output = buffer.check_contents();
        assert_eq!(output, format!("a⚠️ 🚧 ❤️z{}", " ".repeat(0)));
        // \u{fe0f} characters are 1 cell wide...
        assert_eq!(output.width(), 8);
        // But we made it so that the character is 2 cell wide in the buffer:
        assert_eq!(buffer.inner_buf[1].as_ref().unwrap().glyph, 
                   vec!['⚠', '\u{fe0f}']);
        assert_eq!(buffer.inner_buf[1].as_ref().unwrap().is_wide, true);
        assert_eq!(buffer.inner_buf[2].as_ref().unwrap().is_part, true);
        // Overwrite wide char:
        buffer.sync_coord(0, 0);
        buffer.sync_content("a$z");
        let output = buffer.check_contents();
        assert_eq!(output, format!("a$ z🚧 ❤️z{}", " ".repeat(0)));
    }
}

