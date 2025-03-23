#![allow(dead_code)]
use crate::tosreader::BinaryReader;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};
use std::path::Path;

const HEADER_NAME: usize = 128;
const DATA_NAME: usize = 64;

#[derive(Debug, Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
enum IESColumnType {
    Float,
    String,
    StringSecond,
}

#[derive(Default, Debug, Serialize, Deserialize)]
struct IESHeader {
    name: String,
    data_offset: u32,
    resource_offset: u32,
    file_size: u32,
    row_count: u16,
    column_count: u16,
    number_column_count: u16,
    string_column_count: u16,
}

#[derive(Debug, Serialize, Deserialize, Eq)]
struct IESColumn {
    name: String,
    name_second: String,
    column_type: IESColumnType,
    position: u16,
}

impl Default for IESColumn {
    fn default() -> Self {
        IESColumn {
            name: "".to_string(),
            name_second: "".to_string(),
            column_type: IESColumnType::Float,
            position: 0,
        }
    }
}
impl Ord for IESColumn {
    /// Implements ordering for `IESColumn` based on column type and position.
    /// This is used for sorting columns, making it easier to navigate when viewing data in tables.
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.column_type, &other.column_type) {
            (IESColumnType::Float, IESColumnType::Float)
            | (IESColumnType::String, IESColumnType::String)
            | (IESColumnType::StringSecond, IESColumnType::StringSecond) => {
                self.position.cmp(&other.position)
            }
            (IESColumnType::Float, _) => Ordering::Less,
            (_, IESColumnType::Float) => Ordering::Greater,
            (IESColumnType::String, IESColumnType::StringSecond) => Ordering::Less,
            (IESColumnType::StringSecond, IESColumnType::String) => Ordering::Greater,
        }
    }
}

impl PartialOrd for IESColumn {
    /// Implements partial ordering for `IESColumn` based on column type and position.
    /// This is used for sorting columns, making it easier to navigate when viewing data in tables.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for IESColumn {
    /// Implements equality comparison for `IESColumn` based on column type and position.
    /// This is used for sorting columns, making it easier to navigate when viewing data in tables.
    fn eq(&self, other: &Self) -> bool {
        self.column_type == other.column_type && self.position == other.position
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IESRow {
    value_float: Option<f32>,
    value_int: Option<u32>,
    value_string: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IESFile {
    header: IESHeader,
    columns: Vec<IESColumn>,
    rows: Vec<Vec<IESRow>>,
}

impl IESFile {
    pub fn load_from_file<P: AsRef<Path>>(file_path: P) -> io::Result<Self> {
        let file = std::fs::File::open(file_path)?;
        let mut buf_reader = BufReader::new(file);
        let mut binary_reader = BinaryReader::new(&mut buf_reader);
        Self::load_from_reader(&mut binary_reader)
    }

    pub fn load_from_bytes(mut bytes: Vec<u8>) -> io::Result<Self> {
        let cursor = Cursor::new(&mut bytes);
        let mut binary_reader = BinaryReader::new(cursor);
        Self::load_from_reader(&mut binary_reader)
    }

    fn load_from_reader<R: Read + Seek>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let mut ies_data = IESFile::default();
        ies_data.read_header(reader)?;
        ies_data.read_columns(reader)?;
        ies_data.read_rows(reader)?;
        Ok(ies_data)
    }

    fn read_header<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> io::Result<&mut Self> {
        let name = reader.read_bytes(HEADER_NAME)?;
        // Convert to UTF-8 and trim trailing null characters
        self.header.name = String::from_utf8_lossy(&name)
            .trim_end_matches('\0') // Trim trailing null characters
            .to_string(); // Convert to String

        reader.read_u32()?; // Padding
        self.header.data_offset = reader.read_u32()?;
        self.header.resource_offset = reader.read_u32()?;
        self.header.file_size = reader.read_u32()?;
        reader.read_u16()?; // Padding
        self.header.row_count = reader.read_u16()?;
        self.header.column_count = reader.read_u16()?;
        self.header.number_column_count = reader.read_u16()?;
        self.header.string_column_count = reader.read_u16()?;
        reader.read_u16()?; // Padding
        Ok(self)
    }

    fn read_columns<R: Read + Seek>(
        &mut self,
        reader: &mut BinaryReader<R>,
    ) -> io::Result<&mut Self> {
        reader.seek(SeekFrom::End(
            -((self.header.resource_offset as i64) + (self.header.data_offset as i64)),
        ))?;
        for _ in 0..self.header.column_count {
            let mut column = IESColumn::default();

            let name = reader.read_bytes(DATA_NAME)?;
            column.name = Self::decrypt_string(&name)?;

            let name_second = reader.read_bytes(DATA_NAME)?;
            column.name_second = Self::decrypt_string(&name_second)?;
            let num = reader.read_u16()?;
            column.column_type = match num {
                0 => IESColumnType::Float,
                1 => IESColumnType::String,
                2 => IESColumnType::StringSecond,
                _ => panic!("Invalid column type"),
            };
            reader.read_u32()?; // Padding
            column.position = reader.read_u16()?;
            self.columns.push(column);
        }
        self.columns.sort();
        Ok(self)
    }

    fn read_rows<R: Read + Seek>(&mut self, reader: &mut BinaryReader<R>) -> io::Result<&mut Self> {
        reader.seek(SeekFrom::End(-(self.header.resource_offset as i64)))?;

        for _ in 0..self.header.row_count {
            reader.read_u32()?; // Padding

            let count = reader.read_u16()?;
            let _buffer = reader.read_bytes(count as usize)?;
            let mut row = Vec::with_capacity(self.header.row_count as usize);

            for (_, column) in self.columns.iter().enumerate() {
                let value = if column.column_type == IESColumnType::Float {
                    let nan = reader.read_f32()?;
                    let max_value = f32::from_bits(u32::MAX);
                    if (nan - max_value).abs() < f32::EPSILON {
                        IESRow {
                            value_float: Some(max_value),
                            value_int: None,
                            value_string: None,
                        }
                    } else {
                        IESRow {
                            value_float: None,
                            value_int: Some(nan as u32),
                            value_string: None,
                        }
                    }
                } else {
                    let length = reader.read_u16()?;
                    let string_buffer = reader.read_bytes(length as usize)?;
                    let string_value = Self::decrypt_string(&string_buffer)?;
                    if !string_value.is_empty() {
                        IESRow {
                            value_float: None,
                            value_int: None,
                            value_string: Some(string_value),
                        }
                    } else {
                        IESRow {
                            value_float: None,
                            value_int: None,
                            value_string: None,
                        }
                    }
                };
                row.push(value);
            }

            self.rows.push(row);
            reader.seek(SeekFrom::Current(self.header.string_column_count as i64))?;
        }
        Ok(self)
    }

    /// Decrypts a byte array using a simple XOR operation.
    /// The function applIES a XOR operation using a predefined key (xor_key = 1) to each byte in the input data array.
    /// The decrypted byte array is then converted into a UTF-8 string, removing trailing null characters ('\u{1}'),
    /// and returning the resulting string.
    fn decrypt_string(data: &[u8]) -> io::Result<String> {
        let xor_key = 1;

        // Apply XOR operation to each byte in the input data array to decrypt it.
        let decrypted_data: Vec<u8> = data.iter().map(|&byte| byte ^ xor_key).collect();

        // Convert the decrypted byte array into a UTF-8 string.
        // Trim trailing null characters ('\u{1}') and return the resulting string.
        Ok(String::from_utf8(decrypted_data)
            .unwrap()
            .trim_end_matches('\u{1}')
            .to_string())
    }

    pub fn get_columns_length(&self) -> io::Result<usize> {
        Ok(self.columns.len())
    }
    pub fn get_rows_length(&self) -> io::Result<usize> {
        Ok(self.rows.len())
    }

    pub fn get_data_by_column_name_and_index(
        &self,
        column_name: &str,
        row_index: usize,
    ) -> Option<&IESRow> {
        if let Some(column_index) = self.get_column_index_by_name(column_name) {
            if row_index < self.rows.len() {
                Some(&self.rows[row_index][column_index])
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_column_index_by_name(&self, column_name: &str) -> Option<usize> {
        if let Some(index) = self.columns.iter().position(|col| col.name == column_name) {
            Some(index)
        } else {
            self.columns
                .iter()
                .position(|col| col.name_second == column_name)
        }
    }

    pub fn get_column_names(&self) -> Vec<&String> {
        self.columns.iter().map(|col| &col.name).collect()
    }
}
