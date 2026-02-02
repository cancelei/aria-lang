#!/bin/bash

echo "=========================================="
echo "Aria Build Command Demonstration"
echo "=========================================="
echo

echo "1. Basic compilation (object file only)"
echo "   Command: aria build test_hello_world.aria"
./target/debug/aria build test_hello_world.aria
echo

echo "2. Compile and link to executable"
echo "   Command: aria build test_hello_world.aria --link"
./target/debug/aria build test_hello_world.aria --link
echo

echo "3. Run the executable"
echo "   Command: ./test_hello_world"
./test_hello_world
echo

echo "4. Optimized release build"
echo "   Command: aria build test_math.aria --link --release"
./target/debug/aria build test_math.aria --link --release
echo

echo "5. Run optimized executable"
echo "   Command: ./test_math"
./test_math
echo

echo "6. WebAssembly target"
echo "   Command: aria build test_hello_world.aria --target wasm32"
./target/debug/aria build test_hello_world.aria --target wasm32
echo

echo "7. Check WASM output"
echo "   Command: file test_hello_world.wasm"
file test_hello_world.wasm
echo

echo "=========================================="
echo "Demonstration Complete!"
echo "=========================================="
