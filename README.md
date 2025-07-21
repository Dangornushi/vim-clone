# vim-clone

Vimと同じ動きをするエディタをAIの力で作り直す試み

<img width="190*2" height="102*2" alt="vim clone" src="https://github.com/user-attachments/assets/4dc564bd-1f93-4168-9ce1-9566834de66d" />

## Usage

```bash
cargo run -- [OPTIONS] [FILE]
```

### Options

*   `-f`, `--file <FILE>`: Open a specified file.
*   `-h`, `--help`: Print help (see a summary with '-h')
*   `-V`, `--version`: Print version information

### Subcommands

*   `new <NAME>`: Create a new file with the specified name.
*   `version`: Display version information.
