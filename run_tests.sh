#!/bin/bash

LOG_FILE="run_tests.log"

set -e
set -x

INVOCATION_SEMANTICS=(
    "--invocation-semantics maybe"
    "--invocation-semantics at-least-once"
    "--invocation-semantics at-most-once"
)

SIM_OMMISIONS=(
    "--simulate-ommisions 10"
    "--simulate-ommisions 100"
    "--simulate-ommisions 1000"
    "--simulate-ommisions 10000"
    "--simulate-ommisions 100000"
    "--simulate-ommisions 1000000"
)

# iterate over each elemtn in the INVOCATION_SEMANTICS array
for INVO in "${INVOCATION_SEMANTICS[@]}"; do

    # iter over all SIM_OMMISIONS
    for SIM_OMIT in "${SIM_OMMISIONS[@]}"; do

        echo "Running tests with $INVO and $SIM_OMIT" >> $LOG_FILE

        echo "running normal server" >> $LOG_FILE
        cargo run --release --bin rfs_server -- $INVO &
        SERVER_PID=$!
        cargo run --bin rfs_client -- $INVO $SIM_OMIT --test
        kill $SERVER_PID

        echo "running faulty server" >> $LOG_FILE
        cargo run --bin rfs_server -- $INVO $SIM_OMIT &
        SERVER_PID=$!
        cargo run --release --bin rfs_client -- $INVO $SIM_OMIT --test
        kill $SERVER_PID

        sleep 0.5
    done

done
