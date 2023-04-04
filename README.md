#CommitGPT: A ChatGPT-powered Commit Message Generator

This repository contains the source code for CommitGPT, a tool that helps you create commit messages using the OpenAI ChatGPT API. It uses the GPT-4 architecture and provides a simple CLI interface for generating commit messages based on the staged changes in a Git repository.

## Features

- Generate multiple suggestions for commit messages based on staged changes in the repository.
- Choose from different ChatGPT models.
- Customize the number of suggestions, tokens, and other options.
- Interactive selection of the generated commit messages.

## Prerequisites

To use CommitGPT, you need:

- An OpenAI API key. You can create one by visiting [https://platform.openai.com/account/api-keys](https://platform.openai.com/account/api-keys).
- Rust programming language installed on your system.

## Installation

1. Install from crates.io

```
cargo install commitgpt
```

2. Create the configuration file:
    
```bash
mkdir -p ~/.config/commitgpt
touch ~/.config/commitgpt/config.toml
```

3. Add your OpenAI API key to the configuration file:

```bash
echo 'api_key = "YOUR_OPENAI_API_KEY"' > ~/.config/commitgpt/config.toml
```

## Usage

1. Stage your changes in a Git repository using `git add`.
2. Run CommitGPT to generate commit message suggestions:

```bash
codecommitgpt
```

3. Pick a commit message from the generated suggestions, or exit the selection prompt to cancel.
4. Optionally, edit the commit message and save to complete the commit process.

## Customization

You can customize the behavior of CommitGPT using command-line options. For example, to generate 7 commit message suggestions and limit the message length to 300 tokens, run:

```bash
commitgpt -s 7 -t 300
```

For more options, run `commitgpt --help`.

## Contributing

Please feel free to submit issues and pull requests on GitLab: https://gitlab.com/kerkmann/commitgpt

#### Trademarks

I am not affiliated, associated, authorized, endorsed by, or in any way officially connected with OpenAi, ChatGPT and the OpenAi are trademarks or registered trademarks of OpenAi in San Francisco, CA and/or other countries.

## License

Licensed under the EUPL, Version 1.2 or – as soon they will be approved by the European Commission - subsequent versions of the EUPL (the "Licence"); \
You may not use this work except in compliance with the Licence. \
You may obtain a copy of the Licence at:

[https://joinup.ec.europa.eu/software/page/eupl](https://joinup.ec.europa.eu/software/page/eupl)

Unless required by applicable law or agreed to in writing, software distributed under the Licence is distributed on an "AS IS" basis, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. \
See the Licence for the specific language governing permissions and limitations under the Licence.

