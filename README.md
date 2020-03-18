# Github Repo Automater
`gram` is a cli to help automate common repository tasks.

# Usage

## Diff settings
`gram` supports diffing current repository settings with expected settings 
defined in a toml file. 
```
$ gram -h
    Finished dev [unoptimized + debuginfo] target(s) in 0.06s
     Running `target/debug/gram-cli -h`
gram-cli 0.1.0
Supported commands and options.

# Diff settings `gram` supports diffing known settings defined in a settings toml file against the current repository
settings. e.g.

```shell $ gram diff-settings -t ${TOKEN} -o ${OWNER} -r ${REPO} -s ${PATH_TO_TOML} ```

USAGE:
    gram-cli --token <token> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -t, --token <token>    Github token to use [env: GITHUB_TOKEN=]

SUBCOMMANDS:
    diff-settings    Diff actual settings with expected settings defined in a settings toml file
    help             Prints this message or the help of the given subcommand(s)

``` 
$ gram diff-settings -h
gram-diff-settings 0.1.0
Diff actual settings with expected settings defined in a settings.toml.

USAGE:
    gram diff-settings --owner <owner> --repo <repo> --settings <settings>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -o, --owner <owner>          The owner of the repository
    -r, --repo <repo>            The name of the repository
    -s, --settings <settings>    Path to the settings file

                                 This is a toml file. For example:
                                 -----------------------------------------
                                 description = "This is a test repository"

                                 [settings]
                                 merge.allow-squash = false 
                                 -----------------------------------------

```
