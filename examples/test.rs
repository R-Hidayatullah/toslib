use std::{fs::File, io::BufReader};

use toslib::add;
use toslib::ipf::IPFFile;
use toslib::tosreader::BinaryReader;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Add 7 + 4 : {}", add(7, 4));
    let file = File::open("/home/ridwan/Documents/TreeOfSaviorCN/data/bg_hi.ipf")?;
    let mut reader = BinaryReader::new(BufReader::new(file));

    // Load the IPF file
    let ipf = IPFFile::load_from_reader(&mut reader)?;
    println!("Loaded IPF file with {} entries", ipf.footer().file_count());
    Ok(())
}
