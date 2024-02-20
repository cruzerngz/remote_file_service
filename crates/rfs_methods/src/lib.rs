//! Remote methods, data structures between server and client are defined here.

use std::path::PathBuf;

use rfs_core::remote_interface;

use rfs_core::RemoteMethodSignature;

/// Immutable file operations are defined in this interface.
#[remote_interface]
pub trait ImmutableFileOps {
    /// Read the contents of a file.
    async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8>;

    // this is implemented by remote-interface
    // async fn read_file_payload(payload: ImmutableFileOpsReadFile) -> Vec<u8> {
    //      Self::read_file(
    //          .. params
    //      ).await
    // }
    // type X  = bool;
}

/// Mutable file operations are defined in this interface.
#[remote_interface]
pub trait MutableFileOps {
    /// Create a new file at the new path
    async fn create_file(path: PathBuf, truncate: bool) -> Result<(bool, i32), ()>;
}

#[cfg(test)]
#[allow(unused)]
mod tests {

    use async_trait::async_trait;
    use rfs_core::RemotelyInvocable;
    use std::io::Write;

    use super::*;

    struct S;

    #[async_trait]
    impl ImmutableFileOps for S {
        async fn read_file(path: PathBuf, offset: Option<usize>) -> Vec<u8> {
            todo!()
        }
    }

    // #[async_trait]
    impl MutableFileOps for S {
        #[doc = r" Create a new file at the new path"]
        #[must_use]
        #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
        fn create_file<'async_trait>(
            path: PathBuf,
            truncate: bool,
        ) -> ::core::pin::Pin<
            Box<
                dyn ::core::future::Future<Output = Result<(bool, i32), ()>>
                    + ::core::marker::Send
                    + 'async_trait,
            >,
        > {
            todo!()
        }
    }

    /// Test the fully integrated ser/de of the payload of a remote invocation.
    #[test]
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

    #[test]
    fn test_async_fn_pointers() {
        let x: Box<
            dyn Fn(
                PathBuf,
                Option<usize>,
            ) -> ::core::pin::Pin<
                Box<dyn ::core::future::Future<Output = Vec<u8>> + ::core::marker::Send>,
            >,
        > = Box::new(<S as ImmutableFileOps>::read_file);

        let y: fn(
            PathBuf,
            bool,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(bool, i32), ()>> + Send>,
        > = <S as MutableFileOps>::create_file;
    }
}
