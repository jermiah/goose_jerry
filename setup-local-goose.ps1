# ============================================================================
# GOOSE LOCAL BUILD SETUP - All-in-One Script
# ============================================================================
# This script sets up your local goose build to be used instead of Block installation
# Run this once to set permanent PATH, or run each session for temporary PATH

param(
    [switch]$Permanent,
    [switch]$Verify,
    [switch]$Help
)

$gooseDir = "E:\BlackboxAIwork\goose repo\goose\target\release"
$gooseExe = "$gooseDir\goose.exe"

# ============================================================================
# HELP
# ============================================================================
if ($Help) {
    Write-Host "=== GOOSE LOCAL BUILD SETUP ===" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "USAGE:" -ForegroundColor Yellow
    Write-Host "  .\setup-local-goose.ps1              # Temporary PATH (current session only)"
    Write-Host "  .\setup-local-goose.ps1 -Permanent   # Permanent PATH (all future sessions)"
    Write-Host "  .\setup-local-goose.ps1 -Verify      # Verify which goose is active"
    Write-Host "  .\setup-local-goose.ps1 -Help        # Show this help"
    Write-Host ""
    Write-Host "EXAMPLES:" -ForegroundColor Yellow
    Write-Host "  # Quick start (run every new PowerShell session):"
    Write-Host "  cd 'E:\BlackboxAIwork\goose repo\goose'"
    Write-Host "  .\setup-local-goose.ps1"
    Write-Host "  goose session"
    Write-Host ""
    Write-Host "  # One-time permanent setup:"
    Write-Host "  cd 'E:\BlackboxAIwork\goose repo\goose'"
    Write-Host "  .\setup-local-goose.ps1 -Permanent"
    Write-Host ""
    exit
}

# ============================================================================
# VERIFY MODE
# ============================================================================
if ($Verify) {
    Write-Host "=== GOOSE VERIFICATION ===" -ForegroundColor Cyan
    Write-Host ""
    
    # Check if goose command exists
    try {
        $gooseCmd = Get-Command goose -ErrorAction Stop
        Write-Host "[SUCCESS] Goose command found!" -ForegroundColor Green
        Write-Host "  Location: $($gooseCmd.Source)" -ForegroundColor White
        
        $fileInfo = Get-Item $gooseCmd.Source
        Write-Host "  Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor White
        Write-Host "  Modified: $($fileInfo.LastWriteTime)" -ForegroundColor White
        Write-Host ""
        
        # Check if it's the local build
        if ($gooseCmd.Source -eq $gooseExe) {
            Write-Host "[SUCCESS] Using YOUR LOCAL BUILD!" -ForegroundColor Green -BackgroundColor DarkGreen
        } else {
            Write-Host "[WARNING] Using different installation!" -ForegroundColor Yellow
            Write-Host "  Expected: $gooseExe" -ForegroundColor Gray
            Write-Host "  Actual: $($gooseCmd.Source)" -ForegroundColor Gray
        }
        
        Write-Host ""
        Write-Host "Version:" -ForegroundColor Yellow
        & goose --version
        
        Write-Host ""
        Write-Host "Configuration:" -ForegroundColor Yellow
        & goose info
        
    } catch {
        Write-Host "[ERROR] Goose command not found!" -ForegroundColor Red
        Write-Host "  Run: .\setup-local-goose.ps1" -ForegroundColor Yellow
    }
    
    exit
}

# ============================================================================
# MAIN SETUP
# ============================================================================
Write-Host "=== GOOSE LOCAL BUILD SETUP ===" -ForegroundColor Cyan
Write-Host ""

# Check if local build exists
if (-not (Test-Path $gooseExe)) {
    Write-Host "[ERROR] Local build not found at:" -ForegroundColor Red
    Write-Host "  $gooseExe" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Please build goose first:" -ForegroundColor Yellow
    Write-Host "  cd 'E:\BlackboxAIwork\goose repo\goose'"
    Write-Host "  cargo build --release"
    exit 1
}

Write-Host "[1] Local build found:" -ForegroundColor Green
$fileInfo = Get-Item $gooseExe
Write-Host "    Location: $gooseExe" -ForegroundColor White
Write-Host "    Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB" -ForegroundColor White
Write-Host "    Modified: $($fileInfo.LastWriteTime)" -ForegroundColor White
Write-Host ""

# ============================================================================
# PERMANENT PATH SETUP
# ============================================================================
if ($Permanent) {
    Write-Host "[2] Setting up PERMANENT PATH..." -ForegroundColor Yellow
    Write-Host ""
    
    # Get current user PATH
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    
    # Check if already in PATH
    if ($currentPath -like "*$gooseDir*") {
        Write-Host "    Already in permanent PATH!" -ForegroundColor Green
    } else {
        # Add to beginning of PATH
        $newPath = "$gooseDir;$currentPath"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "    [SUCCESS] Added to permanent PATH!" -ForegroundColor Green
        Write-Host ""
        Write-Host "    IMPORTANT: Close and reopen PowerShell for changes to take effect!" -ForegroundColor Yellow
    }
    
    # Also set for current session
    $env:Path = "$gooseDir;$env:Path"
    Write-Host "    Also set for current session" -ForegroundColor Gray
    
} else {
    # ============================================================================
    # TEMPORARY PATH SETUP (Current Session Only)
    # ============================================================================
    Write-Host "[2] Setting up TEMPORARY PATH (current session only)..." -ForegroundColor Yellow
    
    # Clean existing PATH entries for this directory
    $pathParts = $env:Path -split ";"
    $cleanPath = ($pathParts | Where-Object { $_ -ne $gooseDir }) -join ";"
    
    # Prepend to PATH
    $env:Path = "$gooseDir;$cleanPath"
    
    Write-Host "    [SUCCESS] Added to PATH for this session!" -ForegroundColor Green
    Write-Host ""
    Write-Host "    NOTE: This is temporary. Run this script each new PowerShell session." -ForegroundColor Gray
    Write-Host "    Or use -Permanent flag for one-time permanent setup." -ForegroundColor Gray
}

Write-Host ""
Write-Host "[3] Verifying setup..." -ForegroundColor Yellow

try {
    $gooseCmd = Get-Command goose -ErrorAction Stop
    Write-Host "    Goose command: $($gooseCmd.Source)" -ForegroundColor White
    
    if ($gooseCmd.Source -eq $gooseExe) {
        Write-Host "    [SUCCESS] Local build is active!" -ForegroundColor Green
    } else {
        Write-Host "    [WARNING] Different goose found first in PATH!" -ForegroundColor Yellow
    }
} catch {
    Write-Host "    [ERROR] Goose command not found!" -ForegroundColor Red
}

Write-Host ""
Write-Host "=== SETUP COMPLETE ===" -ForegroundColor Green
Write-Host ""
Write-Host "QUICK START:" -ForegroundColor Cyan
Write-Host "  goose session       # Start a new session"
Write-Host "  goose --help        # Show all commands"
Write-Host "  goose info          # Show configuration"
Write-Host ""
Write-Host "VERIFY:" -ForegroundColor Cyan
Write-Host "  .\setup-local-goose.ps1 -Verify"
Write-Host ""

if (-not $Permanent) {
    Write-Host "TIP: For permanent setup, run:" -ForegroundColor Yellow
    Write-Host "  .\setup-local-goose.ps1 -Permanent"
    Write-Host ""
}
