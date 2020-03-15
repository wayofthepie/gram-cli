# Github Repo Automater
`gram` is a cli to help automate common repository tasks.

# Usage

## Diff settings
`gram` supports diffing current repository settings with expected settings 
defined in a toml file. 

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
