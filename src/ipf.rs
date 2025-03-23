#![allow(dead_code)]
use crate::tosreader::BinaryReader;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};

const HEADER_LOCATION: i64 = -24;
const MAGIC_NUMBER: u32 = 0x6054B50;
const CRC32_TABLE: [u32; 256] = [
    0x00000000, 0x77073096, 0xee0e612c, 0x990951ba, 0x076dc419, 0x706af48f, 0xe963a535, 0x9e6495a3,
    0x0edb8832, 0x79dcb8a4, 0xe0d5e91e, 0x97d2d988, 0x09b64c2b, 0x7eb17cbd, 0xe7b82d07, 0x90bf1d91,
    0x1db71064, 0x6ab020f2, 0xf3b97148, 0x84be41de, 0x1adad47d, 0x6ddde4eb, 0xf4d4b551, 0x83d385c7,
    0x136c9856, 0x646ba8c0, 0xfd62f97a, 0x8a65c9ec, 0x14015c4f, 0x63066cd9, 0xfa0f3d63, 0x8d080df5,
    0x3b6e20c8, 0x4c69105e, 0xd56041e4, 0xa2677172, 0x3c03e4d1, 0x4b04d447, 0xd20d85fd, 0xa50ab56b,
    0x35b5a8fa, 0x42b2986c, 0xdbbbc9d6, 0xacbcf940, 0x32d86ce3, 0x45df5c75, 0xdcd60dcf, 0xabd13d59,
    0x26d930ac, 0x51de003a, 0xc8d75180, 0xbfd06116, 0x21b4f4b5, 0x56b3c423, 0xcfba9599, 0xb8bda50f,
    0x2802b89e, 0x5f058808, 0xc60cd9b2, 0xb10be924, 0x2f6f7c87, 0x58684c11, 0xc1611dab, 0xb6662d3d,
    0x76dc4190, 0x01db7106, 0x98d220bc, 0xefd5102a, 0x71b18589, 0x06b6b51f, 0x9fbfe4a5, 0xe8b8d433,
    0x7807c9a2, 0x0f00f934, 0x9609a88e, 0xe10e9818, 0x7f6a0dbb, 0x086d3d2d, 0x91646c97, 0xe6635c01,
    0x6b6b51f4, 0x1c6c6162, 0x856530d8, 0xf262004e, 0x6c0695ed, 0x1b01a57b, 0x8208f4c1, 0xf50fc457,
    0x65b0d9c6, 0x12b7e950, 0x8bbeb8ea, 0xfcb9887c, 0x62dd1ddf, 0x15da2d49, 0x8cd37cf3, 0xfbd44c65,
    0x4db26158, 0x3ab551ce, 0xa3bc0074, 0xd4bb30e2, 0x4adfa541, 0x3dd895d7, 0xa4d1c46d, 0xd3d6f4fb,
    0x4369e96a, 0x346ed9fc, 0xad678846, 0xda60b8d0, 0x44042d73, 0x33031de5, 0xaa0a4c5f, 0xdd0d7cc9,
    0x5005713c, 0x270241aa, 0xbe0b1010, 0xc90c2086, 0x5768b525, 0x206f85b3, 0xb966d409, 0xce61e49f,
    0x5edef90e, 0x29d9c998, 0xb0d09822, 0xc7d7a8b4, 0x59b33d17, 0x2eb40d81, 0xb7bd5c3b, 0xc0ba6cad,
    0xedb88320, 0x9abfb3b6, 0x03b6e20c, 0x74b1d29a, 0xead54739, 0x9dd277af, 0x04db2615, 0x73dc1683,
    0xe3630b12, 0x94643b84, 0x0d6d6a3e, 0x7a6a5aa8, 0xe40ecf0b, 0x9309ff9d, 0x0a00ae27, 0x7d079eb1,
    0xf00f9344, 0x8708a3d2, 0x1e01f268, 0x6906c2fe, 0xf762575d, 0x806567cb, 0x196c3671, 0x6e6b06e7,
    0xfed41b76, 0x89d32be0, 0x10da7a5a, 0x67dd4acc, 0xf9b9df6f, 0x8ebeeff9, 0x17b7be43, 0x60b08ed5,
    0xd6d6a3e8, 0xa1d1937e, 0x38d8c2c4, 0x4fdff252, 0xd1bb67f1, 0xa6bc5767, 0x3fb506dd, 0x48b2364b,
    0xd80d2bda, 0xaf0a1b4c, 0x36034af6, 0x41047a60, 0xdf60efc3, 0xa867df55, 0x316e8eef, 0x4669be79,
    0xcb61b38c, 0xbc66831a, 0x256fd2a0, 0x5268e236, 0xcc0c7795, 0xbb0b4703, 0x220216b9, 0x5505262f,
    0xc5ba3bbe, 0xb2bd0b28, 0x2bb45a92, 0x5cb36a04, 0xc2d7ffa7, 0xb5d0cf31, 0x2cd99e8b, 0x5bdeae1d,
    0x9b64c2b0, 0xec63f226, 0x756aa39c, 0x026d930a, 0x9c0906a9, 0xeb0e363f, 0x72076785, 0x05005713,
    0x95bf4a82, 0xe2b87a14, 0x7bb12bae, 0x0cb61b38, 0x92d28e9b, 0xe5d5be0d, 0x7cdcefb7, 0x0bdbdf21,
    0x86d3d2d4, 0xf1d4e242, 0x68ddb3f8, 0x1fda836e, 0x81be16cd, 0xf6b9265b, 0x6fb077e1, 0x18b74777,
    0x88085ae6, 0xff0f6a70, 0x66063bca, 0x11010b5c, 0x8f659eff, 0xf862ae69, 0x616bffd3, 0x166ccf45,
    0xa00ae278, 0xd70dd2ee, 0x4e048354, 0x3903b3c2, 0xa7672661, 0xd06016f7, 0x4969474d, 0x3e6e77db,
    0xaed16a4a, 0xd9d65adc, 0x40df0b66, 0x37d83bf0, 0xa9bcae53, 0xdebb9ec5, 0x47b2cf7f, 0x30b5ffe9,
    0xbdbdf21c, 0xcabac28a, 0x53b39330, 0x24b4a3a6, 0xbad03605, 0xcdd70693, 0x54de5729, 0x23d967bf,
    0xb3667a2e, 0xc4614ab8, 0x5d681b02, 0x2a6f2b94, 0xb40bbe37, 0xc30c8ea1, 0x5a05df1b, 0x2d02ef8d,
];
const PASSWORD: [u8; 20] = [
    0x6F, 0x66, 0x4F, 0x31, 0x61, 0x30, 0x75, 0x65, 0x58, 0x41, 0x3F, 0x20, 0x5B, 0xFF, 0x73, 0x20,
    0x68, 0x20, 0x25, 0x3F,
];

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IPFFooter {
    file_count: u16,
    file_table_pointer: u32,
    footer_pointer: u32,
    magic: u32,
    version_to_patch: u32,
    new_version: u32,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IPFFileTable {
    directory_name_length: u16,
    crc32: u32,
    file_size_compressed: u32,
    file_size_uncompressed: u32,
    file_pointer: u32,
    container_name_length: u16,
    container_name: Vec<u8>,
    directory_name: Vec<u8>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct IPFFile {
    footer: IPFFooter,
    file_table: Vec<IPFFileTable>,
}

impl IPFFile {
    pub fn _load_from_file<P: AsRef<std::path::Path>>(file_path: P) -> io::Result<Self> {
        let file = File::open(file_path)?;
        let buf_reader = BufReader::new(file);
        let mut reader = BinaryReader::new(buf_reader);
        Self::load_from_reader(&mut reader)
    }

    pub fn load_from_reader<R: Read + Seek>(reader: &mut BinaryReader<R>) -> io::Result<Self> {
        let footer = Self::read_footer(reader)?;
        let file_table =
            Self::read_file_table(reader, footer.file_table_pointer, footer.file_count)?;

        Ok(IPFFile { footer, file_table })
    }

    fn read_footer<R: Read + Seek>(reader: &mut BinaryReader<R>) -> io::Result<IPFFooter> {
        let mut footer = IPFFooter::default();

        reader.seek(SeekFrom::End(HEADER_LOCATION))?;

        footer.file_count = reader.read_u16()?;
        footer.file_table_pointer = reader.read_u32()?;
        reader.read_u16()?; // Padding
        footer.footer_pointer = reader.read_u32()?;
        footer.magic = reader.read_u32()?;
        footer.version_to_patch = reader.read_u32()?;
        footer.new_version = reader.read_u32()?;

        if footer.magic != MAGIC_NUMBER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid IPF magic number: expected {:08x}, got {:08x}",
                    MAGIC_NUMBER, footer.magic
                ),
            ));
        }

        Ok(footer)
    }

    fn read_file_table<R: Read + Seek>(
        reader: &mut BinaryReader<R>,
        table_offset: u32,
        file_count: u16,
    ) -> io::Result<Vec<IPFFileTable>> {
        reader.seek(SeekFrom::Start(table_offset as u64))?;
        let mut file_table = Vec::with_capacity(file_count as usize);

        for _ in 0..file_count {
            let file_entry = Self::read_file_entry(reader)?;
            file_table.push(file_entry);
        }

        Ok(file_table)
    }

    fn read_file_entry<R: Read + Seek>(reader: &mut BinaryReader<R>) -> io::Result<IPFFileTable> {
        let directory_name_length = reader.read_u16()?;
        let crc32 = reader.read_u32()?;
        let file_size_compressed = reader.read_u32()?;
        let file_size_uncompressed = reader.read_u32()?;
        let file_pointer = reader.read_u32()?;
        let container_name_length = reader.read_u16()?;

        let container_name = reader.read_bytes_u16(container_name_length)?;
        let directory_name = reader.read_bytes_u16(directory_name_length)?;

        Ok(IPFFileTable {
            directory_name_length,
            crc32,
            file_size_compressed,
            file_size_uncompressed,
            file_pointer,
            container_name_length,
            container_name,
            directory_name,
        })
    }

    // Getter for the footer
    pub fn footer(&self) -> &IPFFooter {
        &self.footer
    }

    // Getter for the file table
    pub fn file_table(&self) -> &[IPFFileTable] {
        &self.file_table
    }

    pub fn test() -> io::Result<()> {
        // Open the file and create a buffered reader
        let file = File::open("/home/ridwan/Documents/TreeOfSaviorCN/data/xml_client.ipf")?;
        let mut reader = BinaryReader::new(BufReader::new(file));

        // Load the IPF file using the reader
        let ipf = IPFFile::load_from_reader(&mut reader)?;
        println!("Loaded IPF file with {} entries", ipf.footer.file_count);

        // Print file table entries
        for file in &ipf.file_table {
            println!("\nFile CRC32: {:08x}", file.crc32);
            println!("Container: {}", file.container_name());
            println!("Directory: {}", file.directory_name());
        }
        // Extract the first file (if available)
        if let Some(file_entry) = ipf.file_table.get(0) {
            println!("\nFilename : {}", file_entry.container_name());
            let result = file_entry.extract(&mut reader)?;
            println!("Extracted Data: {}", String::from_utf8_lossy(&result));
        } else {
            println!("No files found in the archive.");
        }

        Ok(())
    }
}

impl IPFFileTable {
    pub fn extract<R: Read + Seek>(&self, reader: &mut BinaryReader<R>) -> io::Result<Vec<u8>> {
        reader.seek(SeekFrom::Start(self.file_pointer as u64))?;

        let mut encrypted_data = reader.read_bytes(self.file_size_compressed as usize)?;

        self.decrypt(&mut encrypted_data);
        let decompressed_data = self.decompress(&encrypted_data)?;

        Ok(decompressed_data)
    }

    /// Computes the CRC32 value for a single byte using the given CRC32 table.
    fn compute_crc32(&self, crc: u32, b: u8) -> u32 {
        CRC32_TABLE[((crc ^ b as u32) & 0xFF) as usize] ^ (crc >> 8)
    }

    /// Extracts a specific byte from a 32-bit integer.
    fn extract_byte(&self, value: u32, byte_index: usize) -> u8 {
        (value >> (byte_index * 8)) as u8
    }

    /// Updates the encryption keys based on the given byte.
    fn keys_update(&self, keys: &mut [u32; 3], b: u8) {
        keys[0] = self.compute_crc32(keys[0], b);
        keys[1] = 0x8088405u32.wrapping_mul((keys[0] as u8 as u32) + keys[1]) + 1;
        keys[2] = self.compute_crc32(keys[2], self.extract_byte(keys[1], 3));
    }

    /// Generates an initial set of encryption keys based on a predefined password.
    fn keys_generate(&self) -> [u32; 3] {
        let mut keys = [0x12345678, 0x23456789, 0x34567890];

        for &byte in PASSWORD.iter() {
            self.keys_update(&mut keys, byte);
        }

        keys
    }

    fn decrypt(&self, buffer: &mut [u8]) {
        if buffer.is_empty() {
            return;
        }

        let mut keys = self.keys_generate();
        let buffer_size = (buffer.len() - 1) / 2 + 1;

        for i in 0..buffer_size {
            let v = (keys[2] & 0xFFFD) | 2;
            let idx = i * 2;
            if idx < buffer.len() {
                buffer[idx] ^= ((v.wrapping_mul(v ^ 1)) >> 8) as u8;
                self.keys_update(&mut keys, buffer[idx]);
            }
        }
    }

    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut output_data = Vec::with_capacity(self.file_size_uncompressed as usize);

        flate2::Decompress::new(false)
            .decompress_vec(data, &mut output_data, flate2::FlushDecompress::Finish)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to decompress data"))?;

        Ok(output_data)
    }

    // Getter for the directory name length
    pub fn directory_name_length(&self) -> u16 {
        self.directory_name_length
    }

    // Getter for the CRC32
    pub fn crc32(&self) -> u32 {
        self.crc32
    }

    // Getter for the compressed file size
    pub fn file_size_compressed(&self) -> u32 {
        self.file_size_compressed
    }

    // Getter for the uncompressed file size
    pub fn file_size_uncompressed(&self) -> u32 {
        self.file_size_uncompressed
    }

    // Getter for the file pointer
    pub fn file_pointer(&self) -> u32 {
        self.file_pointer
    }

    // Getter for the container name length
    pub fn container_name_length(&self) -> u16 {
        self.container_name_length
    }

    // Example getter for the file name (container name or some specific field)
    pub fn container_name(&self) -> String {
        String::from_utf8_lossy(&self.container_name).to_string()
    }

    // Example getter for the container name (if different from the directory name)
    pub fn directory_name(&self) -> String {
        String::from_utf8_lossy(&self.directory_name).to_string()
    }
}

impl IPFFooter {
    // Getter for the file count
    pub fn file_count(&self) -> u16 {
        self.file_count
    }

    // Getter for the file table pointer
    pub fn file_table_pointer(&self) -> u32 {
        self.file_table_pointer
    }

    // Getter for the footer pointer
    pub fn footer_pointer(&self) -> u32 {
        self.footer_pointer
    }

    // Getter for the magic value
    pub fn magic(&self) -> u32 {
        self.magic
    }

    // Getter for the version to patch
    pub fn version_to_patch(&self) -> u32 {
        self.version_to_patch
    }

    // Getter for the new version
    pub fn new_version(&self) -> u32 {
        self.new_version
    }
}
