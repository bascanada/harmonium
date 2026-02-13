#!/bin/bash
# setup_omr.sh - Setup environment for sheet music parsing

set -e

# Change to script directory if needed, but we assume run from project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VENV_DIR="$PROJECT_ROOT/.venv-omr"

echo "=== Harmonium OMR Setup ==="

if [[ "$OSTYPE" == "darwin"* ]]; then
    if ! command -v brew &> /dev/null; then
        echo "Warning: Homebrew not found. Please install poppler manually if PDF conversion fails."
    else
        echo "Checking for poppler (required for pdf2image)..."
        if ! brew list poppler &> /dev/null; then
            echo "Installing poppler via brew..."
            brew install poppler
        fi
    fi
fi

echo "Creating virtual environment in $VENV_DIR..."
# Use Python 3.12 specifically - TensorFlow doesn't support Python 3.13+ yet
if ! command -v python3.12 &> /dev/null; then
    echo "Error: Python 3.12 is required but not found."
    echo "Install with: brew install python@3.12"
    exit 1
fi
python3.12 -m venv "$VENV_DIR"

echo "Activating virtual environment..."
source "$VENV_DIR/bin/activate"

echo "Upgrading pip..."
pip install --upgrade pip

echo "Installing dependencies (this might take a while, includes TensorFlow)..."
# oemer depends on tensorflow, opencv-python, etc.
# We add tensorflow specifically for the --use-tf fallback
pip install oemer music21 pdf2image tensorflow

echo "=== Setup Complete ==="
echo ""
echo "To use the script:"
echo "1. Activate the environment: source .venv-omr/bin/activate"
echo "2. Run the parser: python scripts/sheet_to_truth.py path/to/your/book.pdf"
echo ""
echo "Note: The first time you run Oemer, it will download its ML models (~500MB)."
