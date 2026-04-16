# Hermes CLI (Rust)

Native Windows x64 rewrite of Hermes Agent CLI.

## Status

**Phase 1: CLI Core Foundation**

Currently implementing basic CLI structure with clap derive.

## Architecture

`
hermes-cli/
├── crates/
│   ├── cli/           # Binary entry point
│   ├── cli-core/      # CLI parsing & commands
│   └── common/        # Shared utilities
└── tests/
`

## Building

`ash
cargo build --release
`

## Commands (Implemented)

- [x] hermes --help / hermes -h
- [x] hermes --version
- [x] hermes --verbose --debug --profile <name>
- [ ] hermes chat (TUI phase)
- [ ] hermes auth {add,list,remove,reset}
- [ ] hermes model [model] [--current] [--global]
- [ ] hermes tools {list,disable,enable}
- [ ] hermes skills {search,browse,inspect,install}
- [ ] hermes gateway {run,start,stop,status,setup}
- [ ] hermes cron {list,add,remove,pause,resume}
- [ ] hermes config {show,get,set,reset}
- [ ] hermes setup
- [ ] hermes doctor
- [ ] hermes status
- [ ] hermes update
- [ ] hermes uninstall

## TODO

- [ ] Phase 2: Core Commands
- [ ] Phase 3: TUI Implementation
- [ ] Phase 4: Gateway & Integrations
- [ ] Phase 5: Advanced Features
- [ ] Phase 6: Polish & Optimization
