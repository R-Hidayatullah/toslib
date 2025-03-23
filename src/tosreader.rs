#![allow(dead_code)]

use byteorder::{LittleEndian, ReadBytesExt};
use std::io::{self, Read, Seek, SeekFrom};

pub struct BinaryReader<R: Read + Seek> {
    pub reader: R,
}

impl<R: Read + Seek> BinaryReader<R> {
    /// Creates a new `BinaryReader` instance.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_u8(&mut self) -> io::Result<u8> {
        self.reader.read_u8()
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        self.reader.read_i32::<LittleEndian>()
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        self.reader.read_u32::<LittleEndian>()
    }

    pub fn read_u16(&mut self) -> io::Result<u16> {
        self.reader.read_u16::<LittleEndian>()
    }

    pub fn read_f32(&mut self) -> io::Result<f32> {
        self.reader.read_f32::<LittleEndian>()
    }

    pub fn read_bytes(&mut self, size: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; size];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Reads a specified number of bytes into a `Vec<u8>`.
    pub fn read_bytes_u16(&mut self, length: u16) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; length as usize];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Reads a null-terminated string and converts it to a `String`.
    pub fn parse_string(raw_bytes: &[u8]) -> String {
        raw_bytes
            .split(|&b| b == 0)
            .next()
            .map_or_else(String::new, |s| String::from_utf8_lossy(s).into_owned())
    }

    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<()> {
        self.reader.seek(pos)?;
        Ok(())
    }

    pub fn tell(&mut self) -> io::Result<u64> {
        self.reader.seek(SeekFrom::Current(0))
    }

    pub fn file_size(&mut self) -> io::Result<u64> {
        let current_position = self.tell()?;
        let result = self.reader.seek(SeekFrom::End(0))?;
        self.reader.seek(SeekFrom::Start(current_position))?;
        Ok(result)
    }

    pub fn is_eof(&mut self) -> io::Result<bool> {
        let mut buffer = [0u8; 1];
        let bytes_read = self.reader.read(&mut buffer)?;
        if bytes_read == 0 {
            Ok(true) // EOF reached
        } else {
            self.reader.seek(SeekFrom::Current(-1))?; // Move back one byte
            Ok(false) // Not EOF
        }
    }

    /// Reads a fixed-size array of 16 `f32` values.
    pub fn read_f32_array_16(&mut self) -> io::Result<[f32; 16]> {
        let mut buffer = [0.0_f32; 16];
        for i in 0..16 {
            buffer[i] = self.read_f32()?;
        }
        Ok(buffer)
    }

    /// Reads a fixed-size array of 3 `u8` values.
    pub fn read_u8_array_3(&mut self) -> io::Result<[u8; 3]> {
        let mut buffer = [0u8; 3];
        self.reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Skips `n` bytes, can move both forward and backward.
    pub fn skip_bytes(&mut self, n: i64) -> io::Result<()> {
        // Use Seek to move forward or backward by `n` bytes
        self.reader.seek(SeekFrom::Current(n))?;
        Ok(())
    }
}
