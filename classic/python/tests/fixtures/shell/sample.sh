#!/bin/bash
# Sample shell script with various patterns

source /etc/profile
. ~/.bashrc

# Global variables
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VERSION="1.0.0"

# Function definitions
function setup() {
    echo "Setting up..."
    export PATH="$PATH:$SCRIPT_DIR/bin"
}

cleanup() {
    echo "Cleaning up..."
    rm -rf /tmp/temp_*
}

# Function with pipes and conditionals
process_data() {
    local input="$1"

    if [ -f "$input" ]; then
        cat "$input" | grep -v "^#" | sort | uniq > output.txt
    else
        echo "File not found: $input" >&2
        return 1
    fi
}

# Main execution
main() {
    setup
    process_data "$@"
    cleanup
}

if [ "${BASH_SOURCE[0]}" == "${0}" ]; then
    main "$@"
fi
