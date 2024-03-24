#!/bin/bash

LOG_FILE="run_tests.log"

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

        cargo r --bin rfs_server -- $INVO &
        SERVER_PID=$!
        kill $SERVER_PID
        cargo r --bin rfs_client -- $INVO $SIM_OMIT --test


        cargo r --bin rfs_server -- $INVO $SIM_OMIT &
        SERVER_PID=$!
        cargo r --bin rfs_client -- $INVO $SIM_OMIT --test
        kill $SERVER_PID

    done

done
