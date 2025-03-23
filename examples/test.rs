use std::path::Path;
use std::{fs, io};
use std::{fs::File, io::BufReader};

use toslib::ipf::IPFFile;
use toslib::tosreader::BinaryReader;
use toslib::xac::XACFile;
use toslib::{add, xac};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Add 7 + 4 : {}", add(7, 4));
    let file = File::open("/home/ridwan/Documents/TreeOfSaviorCN/data/bg_hi.ipf")?;
    let mut reader = BinaryReader::new(BufReader::new(file));

    // Load the IPF file
    let ipf = IPFFile::load_from_reader(&mut reader)?;
    println!("Loaded IPF file with {} entries", ipf.footer().file_count());
    extract_xac_from_ipf(
        "/home/ridwan/Documents/TreeOfSaviorCN/data/bg_hi.ipf",
        "barrack_model.xac",
    )?;
    Ok(())
}

fn extract_xac_from_ipf(ipf_path: &str, xac_filename: &str) -> io::Result<()> {
    // Check if the IPF file exists
    if !Path::new(ipf_path).exists() {
        println!("Error: IPF file '{}' not found!", ipf_path);
    }

    // Ensure output directory exists
    let output_dir = Path::new("output");
    if !output_dir.exists() {
        fs::create_dir(output_dir)?;
    }

    // Open the IPF file
    let file = File::open(ipf_path)?;
    let mut reader = BinaryReader::new(BufReader::new(file));

    // Load the IPF file
    let ipf = IPFFile::load_from_reader(&mut reader)?;
    println!("Loaded IPF file with {} entries", ipf.footer().file_count());

    let mut extracted_count = 0;

    for file_entry in ipf.file_table() {
        let filename = file_entry.directory_name();

        // Extract only the filename part (without the directory)
        let file_name_only = Path::new(&filename)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or("");

        // Check if the extracted filename matches the target
        if file_name_only == xac_filename {
            println!("\nExtracting: {}", file_name_only);
            let result = file_entry.extract(&mut reader)?;
            let mut xac_data = XACFile::load_from_bytes(result)?;

            let output_path = format!("output/{}", file_name_only.trim_end_matches(".xac"));
            xac_data.export_all_meshes(&output_path)?;

            let result = xac_data.export_all_meshes_into_struct()?;
            println!("Mesh length : {} ", result.len());

            println!("✅ Saved {} to {}", file_name_only, output_path);
            extracted_count += 1;
            break; // Stop after extracting the target file
        }
    }

    if extracted_count == 0 {
        println!(
            "No matching XAC file '{}' found in the archive.",
            xac_filename
        );
    } else {
        println!("Finished extracting {} file(s).", extracted_count);
    }

    Ok(())
}
