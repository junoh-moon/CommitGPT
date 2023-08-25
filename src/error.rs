#[derive(thiserror::Error, Debug)]
pub(crate) enum Error {
    #[error("unexpected chat completion error: `{0}`")]
    ChatCompletionBuilder(#[from] openai::chat::ChatCompletionBuilderError),

    #[error("unable to run command: `{0}`")]
    CommandError(#[from] std::io::Error),

    #[error("unable to load config: `{0}`")]
    Config(#[from] config_reader::ConfigError),

    #[error("there are no active changes, add them first to staging")]
    EmptyDiff,

    #[error("couldn't find a suitable selection")]
    EmptySelection,

    #[error("couldn't fetch data, response from openai is not okay: {0}")]
    FetchData(String),

    #[error("unable to parse to utf8: `{0}`")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    #[error("unable to run command 'git commit'")]
    GitCommit,

    #[error("unable to run command 'git diff'")]
    GitDiff,
}
