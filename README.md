# Nyano utility
A simple utility for Linux designed for simple and fast file editing.

## Installation
Use this command to clone repository and compile then install binary:
```shell
curl -fsSL https://raw.githubusercontent.com/shareui/nyano/main/nyainstall.sh | sh
```
Requires `cargo` and `git`

Also requires a C compiler (`cc`/`gcc`/`clang`) on your `PATH`, used to build the bundled `bithash` hashing library.

## Usage guide

### CLI
```shell
host@you: nyano --help
Usage: nyano [OPTIONS] <PATH>

Arguments:
<PATH>  path to the file to open

Options:
-c, --create   create the file if it does not exist
-p, --parent   create parent folders too if they do not exist (requires --create)
-r, --read     open the file in read-only mode
-b, --backup   create a backup copy after saving
-t, --tab-width <TAB_WIDTH>  number of spaces per tab [default: 4]
-h, --help     Print help
-V, --version  Print version
```

### Editor

#### Cursor movement
`Arrows` and `LMB`

#### Selection
`shift+arrows`/`mouse+LMB` - Select text  
`ctrl+a` - Select all


#### Keys
`ctrl+s` - Save changes/file  
`ctrl+c` - Copy selection  
`ctrl+x` - Cut selection  
`ctrl+v` - Paste  
`ctrl+z` - Undo  
`ctrl+y` - Redo  
`ctrl+q`/`esc` - Exit

## License
MIT License 2026
