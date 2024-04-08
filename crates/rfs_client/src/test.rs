use std::io;
use std::time::Duration;

use rfs::fs::VirtFile;
use rfs::interfaces::*;
use rfs::middleware::ContextManager;

/// Test various stuff out
#[allow(unused)]
pub async fn test_mode(mut ctx: ContextManager) -> io::Result<()> {
    log::info!("testing remote invocations");
    let _ = SimpleOpsClient::say_hello(&mut ctx, "new configuration".to_string())
        .await
        .unwrap();

    let mut file = VirtFile::open(ctx.clone(), "remote_file.txt").await?;

    // let contents = rfs::fs::read_to_string(manager, "red_chips_v3.json").await?;

    // println!("file contents: {}", contents);
    // file.write_bytes(b"hello world").await?;
    let data = file.read_bytes().await?;

    println!("bytes read: {:?}", std::str::from_utf8(&data));

    let cloned_ctx = ctx.clone();
    // let cb_handle = tokio::spawn(async move {
    //     CallbackOpsClient::register_file_update(&mut cloned_ctx, "remote_file.txt".to_owned())
    //         .await
    //         .expect("middleware should not fail")
    // });

    log::info!("spawning separate watch task");
    let handle = tokio::spawn(async move {
        log::debug!("watching file");
        let mut file = VirtFile::open(cloned_ctx, "remote_file.txt").await.unwrap();

        match file.watch().await {
            Ok(c) => {
                log::info!("successfully received file update");
                c
            }
            Err(e) => {
                log::error!("error watching file: {:?}", e);
                return;
            }
        };
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    log::info!("writing to file from another client");
    let _ = file
        .write_bytes(FileUpdate::Insert((
            3,
            "hello world hello world\n".as_bytes().to_vec(),
        )))
        .await?;
    log::info!("wrote update to file");

    handle.await.expect("thread join error");

    log::info!("reading server's base directory");

    let read_dir = rfs::fs::read_dir(ctx, ".").await?;

    for entry in read_dir.iter() {
        log::info!("{:?}", entry);
    }

    Ok(())
}
