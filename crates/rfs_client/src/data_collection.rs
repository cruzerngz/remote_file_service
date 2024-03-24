//! Data collection module. Tests a particular protocol and the success rate
#![allow(unused)]

use std::{
    borrow::BorrowMut,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    net::{Ipv4Addr, SocketAddrV4},
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use rfs::{
    interfaces::TestOpsClient,
    middleware::{
        ContextManager, DefaultProto, FaultyDefaultProto, FaultyHandshakeProto,
        FaultyRequestAckProto, HandshakeProto, RequestAckProto, TransmissionProtocol,
    },
};
use serde::Serialize;
use tokio::sync::Mutex;

use crate::args::InvocationSemantics;

/// Number of test iterations to perform
const TEST_ITERATIONS: usize = 10;

// /// Number of method calls to perform every test iteration
// const METHOD_CALLS: usize = 1000;

/// Number of detected failures until we stop testing a particular protocol
const NUM_FAILURE_THRESHOLD: usize = 50;

/// Wait for this method calls before performing termination checks
///
/// Termination checks will terminate the test if the failure rate is too low.
/// I want to save time.
const MIN_METHOD_CALLS_TO_PROB_CHECK: usize = 10_000;

/// Maximum number of method calls to perform in a single test iteration
/// If the failure threshold is not reached, we stop testing the protocol
const MAX_METHOD_CALLS: usize = 10_000;

/// We will go through six "nines" of network reliability:
/// 90% -> 99% -> 99.9% -> ...
///
/// If I test any more than this I will be waiting forever for a failure.
const INV_PROBABILITIES: [usize; 6] = [10, 100, 1_000, 10_000, 100_000, 1_000_000];

#[derive(Debug, Default, Serialize)]
struct TestResult {
    // protocol names
    client_protocol: String,
    remote_protocol: String,

    // protocol simulated failures
    // client_failures: bool,
    // remote_failures: bool,

    // failure probabilities (same for both client and remote)
    inverse_failure_probability: Option<u32>,

    // test results
    init_count: usize,
    init_failures: usize,

    method_call_count: usize,
    method_call_failures: usize,

    non_idempotent_calls: usize,
    non_idempotent_mismatches: usize,
}

/// Run a test based on the consts defined above
pub async fn test(
    semantics: InvocationSemantics,
    inv_prob: u32, // used only for faulty protos
    source: Ipv4Addr,
    target: Ipv4Addr,
    port: u16,
    timeout: Duration,
    retries: u8,
) -> io::Result<()> {
    // let (normal_proto, faulty_proto) = match semantics {
    //     InvocationSemantics::Maybe => (Arc::new(DefaultProto), Arc::new(DefaultProto)),
    //     InvocationSemantics::AtLeastOnce => todo!(),
    //     InvocationSemantics::AtMostOnce => todo!(),
    // };

    let (normal_proto, faulty_proto): (
        Arc<dyn TransmissionProtocol + Send + Sync>,
        Arc<dyn TransmissionProtocol + Send + Sync>,
    ) = match semantics {
        InvocationSemantics::Maybe => (
            Arc::new(DefaultProto),
            Arc::new(FaultyDefaultProto::from_frac(inv_prob as u32)),
        ),
        InvocationSemantics::AtLeastOnce => (
            Arc::new(RequestAckProto),
            Arc::new(FaultyRequestAckProto::from_frac(inv_prob as u32)),
        ),
        InvocationSemantics::AtMostOnce => (
            Arc::new(HandshakeProto),
            Arc::new(FaultyHandshakeProto::from_frac(inv_prob as u32)),
        ),
    };

    let mut temp_ctx = ContextManager::new(
        source,
        SocketAddrV4::new(target, port),
        timeout,
        retries,
        normal_proto.clone(),
    )
    .await?;

    let remote_proto = get_remote_protocol_name(&mut temp_ctx).await;

    let mut faulty_res = TestResult {
        client_protocol: format!("{}", faulty_proto),
        remote_protocol: remote_proto.clone(),
        inverse_failure_probability: Some(inv_prob),
        ..Default::default()
    };

    let mut res = TestResult {
        client_protocol: format!("{}", normal_proto),
        remote_protocol: remote_proto.clone(),
        inverse_failure_probability: None,
        ..Default::default()
    };

    // test normal proto
    for test_iter in 0..TEST_ITERATIONS {
        single_test_iteration(
            normal_proto.clone(),
            source,
            target,
            port,
            timeout,
            retries,
            &mut res,
        )
        .await?;
    }

    // test faulty proto
    for test_iter in 0..TEST_ITERATIONS {
        single_test_iteration(
            normal_proto.clone(),
            source,
            target,
            port,
            timeout,
            retries,
            &mut faulty_res,
        )
        .await?;
    }

    write_results_to_file(&[res, faulty_res])?;

    Ok(())
}

/// Write the results to a file.
///
/// The file is named according to these fields of the first element:
/// - remote protocol
/// - failure probability
fn write_results_to_file(results: &[TestResult]) -> io::Result<()> {
    let failure_prob = results
        .iter()
        .find_map(|r| match r.inverse_failure_probability {
            Some(p) => Some(p),
            None => None,
        })
        .expect("one element must have a failure probability defined");

    let file_name = format!("test_{}_{}.csv", results[0].remote_protocol, failure_prob);
    log::info!("writing to file: {}", file_name);

    let mut csv_writer = csv::Writer::from_path(file_name)?;
    for result in results.iter() {
        csv_writer.serialize(result)?;
    }

    csv_writer.flush()?;

    Ok(())
}

/// Get the status of the remote and what protocol it is using.
///
/// This function will never fail.
async fn get_remote_protocol_name(ctx: &mut ContextManager) -> String {
    let remote_proto_name = loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                log::error!("timeout, retrying...");
                continue;
            },

            remote_proto_res = TestOpsClient::get_remote_protocol(ctx) => {
                match remote_proto_res {
                    Ok(p) => break p,
                    Err(_) => {
                        log::error!("failed to get remote protocol, retrying...");
                        continue;
                    }
                }
            }
        }
    };

    remote_proto_name
}

/// Run a single test iteration for a particular protocol
async fn single_test_iteration(
    proto: Arc<dyn TransmissionProtocol + Send + Sync>,
    // client_sim_fail: bool,
    // inv_probability: Option<usize>,
    source: Ipv4Addr,
    target: Ipv4Addr,
    port: u16,
    timeout: Duration,
    retries: u8,
    results: &mut TestResult,
) -> io::Result<()> {
    results.init_count += 1;

    let mut ctx = match ContextManager::new(
        source,
        SocketAddrV4::new(target, port),
        timeout,
        retries,
        proto,
    )
    .await
    {
        Ok(ctx) => ctx,
        Err(e) => {
            results.init_failures += 1;
            return Ok(());
        }
    };

    let mut num_method_calls = 0;
    let mut method_failures = 0;

    // a very generous timeout for executing a single method call
    let method_call_absolute_timeout = timeout * retries as u32 * 10;

    while num_method_calls < MAX_METHOD_CALLS {
        log::debug!(
            "method call count: {}, failure count: {}",
            num_method_calls,
            method_failures
        );

        if method_failures >= NUM_FAILURE_THRESHOLD {
            break;
        }

        // early exit for very reliable protocols
        if num_method_calls >= MIN_METHOD_CALLS_TO_PROB_CHECK {
            let failure_rate = method_failures as f64 / num_method_calls as f64;
            if failure_rate < 0.001 {
                break;
            }
        }

        let u_id = {
            let now = std::time::SystemTime::now();
            let mut hasher = DefaultHasher::new();
            now.hash(&mut hasher);

            hasher.finish()
        };

        // idempotent
        // need to implement timeout here cause of maybe semantics
        num_method_calls += 1;
        tokio::select! {
            _ = tokio::time::sleep(method_call_absolute_timeout) => {
                method_failures += 1;
            },

            method_call_res = TestOpsClient::test_idempotent(&mut ctx, u_id) => {
                match method_call_res {
                    Ok(_) => (),
                    Err(_) => {
                        method_failures += 1;
                    }
                }

            }
        }

        // non-idempotent
        num_method_calls += 1;
        tokio::select! {
            _ = tokio::time::sleep(method_call_absolute_timeout) => {
                method_failures += 1;
            },

            method_call_res = TestOpsClient::test_non_idempotent(&mut ctx, u_id) => {
                match method_call_res {
                    Ok(val) => {
                        results.non_idempotent_calls += 1;

                        if val != 1 {
                            results.non_idempotent_mismatches += 1;
                        }
                    },

                    Err(_) => {
                        method_failures += 1;
                    }
                }

            }
        }

        // reset non-idempotent
        num_method_calls += 1;
        tokio::select! {
            _ = tokio::time::sleep(method_call_absolute_timeout) => {
                method_failures += 1;
            },

            method_call_res = TestOpsClient::reset_non_idempotent(&mut ctx) => {
                match method_call_res {
                    Ok(_) => (),
                    Err(_) => {
                        method_failures += 1;
                    }
                }

            }
        }
    }

    results.method_call_count += num_method_calls;
    results.method_call_failures += method_failures;

    Ok(())
}
