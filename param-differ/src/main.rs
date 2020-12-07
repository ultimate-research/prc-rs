mod args;

use args::{Args, Mode};
use diff::Diff;
use prc::hash40::{read_custom_labels, set_custom_labels};
use prc::{open, save};
use serde_yaml::{from_reader, to_writer};
use structopt::StructOpt;

use std::fs::File;
use std::io::{BufReader, BufWriter, Seek, SeekFrom};
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
        Mode::Diff { a, b } => {
            let now = Instant::now();
            let file_a = open(&a).unwrap();
            let file_b = open(&b).unwrap();
            let diff = file_a.diff(&file_b);
            let mut writer =
                BufWriter::new(File::create(args.out.as_deref().unwrap_or("out.yml")).unwrap());
            to_writer(&mut writer, &diff).unwrap();
            println!("Completed in {}", now.elapsed().as_secs_f32())
        }
        Mode::Patch { file, diff } => {
            todo!()
            // if let Err(e) = to_xml(&file, args.out.as_deref().unwrap_or("out.xml")) {
            //     eprintln!("Error in prc-to-xml step: \n{:#?}", e);
            // } else {
            //     println!("Completed in {}", now.elapsed().as_secs_f32())
            // }
        }
    }
}
