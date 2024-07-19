use std::path::PathBuf;

use serde::Deserialize;
use serde_valid::Validate;

#[derive(Deserialize, Validate)]
pub(crate) struct Config {
    /// Your API key from https://platform.openai.com/account/api-keys
    pub(crate) api_key: String,

    /// The given context to let ChatGPT know what he should do with the git diff
    #[serde(default = "default_context_prefix")]
    pub(crate) context_prefix: String,

    /// The amount of suggestions ChatGPT should generate
    #[validate(minimum = 1)]
    #[validate(maximum = 10)]
    #[serde(default = "default_suggestions")]
    pub(crate) suggestions: u8,

    /// Ignore space change and blank lines in the git diff
    #[serde(default = "default_ignore_space")]
    pub(crate) ignore_space: bool,

    /// The maximum amount of token which should be used for ChatGPT
    #[validate(minimum = 1)]
    #[validate(maximum = 128000)]
    #[serde(default = "default_tokens")]
    pub(crate) max_tokens: u64,

    /// The model which should be used for ChatGPT
    #[serde(default)]
    pub(crate) model: super::Model,
}

pub(crate) fn default_suggestions() -> u8 {
    5
}

pub(crate) fn default_ignore_space() -> bool {
    true
}

pub(crate) fn default_tokens() -> u64 {
    400
}

pub(crate) fn default_context_prefix() -> String {
    r#"You are a helpful assistant which helps to write commit messages based on the given diff and reason.
The first line is explaining why there are specific changes and the other lines describes what have been changed.
Follow the following git commit message convention:
<type>: <description>

<why>

Changes:
<what>"#
        .to_string()
}

pub(crate) async fn read_config() -> Result<Config, crate::Error> {
    let mut settings_path = if let Ok(xdg_env) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_env)
    } else {
        let mut path = PathBuf::from(std::env!("HOME"));
        path.push(".config");
        path
    };
    settings_path.push("commitgpt/config");

    let settings = config_reader::Config::builder()
        .add_source(
            config_reader::File::with_name(settings_path.to_string_lossy().as_ref()).required(true),
        )
        .add_source(config_reader::Environment::with_prefix("OPENAI"))
        .build()?;

    let config = settings.try_deserialize::<Config>()?;
    Ok(config)
}
