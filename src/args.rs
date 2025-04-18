use clap::{command, Parser};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
pub(crate) struct Args {
    /// The amount of suggestions ChatGPT should generate
    #[arg(short, long, value_parser = 1..=10)]
    pub(crate) suggestions: Option<i64>,

    /// Ignore space change and blank lines in the git diff
    #[arg(short, long)]
    pub(crate) ignore_space: Option<bool>,

    /// The maximum amount of token which should be used for ChatGPT
    #[arg(short = 't', long, value_parser = 1..=128000)]
    pub(crate) max_tokens: Option<i64>,

    /// The model which should be used for ChatGPT
    #[arg(short, long)]
    pub(crate) model: Option<String>,

    /// The files which should be transmitted as diff, otherwise all files till be transmited
    pub(crate) path: Vec<String>,
}
