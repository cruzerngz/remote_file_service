use std::path::PathBuf;

mod context_manager;
mod ser_de;

/// Remote file operations interface
pub trait RemoteFileOperations {
    /// Get the contents of a file.
    ///
    fn get_file(path: PathBuf, offset: Option<usize>) -> String;
}

/// This trait will be derived from any interface that has the
/// `#[remote_interface]` proc-macro.
trait RemoteMethodSignature {
    /// Returns the method signature of a remote interface method.
    ///
    /// Used for routing method calls on the server side.
    fn remote_method_signature() -> &'static [u8];
}

struct S;

impl RemoteMethodSignature for S {
    fn remote_method_signature() -> &'static [u8] {
        "Function::method".as_bytes()
    }
}

/// Macro testing mod
mod derive_tests {
    use rfs_macros::remote_interface;

    use crate::RemoteMethodSignature;

    #[remote_interface]
    pub trait FileOperations {
        fn get_file_info(path: String, offset: Option<usize>) -> String;

        fn create_file(path: String) -> bool;
    }
}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;
    use derive_tests::FileOperationsGetFileInfoMessage;
    use rfs_macros::remote_interface;

    #[remote_interface]
    pub trait ASD {
        fn get_file_info(path: String, offset: Option<usize>) -> String;

        fn create_file(path: String) -> bool;
    }

    #[test]
    fn asd() {
        let s = S::remote_method_signature();

        let res = ASDCreateFileMessage::remote_method_signature();
        let res = std::str::from_utf8(res).unwrap();

        println!("{:?}", s);

        println!("{:?}", res);
    }
}
