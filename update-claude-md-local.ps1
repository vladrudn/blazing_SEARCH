# PowerShell script for local CLAUDE.md update
# Equivalent to GitHub Actions workflow

param(
    [string]$AnalysisDepth = "standard",  # basic, standard, detailed
    [switch]$SkipBackup = $false,         # Skip backup
    [switch]$Verbose = $false             # Verbose output
)

# Color output function
function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$ForegroundColor = "White"
    )
    Write-Host $Message -ForegroundColor $ForegroundColor
}

# Check if script is run from correct directory
function Test-ProjectDirectory {
    if (-not (Test-Path "Cargo.toml")) {
        Write-ColorOutput "ERROR: Script must be run from blazing_SEARCH project root directory" "Red"
        Write-ColorOutput "Current directory: $(Get-Location)" "Yellow"
        Write-ColorOutput "Expected directory should contain Cargo.toml file" "Yellow"
        exit 1
    }
    
    if (-not (Test-Path "src\main.rs")) {
        Write-ColorOutput "ERROR: src\main.rs file not found" "Red"
        exit 1
    }
    
    Write-ColorOutput "Project blazing_SEARCH found successfully" "Green"
}

# Check Rust syntax
function Test-RustSyntax {
    Write-ColorOutput "`nChecking Rust project syntax..." "Yellow"
    
    try {
        $result = cargo check --verbose 2>&1
        if ($LASTEXITCODE -ne 0) {
            Write-ColorOutput "ERROR: Rust syntax errors found!" "Red"
            Write-ColorOutput "cargo check output:" "Gray"
            Write-Host $result
            exit 1
        }
        Write-ColorOutput "Rust code syntax is valid" "Green"
        
        if ($Verbose) {
            Write-ColorOutput "Detailed cargo check output:" "Gray"
            Write-Host $result
        }
    }
    catch {
        Write-ColorOutput "ERROR executing cargo check: $_" "Red"
        exit 1
    }
}

# Generate project analysis
function New-ProjectAnalysis {
    Write-ColorOutput "`nGenerating detailed project analysis..." "Yellow"
    
    $analysisFile = "project_analysis_temp.md"
    
    $analysisContent = @"
# blazing_SEARCH - Rust Document Search Engine Analysis

## Project Overview

**blazing_SEARCH** - high-performance document search engine written in Rust using Actix-web framework.

### Key Features:
- DOCX document support with full text analysis
- Inverted index for fast search
- Atomic index management with incremental updates
- Web interface based on Actix-web
- Thread-safe operations using async/await

## Architecture Modules:

### Core Components:
"@

    # Analyze Cargo.toml
    Write-ColorOutput "Analyzing dependencies..." "Cyan"
    if (Test-Path "Cargo.toml") {
        $cargoContent = Get-Content "Cargo.toml" -Raw -Encoding UTF8
        $analysisContent += @"

### Cargo.toml - Dependencies:
```toml
$cargoContent
```

**Key Dependencies:**
- **actix-web 4.8** - Asynchronous web framework
- **tokio 1.0** - Async runtime with full features  
- **serde + serde_json** - JSON serialization
- **quick-xml 0.36** - Fast XML parser
- **regex 1.10** - Regular expressions
- **zip 0.6** - ZIP archive handling (DOCX)
- **walkdir 2.4** - Recursive directory traversal
- **chrono 0.4** - Date and time handling
- **fs4 0.9** - Extended file operations

"@
    }
    
    # Analyze src modules
    Write-ColorOutput "Analyzing src/ modules..." "Cyan"
    $srcFiles = Get-ChildItem "src\*.rs" | Sort-Object Name
    
    $analysisContent += @"
### Rust Modules (src/):

"@
    
    foreach ($file in $srcFiles) {
        $fileName = $file.Name
        $content = ""
        try {
            $content = Get-Content $file.FullName -Raw -Encoding UTF8 -ErrorAction SilentlyContinue
        }
        catch {
            $content = ""
        }
        
        $description = switch ($fileName) {
            "main.rs" { "**Main Module** - application entry point with CLI/Web modes" }
            "atomic_index_manager.rs" { "**Atomic Index Manager** - thread-safe index operations" }
            "auto_indexer.rs" { "**Auto Indexer** - background document indexing" }
            "document_record.rs" { "**Document Structures** - data models for documents" }
            "docx_parser.rs" { "**DOCX Parser** - Word document analysis" }
            "folder_processor.rs" { "**Folder Processor** - recursive folder processing" }
            "inverted_index.rs" { "**Inverted Index** - fast text search" }
            "search_engine.rs" { "**Search Engine** - core search logic" }
            "web_server.rs" { "**Web Server** - Actix-web HTTP server" }
            default { "**$fileName** - additional module" }
        }
        
        $size = if ($content) { [math]::Round($content.Length / 1024, 2) } else { 0 }
        
        $analysisContent += @"
#### $description
- **File:** ``src/$fileName``
- **Size:** $size KB

"@

        # Add function analysis for main.rs
        if ($fileName -eq "main.rs" -and $content.Length -gt 0) {
            $functions = [regex]::Matches($content, "fn\s+(\w+)") | ForEach-Object { $_.Groups[1].Value }
            if ($functions.Count -gt 0) {
                $analysisContent += @"
**Functions:** $($functions -join ', ')

"@
            }
        }
    }
    
    # Analyze web interface
    if (Test-Path "web") {
        $analysisContent += @"

### Web Interface (web/):

"@
        
        $webFiles = Get-ChildItem "web\*" | Sort-Object Name
        foreach ($file in $webFiles) {
            $size = [math]::Round($file.Length / 1024, 2)
            $analysisContent += @"
- **$($file.Name)** - $size KB
"@
        }
    }
    
    # Commands and recommendations
    $analysisContent += @"

## Development Commands:

### Basic commands:
```bash
# Syntax check (ALLOWED)
cargo check

# CLI mode - document indexing
cargo run

# Web mode - start HTTP server  
cargo run web

# Code formatting
cargo fmt

# Linting
cargo clippy
```

### Restrictions:
- **FORBIDDEN** ``cargo run`` and ``cargo build`` for testing
- **ALLOWED** only ``cargo check`` for syntax validation
- User tests the project independently

## Architectural Features:

### Thread Safety:
- Using ``Arc`` and ``Mutex`` for shared access
- Async/await with Tokio runtime
- Atomic index operations

### Optimizations:
- Inverted index for O(1) search
- Incremental index updates
- GZIP compression for web resources

### Security:
- Input data validation
- Safe filesystem operations
- Thread-safe operations

## Project Statistics:

"@

    # File statistics
    $totalRustFiles = (Get-ChildItem "src\*.rs").Count
    $totalWebFiles = if (Test-Path "web") { (Get-ChildItem "web\*").Count } else { 0 }
    $cargoSize = if (Test-Path "Cargo.toml") { [math]::Round((Get-Item "Cargo.toml").Length / 1024, 2) } else { 0 }
    
    $analysisContent += @"
- **Rust files:** $totalRustFiles
- **Web files:** $totalWebFiles  
- **Cargo.toml size:** $cargoSize KB
- **Platform:** Windows (C:\Users\vladr\RUST\blazing_SEARCH)

## Development Recommendations:

1. **Before changes:** always run ``cargo check``
2. **Architecture:** follow modular structure
3. **Async code:** use Tokio patterns
4. **Security:** validate all input data
5. **Testing:** user tests independently

---

*Documentation generated locally by PowerShell script*  
*Date: $(Get-Date -Format 'dd.MM.yyyy HH:mm:ss')*
"@
    
    # Write analysis to temporary file
    $analysisContent | Out-File -FilePath $analysisFile -Encoding UTF8 -Force
    Write-ColorOutput "Project analysis completed!" "Green"
    
    return $analysisFile
}

# Backup CLAUDE.md
function Backup-ClaudeMd {
    if (-not $SkipBackup -and (Test-Path "CLAUDE.md")) {
        Write-ColorOutput "`nCreating backup of CLAUDE.md..." "Yellow"
        $backupName = "CLAUDE.md.backup.$(Get-Date -Format 'yyyy-MM-dd-HH-mm-ss')"
        Copy-Item "CLAUDE.md" $backupName -Force
        Write-ColorOutput "Backup created: $backupName" "Green"
        return $backupName
    } else {
        Write-ColorOutput "`nBackup skipped" "Cyan"
        return $null
    }
}

# Update CLAUDE.md
function Update-ClaudeMd {
    param([string]$AnalysisFile)
    
    Write-ColorOutput "`nUpdating CLAUDE.md file..." "Yellow"
    
    $analysisContent = Get-Content $AnalysisFile -Raw -Encoding UTF8
    
    $claudeContent = @"
# CLAUDE.md - blazing_SEARCH Documentation

$analysisContent

## Claude Code Integration Notes:

This file was updated by local PowerShell script to ensure up-to-date documentation for **blazing_SEARCH** project.

### Last Update:
- **Date:** $(Get-Date -Format 'dd.MM.yyyy HH:mm:ss')
- **Method:** Local PowerShell script
- **Analysis depth:** $AnalysisDepth
- **Computer:** $env:COMPUTERNAME
- **User:** $env:USERNAME

### Instructions for Claude Code:
1. **Communication language:** Ukrainian (according to user preferences)
2. **Main technology:** Rust + Actix-web  
3. **Forbidden:** DO NOT run ``cargo run`` or ``cargo build``
4. **Allowed:** Only ``cargo check`` for syntax validation
5. **Commits:** Create only when user writes "Super! ..."

### Local Update:
To update this file use:
```powershell
.\update-claude-md-local.ps1 -AnalysisDepth standard -Verbose
```

---
*Generated locally by PowerShell script update-claude-md-local.ps1*
"@
    
    # Write updated CLAUDE.md
    $claudeContent | Out-File -FilePath "CLAUDE.md" -Encoding UTF8 -Force
    
    Write-ColorOutput "CLAUDE.md successfully updated!" "Green"
    
    # Show new file size
    $newSize = [math]::Round((Get-Item "CLAUDE.md").Length / 1024, 2)
    Write-ColorOutput "New CLAUDE.md size: $newSize KB" "Cyan"
}

# Clean temporary files
function Clear-TempFiles {
    param([string[]]$Files)
    
    Write-ColorOutput "`nCleaning temporary files..." "Yellow"
    
    foreach ($file in $Files) {
        if ($file -and (Test-Path $file)) {
            Remove-Item $file -Force
            Write-ColorOutput "Removed: $file" "Gray"
        }
    }
    
    Write-ColorOutput "Cleanup completed!" "Green"
}

# Main function
function Main {
    Write-ColorOutput "`n============================================" "Green"
    Write-ColorOutput "   CLAUDE.MD LOCAL UPDATE SCRIPT" "Green"  
    Write-ColorOutput "============================================" "Green"
    Write-ColorOutput "`nParameters:" "Cyan"
    Write-ColorOutput "- Analysis depth: $AnalysisDepth" "White"
    Write-ColorOutput "- Skip backup: $SkipBackup" "White"
    Write-ColorOutput "- Verbose output: $Verbose" "White"
    
    $tempFiles = @()
    
    try {
        # Step 1: Check project
        Test-ProjectDirectory
        
        # Step 2: Check Rust syntax
        Test-RustSyntax
        
        # Step 3: Backup
        $backupFile = Backup-ClaudeMd
        if ($backupFile) { $tempFiles += $backupFile }
        
        # Step 4: Generate analysis
        $analysisFile = New-ProjectAnalysis
        $tempFiles += $analysisFile
        
        # Step 5: Update CLAUDE.md
        Update-ClaudeMd -AnalysisFile $analysisFile
        
        # Success
        Write-ColorOutput "`n============================================" "Green"
        Write-ColorOutput "   CLAUDE.MD UPDATED SUCCESSFULLY!" "Green"  
        Write-ColorOutput "============================================" "Green"
        Write-ColorOutput "`nCLAUDE.md file updated with current documentation" "Cyan"
        Write-ColorOutput "Review changes in CLAUDE.md file" "Magenta"
        
        if ($backupFile) {
            Write-ColorOutput "Backup saved: $backupFile" "Yellow"
        }
        
    }
    catch {
        Write-ColorOutput "`nScript execution error: $_" "Red"
        exit 1
    }
    finally {
        # Clean temporary files (except backup)
        $filesToClean = $tempFiles | Where-Object { $_ -notlike "CLAUDE.md.backup.*" }
        if ($filesToClean) {
            Clear-TempFiles -Files $filesToClean
        }
    }
}

# Run script
Main