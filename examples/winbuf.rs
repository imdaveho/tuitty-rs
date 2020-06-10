extern crate tuitty_core;

use std::io::{Error, Result};
use std::thread;
use std::time::Duration;
use std::mem::zeroed;
use crate::tuitty_core::terminal::Term;
use crate::tuitty_core::common::enums::Color;
use crate::tuitty_core::system::wincon::output::{ CHAR_INFO, COORD, SMALL_RECT };


fn main() {
    let ascii = "A";
    let cjk = "園----";
    let emoji = "⚠️";
    // let emoji = "🚧";
    let ch = cjk;

    let collected = ch.encode_utf16();
    println!("collected: {:?}", collected);

    let buf_size = COORD {X: 5, Y: 1};
    let buf_offset = COORD {X: 0, Y: 0};
    let mut dest_rect = SMALL_RECT {
        Top: 0,
        Left: 0,
        Bottom: 1,
        Right: 5,
    };
    
    let mut term = Term::new().unwrap();
    let attr = term.init_data().1;

    term.enable_alt().unwrap();

    // INITIALIZE BUFFER TO BE OUTPUTTED
    let mut read_buf: Vec<CHAR_INFO> = unsafe { vec![zeroed(); 5] };
    for v in &mut read_buf {
        unsafe { *v.Char.UnicodeChar_mut() = ' ' as u16 }
        v.Attributes = attr;
    }

    // MODIFY THE READ BUFFER DIRECTLY
    for (i, u) in collected.enumerate() {
        unsafe {
             *read_buf[i].Char.UnicodeChar_mut() = u;
             read_buf[i].Attributes = 12; // 12 is FG Color Red
         }
     }
    
    // TESTING OFFSET
    // unsafe {
    //     *read_buf[3].Char.UnicodeChar_mut() = '園' as u16;
    // }
    // read_buf[3].Attributes = 12;

    term.paints(read_buf.as_ptr(), buf_size, buf_offset, &mut dest_rect).unwrap();

    let collected = emoji.encode_utf16();
    for (i, u) in collected.enumerate() {
        unsafe {
             *read_buf[i+1].Char.UnicodeChar_mut() = u;
             read_buf[i+1].Attributes = 12; // 12 is FG Color Red
         }
     }
    let buf_size = COORD {X: 5, Y: 1};
    let buf_offset = COORD {X: 1, Y: 0};
    let mut dest_rect = SMALL_RECT {
        Top: 1,
        Left: 0,
        Bottom: 1,
        Right: 5,
    };
     term.paints(read_buf.as_ptr(), buf_size, buf_offset, &mut dest_rect).unwrap();


    let mut read_buf_contents: [u16; 5] = [0; 5];
    let mut read_buf_attribs: [u16; 5] = [0; 5];
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