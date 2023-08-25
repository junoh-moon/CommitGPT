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
use std::{
    process::{Command, ExitCode},
    time::Duration,
};

use clap::{Parser, ValueEnum};
use dialoguer::{theme::ColorfulTheme, Select};
use indicatif::ProgressBar;
use openai::chat::{ChatCompletionBuilder, ChatCompletionMessage, ChatCompletionMessageRole};
use serde::Deserialize;

mod args;
mod config;
mod error;

use args::*;
use config::*;
use error::*;

#[derive(Default, Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(crate) enum Model {
    #[default]
    #[serde(alias = "gpt-3.5-turbo")]
    #[value(name = "gpt-3.5-turbo")]
    GPT3X5Turbo,

    #[serde(alias = "gpt-3.5-turbo-0301")]
    #[value(name = "gpt-3.5-turbo-0301")]
    GPT3X5Turbo0301,

    #[serde(alias = "gpt-4")]
    #[value(name = "gpt-4")]
    GPT4,
}

impl ToString for Model {
    fn to_string(&self) -> String {
        match self {
            Self::GPT3X5Turbo => "gpt-3.5-turbo".to_string(),
            Self::GPT3X5Turbo0301 => "gpt-3.5-turbo-0301".to_string(),
            Self::GPT4 => "gpt-4".to_string(),
        }
    }
}

fn git_preflight_check() -> Result<(), ExitCode> {
    let git_command_exists = match Command::new("git").arg("status").status() {
        Ok(status) => status.success(),
        Err(_) => false,
    };
    if !git_command_exists {
        eprintln!("Git is not installed or you are not in a git repository.");
        return Err(ExitCode::FAILURE);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(code) = git_preflight_check() {
        return code;
    }

    let config = match read_config().await {
        Ok(config) => config,
        Err(_) => {
            eprintln!(
                r#"
  _______________________________________
/ Looks like we have a MOOstake here! The \
\ configuration file is missing.          /
  ---------------------------------------
         \   ^__^ 
          \  (oo)\_______
             (__)\       )\/\\
                 ||----w |
                 ||     ||

tldr; missing config `~/.config/commitgpt/config.toml`
```toml
# (required) Your API key from https://platform.openai.com/account/api-keys
api_key = "YOUR_OPENAI_API_KEY"

# (optional) The given context to let ChatGPT know what he should do with the git diff
context_prefix = """{}"""

# (optional) The amount of suggestions ChatGPT should generate
suggestions = {}

# (optional) Ignore space change and blank lines in the git diff
ignore_space = {}

# (optional) The maximum amount of token which should be used for ChatGPT
max_tokens = {}

# (optional) The model which should be used for ChatGPT
model = "{}"
```

The configuration file for CommitGPT could not be found or is invalid. The expected configuration file should be located at `~/.config/commitgpt/config.toml` in TOML file format.

The possible reasons for this error could be:

- The configuration file is not present at the expected location.
- The configuration file is not named correctly. The filename should be `config.toml` for TOML file format.
- The configuration file does not have the required `api_key` key-value pair.
- The `api_key` key-value pair is not correctly formatted. It should be in the format `api_key = "YOUR_OPENAI_API_KEY"`.
- The configuration file has a syntax error or is not valid for TOML file format.

Please ensure that the configuration file is present at the expected location and is named correctly. Also, ensure that the api_key key-value pair is present and correctly formatted.

If you have confirmed that the configuration file is present, named correctly, and has the correct key-value pair, try opening the configuration file in a text editor to check for syntax errors.
You can also try validating the configuration file using a TOML validator.

You can create an API key by visiting https://platform.openai.com/account/api-keys and following the instructions provided there.

If you continue to experience issues, please feel free to reach out to me under: https://gitlab.com/kerkmann/commitgpt"#,
                default_context_prefix(),
                default_suggestions(),
                default_ignore_space(),
                default_tokens(),
                Model::default().to_string(),
            );
            return ExitCode::FAILURE;
        }
    };
    let args = Args::parse();

    if let Err(err) = Cli::new(config, args).run().await {
        match err {
            Error::Config(_) => {}
            err => {
                eprintln!("{err}");
            }
        }
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

struct Cli {
    config: Config,
    args: Args,
}

impl Cli {
    fn new(config: Config, args: Args) -> Self {
        Self { config, args }
    }

    async fn run(&self) -> Result<(), Error> {
        openai::set_key(self.config.api_key.clone());

        let diff = self.get_git_diff()?;
        if diff.is_empty() {
            return Err(Error::EmptyDiff);
        }

        let response = self.get_response(diff).await?;
        let selection = response
            .clone()
            .into_iter()
            .map(|message| message.split('\n').map(str::to_owned).collect::<Vec<_>>())
            .filter_map(|message| message.first().cloned())
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
                        .commit(response.get(index).ok_or(Error::EmptySelection)?)
                        .is_ok()
                    {
                        return Ok(());
                    }
                }
                Err(_) => return Ok(()),
            };
        }
    }

    fn get_git_diff(&self) -> Result<String, Error> {
        let mut arguments = vec!["--no-pager", "diff", "--staged"];
        if self.args.ignore_space.unwrap_or(self.config.ignore_space) {
            arguments.push("--ignore-space-change");
            arguments.push("--ignore-blank-lines");
        }
        for path in &self.args.path {
            arguments.push(path.as_str());
        }
        let output = Command::new("git").args(&arguments).output()?;
        if !output.status.success() {
            return Err(Error::GitDiff);
        }
        let respone = String::from_utf8(output.stdout)?;
        Ok(respone)
    }

    async fn get_response(&self, diff: String) -> Result<Vec<String>, Error> {
        let progress_bar =
            ProgressBar::new_spinner().with_message("🤖 Fetching responses from ChatGPT.");
        progress_bar.enable_steady_tick(Duration::from_millis(120));

        let response = ChatCompletionBuilder::default()
            .n(self
                .args
                .suggestions
                .map(|suggestions| suggestions as u8)
                .unwrap_or(self.config.suggestions))
            .model(self.args.model.unwrap_or(self.config.model).to_string())
            .max_tokens(
                self.args
                    .max_tokens
                    .map(|suggestions| suggestions as u64)
                    .unwrap_or(self.config.max_tokens),
            )
            .messages(vec![
                self.get_system_message(self.config.context_prefix.clone()),
                self.get_user_message(diff),
            ])
            .create()
            .await
            .map_err(|error| Error::FetchData(error.message))?;

        let choices = response
            .choices
            .into_iter()
            .map(|choice| {
                choice
                    .message
                    .content
                    .expect("expect content data from ChatGPT")
            })
            .collect::<Vec<_>>();
        progress_bar.finish();
        Ok(choices)
    }

    fn get_system_message(&self, context_prefix: String) -> ChatCompletionMessage {
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::System,
            content: Some(context_prefix),
            name: None,
            function_call: None,
        }
    }

    fn get_user_message(&self, diff: String) -> ChatCompletionMessage {
        ChatCompletionMessage {
            role: ChatCompletionMessageRole::User,
            content: Some(format!(
                r#"
Why: {}
What: ```diff
{}
```
"#,
                self.args.reason,
                diff.chars().take(3800).collect::<String>()
            )),
            name: None,
            function_call: None,
        }
    }

    fn commit(&self, message: &str) -> Result<(), Error> {
        let status = Command::new("git")
            .args(["commit", "--message", message, "--edit"])
            .status()?;
        if !status.success() {
            return Err(Error::GitCommit);
        }
        Ok(())
    }
}
