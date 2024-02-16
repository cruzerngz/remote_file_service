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
    use rfs_macros::remote_message_from_trait;


    #[remote_message_from_trait]
    trait FileOperatiosn {
        fn get_file_info(path: String, offset: Option<usize>) -> String;

        fn create_file(path: String) -> bool;
    }

}

#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;

    #[test]
    fn asd() {
        let s = S::remote_method_signature();

        println!("{:?}", s);
    }
}


