# Github Repo Automater
`gram` is a cli to help automate common repository tasks.

# Usage

## Diff settings
`gram` supports diffing current repository settings with expected settings 
defined in a toml file. 

```
$ gram -h
gram 0.1.0
Supported commands and options.

# Diff settings `gram` supports diffing known settings defined in a settings toml file against the current repository
settings.

USAGE:
    gram --token <token> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t, --token <token>    Github token to use [env: GITHUB_TOKEN=]

SUBCOMMANDS:
    diff-settings    Diff actual settings with expected settings defined in a settings toml file
    help             Prints this message or the help of the given subcommand(s)

```
