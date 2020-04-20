mod args;

use args::{Args, Mode};
use prc::{open, save};
use prc::hash40::{read_custom_labels, set_custom_labels};
use prc::xml::{write_xml, read_xml, ReadError};
use structopt::StructOpt;
use quick_xml::Error;

use std::fs::File;
use std::io::{BufWriter, BufReader};
use std::time::Instant;

fn main() {
    let args = Args::from_args();

    if let Some(label_file) = args.label {
        match read_custom_labels(label_file) {
            Ok(l) => set_custom_labels(l.into_iter()),
            Err(e) => println!("Error loading labels: {}", e),
        }
    }

    match args.mode {
        Mode::Asm { file } => {
            let now = Instant::now();
            if let Err(e) = to_prc(&file, args.out.as_deref().unwrap_or("out.prc")) {
                println!("Error in xml-to-prc step: \n{:?}", e);
            } else {
                println!("Completed in {}", now.elapsed().as_secs_f32())
            }
        }
        Mode::Disasm { file } => {
            let now = Instant::now();
            if let Err(e) = to_xml(&file, args.out.as_deref().unwrap_or("out.xml")) {
                println!("Error in prc-to-xml step: \n{:?}", e);
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
    let mut reader = BufReader::new(File::open(in_path)?);
    let p = read_xml(&mut reader)?;
    save(out_path, &p)?;
    Ok(())
}