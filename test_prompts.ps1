# PowerShell script to test different prompt combinations for goose
# This script helps you toggle between system.md/system_v2.md and recipe.md/recipe_code_tasks.md

# Function to print colored output
function Print-Header {
    param([string]$Message)
    Write-Host "=======================================================" -ForegroundColor Blue
    Write-Host $Message -ForegroundColor Blue
    Write-Host "=======================================================" -ForegroundColor Blue
}

function Print-Success {
    param([string]$Message)
    Write-Host "✓ $Message" -ForegroundColor Green
}

function Print-Warning {
    param([string]$Message)
    Write-Host "⚠ $Message" -ForegroundColor Yellow
}

function Print-Error {
    param([string]$Message)
    Write-Host "✗ $Message" -ForegroundColor Red
}

# Function to display the menu
function Show-Menu {
    Clear-Host
    Print-Header "goose Prompt Testing Menu"
    Write-Host ""
    Write-Host "Select a prompt combination to test:"
    Write-Host ""
    Write-Host "1) system.md + recipe.md (DEFAULT)"
    Write-Host "2) system.md + recipe_code_tasks.md"
    Write-Host "3) system_v2.md + recipe.md"
    Write-Host "4) system_v2.md + recipe_code_tasks.md (RECOMMENDED FOR CODING)"
    Write-Host ""
    Write-Host "5) Run all combinations sequentially"
    Write-Host "6) Build goose (cargo build)"
    Write-Host "7) Run tests"
    Write-Host "8) Exit"
    Write-Host ""
}

# Function to set environment variables and run goose
function Run-Test {
    param(
        [string]$SystemPrompt,
        [string]$RecipePrompt,
        [string]$TestName
    )
    
    Print-Header "Testing: $TestName"
    
    # Set environment variables
    $env:GOOSE_SYSTEM_PROMPT = $SystemPrompt
    $env:GOOSE_RECIPE_PROMPT = $RecipePrompt
    
    Print-Success "Environment variables set:"
    Write-Host "  GOOSE_SYSTEM_PROMPT=$env:GOOSE_SYSTEM_PROMPT"
    Write-Host "  GOOSE_RECIPE_PROMPT=$env:GOOSE_RECIPE_PROMPT"
    Write-Host ""
    
    Print-Warning "You can now run goose commands with this configuration."
    Write-Host "Examples:"
    Write-Host "  .\target\debug\goose.exe session"
    Write-Host "  .\target\debug\goose.exe run -t 'Write a function to calculate fibonacci'"
    Write-Host ""
    Write-Host "Press Enter to continue to next test or Ctrl+C to exit..."
    Read-Host
}

# Function to run a quick test with a sample prompt
function Run-QuickTest {
    param(
        [string]$SystemPrompt,
        [string]$RecipePrompt,
        [string]$TestName
    )
    
    Print-Header "Quick Test: $TestName"
    
    # Set environment variables
    $env:GOOSE_SYSTEM_PROMPT = $SystemPrompt
    $env:GOOSE_RECIPE_PROMPT = $RecipePrompt
    
    Print-Success "Testing with: GOOSE_SYSTEM_PROMPT=$SystemPrompt, GOOSE_RECIPE_PROMPT=$RecipePrompt"
    
    # Run a simple test command
    Write-Host ""
    Write-Host "Running: .\target\debug\goose.exe run -t 'Write a simple hello world function in Python'"
    Write-Host ""
    
    & .\target\debug\goose.exe run -t "Write a simple hello world function in Python"
    
    Write-Host ""
    Print-Success "Test completed for: $TestName"
    Write-Host ""
}

# Function to build goose
function Build-Goose {
    Print-Header "Building goose"
    cargo build
    Print-Success "Build completed!"
    Write-Host ""
    Write-Host "Press Enter to continue..."
    Read-Host
}

# Function to run tests
function Run-Tests {
    Print-Header "Running Tests"
    cargo test -p goose
    Print-Success "Tests completed!"
    Write-Host ""
    Write-Host "Press Enter to continue..."
    Read-Host
}

# Main loop
while ($true) {
    Show-Menu
    $choice = Read-Host "Enter your choice [1-8]"
    
    switch ($choice) {
        "1" {
            Run-Test "default" "default" "system.md + recipe.md (DEFAULT)"
        }
        "2" {
            Run-Test "default" "code_tasks" "system.md + recipe_code_tasks.md"
        }
        "3" {
            Run-Test "v2" "default" "system_v2.md + recipe.md"
        }
        "4" {
            Run-Test "v2" "code_tasks" "system_v2.md + recipe_code_tasks.md (RECOMMENDED)"
        }
        "5" {
            Print-Header "Running All Combinations Sequentially"
            Write-Host ""
            Print-Warning "This will run quick tests for all 4 combinations."
            Write-Host "Press Enter to start or Ctrl+C to cancel..."
            Read-Host
            
            Run-QuickTest "default" "default" "Combination 1: system.md + recipe.md"
            Run-QuickTest "default" "code_tasks" "Combination 2: system.md + recipe_code_tasks.md"
            Run-QuickTest "v2" "default" "Combination 3: system_v2.md + recipe.md"
            Run-QuickTest "v2" "code_tasks" "Combination 4: system_v2.md + recipe_code_tasks.md"
            
            Print-Success "All tests completed!"
            Write-Host ""
            Write-Host "Press Enter to return to menu..."
            Read-Host
        }
        "6" {
            Build-Goose
        }
        "7" {
            Run-Tests
        }
        "8" {
            Print-Success "Exiting..."
            exit 0
        }
        default {
            Print-Error "Invalid option. Please try again."
            Start-Sleep -Seconds 2
        }
    }
}
