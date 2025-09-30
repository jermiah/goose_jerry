#!/bin/bash
# Script to test different prompt combinations for goose
# This script helps you toggle between system.md/system_v2.md and recipe.md/recipe_code_tasks.md

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_header() {
    echo -e "${BLUE}=======================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}=======================================================${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Function to display the menu
show_menu() {
    clear
    print_header "goose Prompt Testing Menu"
    echo ""
    echo "Select a prompt combination to test:"
    echo ""
    echo "1) system.md + recipe.md (DEFAULT)"
    echo "2) system.md + recipe_code_tasks.md"
    echo "3) system_v2.md + recipe.md"
    echo "4) system_v2.md + recipe_code_tasks.md (RECOMMENDED FOR CODING)"
    echo ""
    echo "5) Run all combinations sequentially"
    echo "6) Build goose (cargo build)"
    echo "7) Run tests"
    echo "8) Exit"
    echo ""
}

# Function to set environment variables and run goose
run_test() {
    local system_prompt=$1
    local recipe_prompt=$2
    local test_name=$3
    
    print_header "Testing: $test_name"
    
    # Set environment variables
    export GOOSE_SYSTEM_PROMPT=$system_prompt
    export GOOSE_RECIPE_PROMPT=$recipe_prompt
    
    print_success "Environment variables set:"
    echo "  GOOSE_SYSTEM_PROMPT=$GOOSE_SYSTEM_PROMPT"
    echo "  GOOSE_RECIPE_PROMPT=$GOOSE_RECIPE_PROMPT"
    echo ""
    
    print_warning "You can now run goose commands with this configuration."
    echo "Examples:"
    echo "  ./target/debug/goose session"
    echo "  ./target/debug/goose run -t 'Write a function to calculate fibonacci'"
    echo ""
    echo "Press Enter to continue to next test or Ctrl+C to exit..."
    read
}

# Function to run a quick test with a sample prompt
run_quick_test() {
    local system_prompt=$1
    local recipe_prompt=$2
    local test_name=$3
    
    print_header "Quick Test: $test_name"
    
    # Set environment variables
    export GOOSE_SYSTEM_PROMPT=$system_prompt
    export GOOSE_RECIPE_PROMPT=$recipe_prompt
    
    print_success "Testing with: GOOSE_SYSTEM_PROMPT=$system_prompt, GOOSE_RECIPE_PROMPT=$recipe_prompt"
    
    # Run a simple test command
    echo ""
    echo "Running: ./target/debug/goose run -t 'Write a simple hello world function in Python'"
    echo ""
    
    ./target/debug/goose run -t "Write a simple hello world function in Python"
    
    echo ""
    print_success "Test completed for: $test_name"
    echo ""
}

# Function to build goose
build_goose() {
    print_header "Building goose"
    cargo build
    print_success "Build completed!"
    echo ""
    echo "Press Enter to continue..."
    read
}

# Function to run tests
run_tests() {
    print_header "Running Tests"
    cargo test -p goose
    print_success "Tests completed!"
    echo ""
    echo "Press Enter to continue..."
    read
}

# Main loop
while true; do
    show_menu
    read -p "Enter your choice [1-8]: " choice
    
    case $choice in
        1)
            run_test "default" "default" "system.md + recipe.md (DEFAULT)"
            ;;
        2)
            run_test "default" "code_tasks" "system.md + recipe_code_tasks.md"
            ;;
        3)
            run_test "v2" "default" "system_v2.md + recipe.md"
            ;;
        4)
            run_test "v2" "code_tasks" "system_v2.md + recipe_code_tasks.md (RECOMMENDED)"
            ;;
        5)
            print_header "Running All Combinations Sequentially"
            echo ""
            print_warning "This will run quick tests for all 4 combinations."
            echo "Press Enter to start or Ctrl+C to cancel..."
            read
            
            run_quick_test "default" "default" "Combination 1: system.md + recipe.md"
            run_quick_test "default" "code_tasks" "Combination 2: system.md + recipe_code_tasks.md"
            run_quick_test "v2" "default" "Combination 3: system_v2.md + recipe.md"
            run_quick_test "v2" "code_tasks" "Combination 4: system_v2.md + recipe_code_tasks.md"
            
            print_success "All tests completed!"
            echo ""
            echo "Press Enter to return to menu..."
            read
            ;;
        6)
            build_goose
            ;;
        7)
            run_tests
            ;;
        8)
            print_success "Exiting..."
            exit 0
            ;;
        *)
            print_error "Invalid option. Please try again."
            sleep 2
            ;;
    esac
done
