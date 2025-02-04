// Copyright 2022 Datafuse Labs.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::io::BufRead;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::Read;
use std::io::Result;

use crate::cursor_ext::cursor_read_bytes_ext::ReadBytesExt;

pub trait BufferReadStringExt {
    fn read_quoted_text(&mut self, buf: &mut Vec<u8>, quota: u8) -> Result<()>;
    fn read_escaped_string_text(&mut self, buf: &mut Vec<u8>) -> Result<()>;
}

impl<T> BufferReadStringExt for Cursor<T>
where T: AsRef<[u8]>
{
    fn read_quoted_text(&mut self, buf: &mut Vec<u8>, quota: u8) -> Result<()> {
        self.must_ignore_byte(quota)?;

        loop {
            self.keep_read(buf, |b| b != quota && b != b'\\');
            if self.ignore_byte(quota) {
                return Ok(());
            } else if self.ignore_byte(b'\\') {
                let b = self.remaining_slice();
                if b.is_empty() {
                    return Err(std::io::Error::new(
                        ErrorKind::InvalidData,
                        "Expected to have terminated string literal after escaped char '\' ."
                            .to_string(),
                    ));
                }
                let c = b[0];
                self.ignore_byte(c);

                match c {
                    b'n' => buf.push(b'\n'),
                    b't' => buf.push(b'\t'),
                    b'r' => buf.push(b'\r'),
                    b'0' => buf.push(b'\0'),
                    b'\'' => buf.push(b'\''),
                    b'\\' => buf.push(b'\\'),
                    b'\"' => buf.push(b'\"'),
                    _ => {
                        buf.push(b'\\');
                        buf.push(c);
                    }
                }
            } else {
                break;
            }
        }
        Err(std::io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "Expected to have terminated string literal after quota {:?}, while consumed buf: {:?}",
                quota as char, buf
            ),
        ))
    }

    fn read_escaped_string_text(&mut self, buf: &mut Vec<u8>) -> Result<()> {
        loop {
            self.keep_read(buf, |f| f != b'\t' && f != b'\n' && f != b'\\');
            if self.ignore_byte(b'\\') {
                let buffer = self.remaining_slice();
                let c = buffer[0];
                match c {
                    b'\'' | b'\"' | b'\\' | b'/' | b'`' => {
                        buf.push(c);
                        self.consume(1);
                    }
                    b'N' => {
                        self.consume(1);
                    }
                    b'x' => {
                        self.consume(1);
                        let mut b = [0u8; 2];
                        self.read_exact(&mut b[..])?;
                        let high = hex_char_to_digit(b[0]);
                        let low = hex_char_to_digit(b[1]);
                        let c = high * 0x10 + low;
                        buf.push(c);
                    }
                    _ => {
                        let e = unescape(c);
                        if !is_control_ascii(e) {
                            buf.push(b'\\');
                        }
                        buf.push(e);
                        self.consume(1);
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }
}

fn unescape(c: u8) -> u8 {
    match c {
        b'a' => b'\x07', // \a in c
        b'b' => b'\x08', // \b in c
        b'v' => b'\x0B', // \v in c
        b'f' => b'\x0C', // \e in c
        b'e' => b'\x1B', // \e in c
        b'n' => b'\n',
        b'r' => b'\r',
        b't' => b'\t',
        b'0' => b'\0',
        _ => c,
    }
}

#[inline]
fn is_control_ascii(c: u8) -> bool {
    c <= 31
}

#[inline]
fn hex_char_to_digit(c: u8) -> u8 {
    match c {
        b'A'..=b'F' => c - b'A' + 10,
        b'a'..=b'f' => c - b'a' + 10,
        b'0'..=b'9' => c - b'0',
        _ => 0xff,
    }
}
