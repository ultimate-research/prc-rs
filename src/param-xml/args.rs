use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    #[structopt(subcommand)]
    pub mode: Mode,

    #[structopt(long, short, global(true))]
    pub label: Option<String>,

    #[structopt(long, short, global(true))]
    pub out: Option<String>,
}

#[derive(StructOpt)]
pub enum Mode {
    #[structopt(about = "Convert from prc to xml")]
    Disasm { file: String },

    #[structopt(about = "Convert from xml to prc")]
    Asm { file: String },
}
