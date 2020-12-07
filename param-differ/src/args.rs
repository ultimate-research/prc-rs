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
    #[structopt(about = "Diff two param files")]
    Diff { a: String, b: String },

    #[structopt(about = "Patch a param file with a diff file")]
    Patch { file: String, diff: String },
}
