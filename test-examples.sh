#!/bin/bash

set -e

echo "Building compiler..."
cargo build --release

echo ""
echo "Testing examples..."
echo ""

echo "1. Hello World"
./target/release/hilowc examples/hello.hl -o examples/hello
./examples/hello

echo ""
echo "2. Fibonacci (should exit with code 55)"
./target/release/hilowc examples/fibonacci.hl -o examples/fibonacci
./examples/fibonacci || EXIT_CODE=$?
if [ $EXIT_CODE -eq 55 ]; then
    echo "✓ Fibonacci correct (exit code $EXIT_CODE)"
else
    echo "✗ Fibonacci incorrect (exit code $EXIT_CODE, expected 55)"
    exit 1
fi

echo ""
echo "3. Arithmetic (should exit with code 242)"
./target/release/hilowc examples/arithmetic.hl -o examples/arithmetic
./examples/arithmetic || EXIT_CODE=$?
if [ $EXIT_CODE -eq 242 ]; then
    echo "✓ Arithmetic correct (exit code $EXIT_CODE)"
else
    echo "✗ Arithmetic incorrect (exit code $EXIT_CODE, expected 242)"
    exit 1
fi

echo ""
echo "4. Loop (should exit with code 45)"
./target/release/hilowc examples/loop.hl -o examples/loop
./examples/loop || EXIT_CODE=$?
if [ $EXIT_CODE -eq 45 ]; then
    echo "✓ Loop correct (exit code $EXIT_CODE)"
else
    echo "✗ Loop incorrect (exit code $EXIT_CODE, expected 45)"
    exit 1
fi

echo ""
echo "═════════════════════════════════════"
echo "All tests passed! ✓"
echo "═════════════════════════════════════"
