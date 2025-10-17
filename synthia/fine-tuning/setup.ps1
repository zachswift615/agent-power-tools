# ==============================================================================
# Synthia Fine-Tuning Setup Script for Windows
# Optimized for RTX 4060 (8GB VRAM)
#
# This script:
# 1. Checks system requirements (Python, CUDA)
# 2. Creates virtual environment
# 3. Installs PyTorch with CUDA 12.1 support
# 4. Installs Unsloth and all dependencies
# 5. Verifies installation
#
# Requirements:
# - Windows 10/11
# - Python 3.10 or 3.11 (3.12 not fully supported yet)
# - NVIDIA GPU with CUDA support (RTX 4060)
# - ~20GB free disk space
# ==============================================================================

# Set error action preference
$ErrorActionPreference = "Stop"

# Color output functions
function Write-Header {
    param($Message)
    Write-Host "`n================================================================================" -ForegroundColor Cyan
    Write-Host "  $Message" -ForegroundColor Cyan
    Write-Host "================================================================================`n" -ForegroundColor Cyan
}

function Write-Success {
    param($Message)
    Write-Host "[OK] $Message" -ForegroundColor Green
}

function Write-Error-Custom {
    param($Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Write-Warning-Custom {
    param($Message)
    Write-Host "[WARNING] $Message" -ForegroundColor Yellow
}

function Write-Info {
    param($Message)
    Write-Host "[INFO] $Message" -ForegroundColor White
}

# ==============================================================================
# Step 1: Check Python Version
# ==============================================================================

Write-Header "Checking Python Installation"

try {
    $pythonVersion = python --version 2>&1
    Write-Info "Found: $pythonVersion"

    # Extract version number
    if ($pythonVersion -match "Python (\d+)\.(\d+)\.(\d+)") {
        $major = [int]$matches[1]
        $minor = [int]$matches[2]

        if ($major -ne 3) {
            Write-Error-Custom "Python 3 is required. Found Python $major"
            exit 1
        }

        if ($minor -lt 10 -or $minor -gt 11) {
            Write-Warning-Custom "Python 3.10 or 3.11 is recommended. You have Python 3.$minor"
            Write-Warning-Custom "Some packages may not work correctly with Python 3.$minor"
            Write-Info "Continue anyway? (Y/N)"
            $response = Read-Host
            if ($response -ne "Y" -and $response -ne "y") {
                Write-Info "Please install Python 3.10 or 3.11 from https://www.python.org/downloads/"
                exit 1
            }
        }

        Write-Success "Python version is compatible (3.$minor)"
    }
}
catch {
    Write-Error-Custom "Python not found in PATH"
    Write-Info "Please install Python 3.10 or 3.11 from https://www.python.org/downloads/"
    Write-Info "Make sure to check 'Add Python to PATH' during installation"
    exit 1
}

# ==============================================================================
# Step 2: Check CUDA Installation
# ==============================================================================

Write-Header "Checking CUDA Installation"

try {
    $nvidiaSmi = nvidia-smi 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Success "NVIDIA GPU detected"

        # Extract GPU name
        if ($nvidiaSmi -match "NVIDIA GeForce.*") {
            $gpuName = $matches[0]
            Write-Info "GPU: $gpuName"
        }

        # Extract CUDA version
        if ($nvidiaSmi -match "CUDA Version: (\d+\.\d+)") {
            $cudaVersion = $matches[1]
            Write-Info "CUDA Version: $cudaVersion"

            $cudaMajor = [int]$cudaVersion.Split('.')[0]
            if ($cudaMajor -lt 12) {
                Write-Warning-Custom "CUDA 12.1 or higher is recommended for best performance"
                Write-Warning-Custom "You have CUDA $cudaVersion"
            }
            else {
                Write-Success "CUDA version is compatible"
            }
        }
    }
}
catch {
    Write-Error-Custom "NVIDIA GPU driver not found"
    Write-Info "Please install the latest NVIDIA drivers from:"
    Write-Info "https://www.nvidia.com/Download/index.aspx"
    exit 1
}

# ==============================================================================
# Step 3: Create Virtual Environment
# ==============================================================================

Write-Header "Creating Virtual Environment"

$venvPath = "venv"

if (Test-Path $venvPath) {
    Write-Warning-Custom "Virtual environment already exists at: $venvPath"
    Write-Info "Delete and recreate? (Y/N)"
    $response = Read-Host
    if ($response -eq "Y" -or $response -eq "y") {
        Write-Info "Deleting existing virtual environment..."
        Remove-Item -Recurse -Force $venvPath
        Write-Success "Deleted"
    }
    else {
        Write-Info "Using existing virtual environment"
    }
}

if (-not (Test-Path $venvPath)) {
    Write-Info "Creating virtual environment..."
    python -m venv $venvPath
    Write-Success "Virtual environment created at: $venvPath"
}

# ==============================================================================
# Step 4: Activate Virtual Environment
# ==============================================================================

Write-Header "Activating Virtual Environment"

$activateScript = Join-Path $venvPath "Scripts\Activate.ps1"

if (-not (Test-Path $activateScript)) {
    Write-Error-Custom "Virtual environment activation script not found"
    exit 1
}

Write-Info "Activating virtual environment..."
& $activateScript

Write-Success "Virtual environment activated"

# Verify activation
$pythonPath = (Get-Command python).Source
if ($pythonPath -like "*$venvPath*") {
    Write-Success "Using virtual environment Python: $pythonPath"
}
else {
    Write-Warning-Custom "Virtual environment may not be activated correctly"
}

# ==============================================================================
# Step 5: Upgrade pip
# ==============================================================================

Write-Header "Upgrading pip"

Write-Info "Upgrading pip to latest version..."
python -m pip install --upgrade pip

Write-Success "pip upgraded"

# ==============================================================================
# Step 6: Install PyTorch with CUDA 12.1
# ==============================================================================

Write-Header "Installing PyTorch with CUDA 12.1"

Write-Info "This will download ~2GB of packages..."
Write-Info "Installing PyTorch, torchvision, torchaudio..."

# Install PyTorch with CUDA 12.1 support
python -m pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121

if ($LASTEXITCODE -ne 0) {
    Write-Error-Custom "Failed to install PyTorch"
    exit 1
}

Write-Success "PyTorch installed"

# ==============================================================================
# Step 7: Verify PyTorch CUDA Support
# ==============================================================================

Write-Header "Verifying PyTorch CUDA Support"

$pythonCheck = @"
import torch
print(f'PyTorch version: {torch.__version__}')
print(f'CUDA available: {torch.cuda.is_available()}')
if torch.cuda.is_available():
    print(f'CUDA device: {torch.cuda.get_device_name(0)}')
    print(f'CUDA version: {torch.version.cuda}')
"@

$checkResult = python -c $pythonCheck

Write-Info $checkResult

if ($checkResult -like "*CUDA available: False*") {
    Write-Error-Custom "PyTorch CUDA support not working"
    Write-Info "This could mean:"
    Write-Info "1. NVIDIA drivers are not installed"
    Write-Info "2. CUDA toolkit version mismatch"
    Write-Info "3. GPU not compatible with CUDA"
    exit 1
}

Write-Success "PyTorch CUDA support verified"

# ==============================================================================
# Step 8: Install Unsloth and Dependencies
# ==============================================================================

Write-Header "Installing Unsloth and Dependencies"

Write-Info "This will download ~3GB of packages..."
Write-Info "Installing from requirements.txt..."

# Install from requirements.txt
if (Test-Path "requirements.txt") {
    python -m pip install -r requirements.txt
}
else {
    Write-Error-Custom "requirements.txt not found"
    Write-Info "Please create requirements.txt first"
    exit 1
}

if ($LASTEXITCODE -ne 0) {
    Write-Error-Custom "Failed to install dependencies"
    exit 1
}

Write-Success "All dependencies installed"

# ==============================================================================
# Step 9: Verify Unsloth Installation
# ==============================================================================

Write-Header "Verifying Unsloth Installation"

$unslothCheck = @"
try:
    from unsloth import FastLanguageModel
    print('OK: Unsloth imported successfully')
except Exception as e:
    print(f'ERROR: {e}')
"@

$unslothResult = python -c $unslothCheck

Write-Info $unslothResult

if ($unslothResult -like "*ERROR*") {
    Write-Error-Custom "Unsloth installation failed"
    exit 1
}

Write-Success "Unsloth installation verified"

# ==============================================================================
# Step 10: Check Dataset
# ==============================================================================

Write-Header "Checking Dataset"

if (Test-Path "dataset.jsonl") {
    $datasetSize = (Get-Item "dataset.jsonl").Length / 1MB
    Write-Success "Dataset found: dataset.jsonl ($([math]::Round($datasetSize, 2)) MB)"

    # Count lines
    $lineCount = (Get-Content "dataset.jsonl" | Measure-Object -Line).Lines
    Write-Info "Dataset contains $lineCount examples"
}
else {
    Write-Warning-Custom "dataset.jsonl not found"
    Write-Info "Please run generate_dataset.py first to create training data"
}

# ==============================================================================
# Step 11: Summary and Next Steps
# ==============================================================================

Write-Header "Setup Complete!"

Write-Success "Environment successfully configured for fine-tuning on RTX 4060"

Write-Host "`nSystem Summary:" -ForegroundColor Cyan
Write-Host "  - Python: $pythonVersion" -ForegroundColor White
Write-Host "  - PyTorch: Installed with CUDA support" -ForegroundColor White
Write-Host "  - Unsloth: Installed and verified" -ForegroundColor White
Write-Host "  - Virtual environment: $venvPath" -ForegroundColor White

Write-Host "`nNext Steps:" -ForegroundColor Cyan
Write-Host "  1. Ensure dataset.jsonl exists (run generate_dataset.py if needed)" -ForegroundColor White
Write-Host "  2. Review training config in train.py" -ForegroundColor White
Write-Host "  3. Start training: python train.py" -ForegroundColor White
Write-Host "  4. Wait ~1-2 hours for training to complete" -ForegroundColor White
Write-Host "  5. Merge and export: python merge_and_export.py" -ForegroundColor White
Write-Host "  6. Test the model: python test_model.py" -ForegroundColor White

Write-Host "`nUseful Commands:" -ForegroundColor Cyan
Write-Host "  - Activate environment: .\venv\Scripts\Activate.ps1" -ForegroundColor White
Write-Host "  - Deactivate environment: deactivate" -ForegroundColor White
Write-Host "  - Monitor GPU usage: nvidia-smi" -ForegroundColor White
Write-Host "  - Watch GPU in real-time: watch -n 1 nvidia-smi" -ForegroundColor White

Write-Host "`nEstimated Requirements:" -ForegroundColor Cyan
Write-Host "  - VRAM usage: ~6-7GB peak" -ForegroundColor White
Write-Host "  - Disk space: ~20GB (model + checkpoints)" -ForegroundColor White
Write-Host "  - Training time: ~1-2 hours on RTX 4060" -ForegroundColor White

Write-Host ""
Write-Success "Happy fine-tuning!"
