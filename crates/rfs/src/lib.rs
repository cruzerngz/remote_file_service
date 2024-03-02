//! Remote methods, data structures between server and client are defined here.

pub mod fs;
pub mod interfaces;
mod callback;

pub use rfs_core::{
    middleware, payload_handler, RemoteMethodSignature, RemoteRequest, RemotelyInvocable,
};

/// Default constants used between a client and the remote.
pub mod defaults {
    /// The default port used by the remote
    pub const DEFAULT_PORT: u16 = 4013;
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;

    use interfaces::*;
    use rfs_core::RemotelyInvocable;

    /// Test the fully integrated ser/de of the payload of a remote invocation.
    // #[test]
    fn test_remote_serde() {
        type X = ImmutableFileOpsClient;
        let x = ImmutableFileOpsClient::read_file(todo!(), todo!(), todo!());

        let message = ImmutableFileOpsReadFile::Request {
            path: Default::default(),
            offset: None,
        };

        let ser = message.invoke_bytes();

        println!("{:?}", ser);

        let des = ImmutableFileOpsReadFile::process_invocation(&ser).unwrap();

        println!("{:?}", des);

        // let mut x = std::fs::File::create("serialized").unwrap();
        // x.write_all(&ser).unwrap();
    }
}
