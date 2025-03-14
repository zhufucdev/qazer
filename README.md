# Qazer
Receive Tencent recruit message from a telegram bot.

## Installation

To install, you may choose either:
### a. build it with Cargo:
```shell
cargo build --release
```

### b. download CI build:
[![Rust](https://github.com/zhufucdev/qazer/actions/workflows/snapshot.yml/badge.svg)](https://github.com/zhufucdev/qazer/actions/workflows/snapshot.yml)

if you are running one of the following systems:
- Windows x64
- Linux x64
- macOS arm64

> #### Note for macOS users
> Apple enforces gatekeeper to keep unsigned binary away.
> You will have to either bypass the system or self-build.

## Setting up
Either create a bot using [@Botfather](https://t.me/botfather) or using an existing one,
and initialize environment variable `TELOXIDE_TOKEN` with the given token formatted
`123456789:blablabla`.
```shell
TELOXIDE_TOKEN="123456789:blablabla" ./qazer
```

