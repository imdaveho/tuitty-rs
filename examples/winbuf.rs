extern crate tuitty_core;

use std::io::{Error, Result};
use std::thread;
use std::time::Duration;
use std::mem::zeroed;
use crate::tuitty_core::terminal::Term;
use crate::tuitty_core::common::enums::Color;
use crate::tuitty_core::system::wincon::style::into_fg;
use crate::tuitty_core::system::wincon::output::{ CHAR_INFO, COORD, SMALL_RECT };


fn main() {
    let ascii = "A";
    let cjk = "園";
    let emoji = "⚠️";
    // let emoji = "🚧";
    let ch = cjk;

    let collected = ch.encode_utf16();
    println!("collected: {:?}", collected);

    let buf_size = COORD {X: 10, Y: 1};
    let buf_coord = COORD {X: 0, Y: 0};
    let mut dest_rect = SMALL_RECT {
        Top: 0,
        Left: 0,
        Bottom: 1,
        Right: 10,
    };
    
    let mut term = Term::new().unwrap();
    let attr = term.init_data().1;

    term.enable_alt().unwrap();

    // INITIALIZE BUFFER TO BE OUTPUTTED
    let mut read_buf: Vec<CHAR_INFO> = unsafe { vec![zeroed(); 10] };
    for v in &mut read_buf {
        unsafe { *v.Char.UnicodeChar_mut() = ' ' as u16 }
        v.Attributes = term.init_data().1;
    }

    // NODIFY THE READ BUFFER DIRECTLY
    for (i, u) in collected.enumerate() {
        unsafe {
             *read_buf[i].Char.UnicodeChar_mut() = u;
             read_buf[i].Attributes = into_fg(Color::Red, attr, attr)
         }
     }

    term.writebuf(read_buf.as_ptr(), buf_size, buf_coord, &mut dest_rect).unwrap();

    let mut read_buf_contents: [u16; 10] = [0; 10];
    let mut read_buf_attribs: [u16; 10] = [0; 10];
    for (i, v) in read_buf.iter().enumerate() {
        unsafe {
            read_buf_contents[i] = *v.Char.UnicodeChar();
            read_buf_attribs[i] = v.Attributes;
        }
    }

    thread::sleep(Duration::from_millis(2000));
    term.disable_alt().unwrap();
    println!("read_buf_chars: {:?}", read_buf_contents);
    println!("read_buf_attrs: {:?}", read_buf_attribs);

}