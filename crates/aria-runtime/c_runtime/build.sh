#!/bin/bash
#
# Aria Runtime Build Script
#
# This script provides a simple interface for building the Aria runtime
# and linking Aria programs into executables.

set -e

# Configuration
CC="${CC:-gcc}"
CFLAGS="${CFLAGS:--Wall -Wextra -O2 -std=c11}"
RUNTIME_SRC="aria_runtime.c"
RUNTIME_OBJ="aria_runtime.o"

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print colored message
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Build the runtime library
build_runtime() {
    print_info "Building Aria runtime library..."
    $CC $CFLAGS -c "$RUNTIME_SRC" -o "$RUNTIME_OBJ"
    print_info "Runtime library built: $RUNTIME_OBJ"
}

# Link a program with the runtime
link_program() {
    local program_obj="$1"
    local output="$2"

    if [ -z "$program_obj" ]; then
        print_error "No program object file specified"
        return 1
    fi

    if [ ! -f "$program_obj" ]; then
        print_error "Program object file not found: $program_obj"
        return 1
    fi

    if [ -z "$output" ]; then
        # Default output name: remove .o extension
        output="${program_obj%.o}"
    fi

    # Ensure runtime is built
    if [ ! -f "$RUNTIME_OBJ" ]; then
        build_runtime
    fi

    print_info "Linking $program_obj with runtime..."
    $CC "$RUNTIME_OBJ" "$program_obj" -o "$output"
    print_info "Created executable: $output"
}

# Clean build artifacts
clean() {
    print_info "Cleaning build artifacts..."
    rm -f "$RUNTIME_OBJ"
    print_info "Clean complete"
}

# Show usage information
usage() {
    cat << EOF
Aria Runtime Build Script

Usage:
    $0 <command> [arguments]

Commands:
    build                   Build the runtime library
    link <obj> [output]     Link an Aria program with the runtime
    clean                   Remove build artifacts
    help                    Show this help message

Examples:
    $0 build                        # Build the runtime
    $0 link hello.o                 # Link hello.o -> hello
    $0 link hello.o myprogram       # Link hello.o -> myprogram
    $0 clean                        # Clean build files

Environment Variables:
    CC                      C compiler (default: gcc)
    CFLAGS                  Compiler flags (default: -Wall -Wextra -O2 -std=c11)

EOF
}

# Main script logic
main() {
    local command="${1:-help}"

    case "$command" in
        build)
            build_runtime
            ;;
        link)
            if [ $# -lt 2 ]; then
                print_error "link command requires a program object file"
                echo ""
                usage
                exit 1
            fi
            link_program "$2" "$3"
            ;;
        clean)
            clean
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            print_error "Unknown command: $command"
            echo ""
            usage
            exit 1
            ;;
    esac
}

main "$@"
