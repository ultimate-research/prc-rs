use clap::Parser;

#[derive(Parser)]
pub struct Args {
    #[clap(subcommand)]
    pub mode: Mode,

    #[clap(long, short, global(true))]
    pub label: Option<String>,

    #[clap(
        long,
        short,
        global(true),
        requires("label"),
        help = "Whether to fail if a label does not have a corresponding hash. \
                Useful to catch spelling errors. By default, the program uses the \
                default hash40 algorithm to generate hashes for unmatched labels"
    )]
    pub strict: bool,

    #[clap(long, short, global(true), help = "The file to output the result to")]
    pub out: Option<String>,
}

#[derive(Parser)]
pub enum Mode {
    #[clap(about = "Convert from prc to xml")]
    Disasm { file: String },

    #[clap(about = "Convert from xml to prc")]
    Asm { file: String },
}
