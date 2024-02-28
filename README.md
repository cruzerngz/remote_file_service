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
This project contains an RPC-like library ([`rfs_core`](./crates/rfs_core/)) and server/client executables.
The executables ([`rfs_server`](./crates/rfs_server/), [`rfs_client`](./crates/rfs_client/)) contain the following features:
- UDP only communications
- At-most-once invocation semantics
- At-least-once invocation semantics
- Fault tolerance

## Implementations
As the project imposes restrictions on what types of libraries can be used,
the following protocols/stuff is custom:
- arbitrary serialization/deserialization
- request/reply formats
- RPC, inspired by [`tarpc`](https://github.com/google/tarpc)
