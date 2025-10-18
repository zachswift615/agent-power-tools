#!/bin/bash
cd /Users/zachswift/projects/agent-power-tools/synthia
echo "Checking if Synthia builds..."
cargo check 2>&1 | tee build_check.log
EXIT_CODE=${PIPESTATUS[0]}

if [ $EXIT_CODE -eq 0 ]; then
    echo "✓ Build check passed!"
    echo "Running tests..."
    cargo test --lib 2>&1 | tee test_output.log
    TEST_EXIT_CODE=${PIPESTATUS[0]}
    if [ $TEST_EXIT_CODE -eq 0 ]; then
        echo "✓ Tests passed!"
    else
        echo "✗ Tests failed with exit code $TEST_EXIT_CODE"
        echo "See test_output.log for details"
    fi
else
    echo "✗ Build check failed with exit code $EXIT_CODE"
    echo "See build_check.log for details"
fi

exit $EXIT_CODE
