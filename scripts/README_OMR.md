# Harmonium OMR (Optical Music Recognition) Tool

This tool allows you to convert PDF songbooks or image-based music sheets into MusicXML and the Harmonium "Truth" format (JSON).

## Setup

1. Run the setup script to create a virtual environment and install dependencies:
   ```bash
   ./scripts/setup_omr.sh
   ```
   *Note: This will install heavy dependencies like TensorFlow and OpenCv.*

2. Activate the environment:
   ```bash
   source .venv-omr/bin/activate
   ```

## Usage

### Basic Usage
To parse a PDF and extract carols:
```bash
python scripts/sheet_to_truth.py path/to/book.pdf --output ./my_carols
```

### Options
- `--output`, `-o`: Directory where final .musicxml and .truth.json files will be saved.
- `--temp`, `-t`: Directory for intermediate files (burst images, raw OMR output).
- `--skip-omr`: If you already ran Oemer and just want to re-run the splitting/truth-conversion logic on the XMLs in the temp directory.

## How it works
1. **Bursting**: If the input is a PDF, it's converted to high-resolution PNG images.
2. **OMR (Oemer)**: Each image is processed by the Oemer OMR engine to produce a MusicXML file.
3. **Splitting**: The tool looks for 'final' or 'double' barlines in the music. This allows it to extract multiple short songs (like carols) from a single page or a continuous book.
4. **Truth Conversion**: Each split song is converted into the Harmonium `RecordingTruth` format, mapping notes to `NoteOn`/`NoteOff` events with step-based timestamps.

## Accuracy
OMR is rarely 100% perfect. You may need to open the resulting `.musicxml` files in MuseScore or Finale to correct pitches or rhythms before using them in the Harmonium engine.
