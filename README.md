# Github Repo Automater
`gram` is a cli to help automate common repository tasks.

## Usage
```
$ gram -h
gram 0.1.0

USAGE:
    gram --owner <owner> --repo <repo> --settings <settings>

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
