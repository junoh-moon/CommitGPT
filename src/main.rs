/**
 * Copyright (c) 2023 Daniél Kerkmann <daniel@kerkmann.dev>
 *
 * Licensed under the EUPL, Version 1.2 or – as soon they will be approved by
 * the European Commission - subsequent versions of the EUPL (the "Licence");
 * You may not use this work except in compliance with the Licence.
 * You may obtain a copy of the Licence at:
 *
 * https://joinup.ec.europa.eu/software/page/eupl
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the Licence is distributed on an "AS IS" basis,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the Licence for the specific language governing permissions and
 * limitations under the Licence.
 *
 * We are not affiliated, associated, authorized, endorsed by, or in any way
 * officially connected with Jura Elektroapparate AG. JURA and the JURA logo are
 * trademarks or registered trademarks of Jura Elektroapparate AG in Switzerland
 * and/or other countries.
 *
 * Using our software or hardware with you coffee machine may void your warranty
 * and we cannot be held liable for any damage or operating failure.
 */
use clap::{Parser, ValueEnum};
use config::{Config, Environment, File};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use openai::chat::{
    ChatCompletion, ChatCompletionBuilder, ChatCompletionMessage, ChatCompletionMessageRole,
};
use openai::BASE_URL;
use reqwest::StatusCode;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::Duration;
use thiserror::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(next_line_help = true)]
struct Cli {
    /// The amount of suggestions ChatGPT should generate
    #[arg(short, long, value_parser = 1..=10, default_value_t = 5)]
    suggestions: i64,
    #[arg(short, long, default_value_t = true)]
    ignore_space: bool,
    #[arg(short, long)]
    path: Option<String>,
    #[arg(short, long, value_parser = 1..=4096, default_value_t = 400)]
    tokens: i64,
    #[arg(short, long, value_enum, default_value_t = Model::Chat3X5Turbo)]
    model: Model,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Model {
    Chat3X5Turbo,
    Chat3X5Turbo0301,
}

impl ToString for Model {
    fn to_string(&self) -> String {
        match self {
            Self::Chat3X5Turbo => "gpt-3.5-turbo".to_string(),
            Self::Chat3X5Turbo0301 => "gpt-3.5-turbo-0301".to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub(crate) enum CliError {
    #[error("unexpected chat completion error: `{0}`")]
    ChatCompletionBuilder(#[from] openai::chat::ChatCompletionBuilderError),
    #[error("unable to run command: `{0}`")]
    CommandError(#[from] std::io::Error),
    #[error("unable to load config: `{0}`")]
    Config(#[from] config::ConfigError),
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
    #[error("unable to fetch data: `{0}`")]
    Reqwest(#[from] reqwest::Error),
    #[error("unable to parse json: `{0}`")]
    Serde(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct CliConfig {
    api_key: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = Cli::parse().run().await {
        match err {
            CliError::Config(_) => {
                eprintln!(
                    r#"
  _______________________________________
/ Looks like we have a MOOstake here! The \\
\\ configuration file is missing.          /
  ---------------------------------------
         \   ^__^ 
          \  (oo)\_______
             (__)\       )\/\\
                 ||----w |
                 ||     ||

tldr; missing config `~/.config/commitgpt/config.toml`
```toml
api_key = "YOUR_OPENAI_API_KEY"
```

The configuration file for CommitGPT could not be found or is invalid. The expected configuration file should be located at `~/.config/commitgpt/config.toml` for TOML file format.

The possible reasons for this error could be:

- The configuration file is not present at the expected location.
- The configuration file is not named correctly. The filename should be `config.toml` for TOML file format.
- The configuration file does not have the required `api_key` key-value pair.
- The `api_key` key-value pair is not correctly formatted. It should be in the format `api_key = "YOUR_OPENAI_API_KEY"`.
- The configuration file has a syntax error or is not valid for TOML file format.

Please ensure that the configuration file is present at the expected location and is named correctly. Also, ensure that the api_key key-value pair is present and correctly formatted.

If you have confirmed that the configuration file is present, named correctly, and has the correct key-value pair, try opening the configuration file in a text editor to check for syntax errors. You can also try validating the configuration file using a TOML validator.

You can create an API key by visiting https://platform.openai.com/account/api-keys and following the instructions provided there.

If you continue to experience issues, please feel free to reach out to me under: https://gitlab.com/kerkmann/commitgpt"#
                )
            }
            err => {
                eprintln!("{err}");
            }
        }
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

impl Cli {
    async fn run(&self) -> Result<(), CliError> {
        let api_key = self.read_config().await?.api_key;

        let diff = self.get_diff()?;
        if diff.is_empty() {
            return Err(CliError::EmptyDiff);
        }
        let response = self.get_response(api_key, diff).await?;
        let selection = response
            .clone()
            .into_iter()
            .map(|message| message.split('\n').map(str::to_owned).collect::<Vec<_>>())
            .map(|message| message.first().unwrap().clone())
            .collect::<Vec<_>>();

        loop {
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Pick commit message")
                .default(0)
                .items(&selection)
                .interact();

            match selection {
                Ok(index) => {
                    if self
                        .commit(response.get(index).ok_or(CliError::EmptySelection)?)
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                Err(_) => return Ok(()),
            };
        }
    }

    async fn read_config(&self) -> Result<CliConfig, CliError> {
        let mut settings_path = if let Ok(xdg_env) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_env)
        } else {
            let mut path = PathBuf::from(std::env!("HOME"));
            path.push(".config");
            path
        };
        settings_path.push("commitgpt/config");

        let settings = Config::builder()
            .add_source(File::with_name(settings_path.to_string_lossy().as_ref()).required(true))
            .add_source(Environment::with_prefix("OPENAI"))
            .build()?;

        let config = settings.try_deserialize::<CliConfig>()?;
        Ok(config)
    }

    fn get_diff(&self) -> Result<String, CliError> {
        let mut arguments = vec!["--no-pager", "diff", "--staged"];
        if self.ignore_space {
            arguments.push("--ignore-space-change");
            arguments.push("--ignore-blank-lines");
        }
        if let Some(ref path) = self.path {
            arguments.push(path.as_str());
        }
        let output = Command::new("git").args(&arguments).output()?;
        if !output.status.success() {
            return Err(CliError::GitDiff);
        }
        let respone = String::from_utf8(output.stdout)?;
        Ok(respone)
    }

    async fn get_response(&self, api_key: String, diff: String) -> Result<Vec<String>, CliError> {
        let pb = ProgressBar::new_spinner().with_message("🤖 Fetching responses from ChatGPT.");
        pb.enable_steady_tick(Duration::from_millis(120));

        let request = ChatCompletionBuilder::default()
            .n(self.suggestions as u8)
            .model(self.model.to_string())
            .max_tokens(self.tokens as u64)
            .messages(vec![self.get_system_message(), self.get_user_message(diff)])
            .build()?;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{BASE_URL}chat/completions"))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await?;
        if response.status() != StatusCode::OK {
            return Err(CliError::FetchData(response.text().await?))
        }
        let response = response.json::<ChatCompletion>().await?;

        let choices = response
            .choices
            .into_iter()
            .map(|choice| choice.message.content)
            .collect::<Vec<_>>();
        pb.finish();
        Ok(choices)
    }

    fn get_system_message(&self) -> ChatCompletionMessage {
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: r#"You are a helpful assistant which helps to write commit messages based on the given diff.
Follow the following git commit message convention:
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```
"#
            .to_string(),
            name: None
        }
    }

    fn get_user_message(&self, diff: String) -> ChatCompletionMessage {
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: format!(
                "```diff\n{}\n```",
                diff.chars().take(3800).collect::<String>()
            ),
            name: None,
        }
    }

    fn commit(&self, message: &str) -> Result<(), CliError> {
        let status = Command::new("git")
            .args(["commit", "--message", message, "--edit"])
            .status()?;
        if !status.success() {
            return Err(CliError::GitCommit);
        }
        Ok(())
    }
}
