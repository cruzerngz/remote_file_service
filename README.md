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


## Common errors

### Incorrect dispatch routing
There may be times when dispatch matches the method signature for a method early
and routes the payload to the wrong method handler.
This can happen if any method signature is a prefix of another.

> [!example]
> The first method signature is a prefix of the second:
>
> SomeInterface::method
> SomeInterface::method_b

If this occurs, check that the [signature collision unit test](./crates/rfs/src/interfaces.rs) passes.
