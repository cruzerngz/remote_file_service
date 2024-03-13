# remote_file_service
remote file access using RPC

## Build and run
```sh
cargo b # build
cargo r --bin rfs_client # run client
cargo r --bin rfs_server # run server

cargo r --bin rfs_client -- --help # view help
cargo r --bin rfs_server -- --help # view help
```

## Overview
This project contains an RPC-like library ([`rfs`](./crates/rfs/)) and server/client executables.
The executables ([`rfs_server`](./crates/rfs_server/), [`rfs_client`](./crates/rfs_client/)) contain the following features:
- UDP only communications
- Implementations of [various messaging protocols](./crates/rfs_core/src/middleware.rs) with various levels of fault tolerance
- At-most-once invocation semantics
- At-least-once invocation semantics

## Implementations
As the project imposes restrictions on what types of libraries can be used,
the following protocols/stuff is custom:
- arbitrary serialization/deserialization
- request/reply formats
- proc-macro RPC, inspired by [`tarpc`](https://github.com/google/tarpc)
