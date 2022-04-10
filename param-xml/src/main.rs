mod args;

use args::{Args, Mode};
use clap::Parser;
use prc::hash40::Hash40;
use prc::xml::quick_xml::Error;
use prc::xml::{get_xml_error, read_xml, write_xml, ReadError};
use prc::{open, save};

use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
use std::time::Instant;

fn main() {
    let args = Args::parse();

    if let Some(label_file) = args.label {
        let label_clone = Hash40::label_map();
        let mut labels = label_clone.lock().unwrap();
        labels.add_custom_labels_from_path(label_file).unwrap();
        labels.strict = args.strict;
    }

    match args.mode {
        Mode::Asm { file } => {
            let now = Instant::now();
            if let Err(e) = to_prc(&file, args.out.as_deref().unwrap_or("out.prc")) {
                eprintln!("Error in xml-to-prc step: \n{:?}", e);
            } else {
                println!("Completed in {}", now.elapsed().as_secs_f32())
            }
        }
        Mode::Disasm { file } => {
            let now = Instant::now();
            if let Err(e) = to_xml(&file, args.out.as_deref().unwrap_or("out.xml")) {
                eprintln!("Error in prc-to-xml step: \n{:#?}", e);
            } else {
                println!("Completed in {}", now.elapsed().as_secs_f32())
            }
        }
    }
}

fn to_xml(in_path: &str, out_path: &str) -> Result<(), Error> {
    let p = open(in_path)?;
    let mut writer = BufWriter::new(File::create(out_path)?);
    write_xml(&p, &mut writer)
}

fn to_prc(in_path: &str, out_path: &str) -> Result<(), ReadError> {
    let mut file = File::open(in_path)?;
    let mut reader = BufReader::new(&file);
    match read_xml(&mut reader) {
        Ok(p) => {
            save(out_path, &p)?;
            Ok(())
        }
        Err(e) => {
            file.seek(SeekFrom::Start(0))?;
            eprint!("{}", get_xml_error(&mut file, e.start, e.end)?);
            Err(e.error)
        }
    }
}
