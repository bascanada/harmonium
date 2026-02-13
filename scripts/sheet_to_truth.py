import os
import subprocess
import json
import argparse
import sys
from pathlib import Path

# --- macOS / ML Runtime Compatibility ---
# Force CPU for TensorFlow to avoid CUDA warnings on Mac
os.environ["CUDA_VISIBLE_DEVICES"] = "-1"
# Suppress ONNX Runtime noisy cleanup warnings on macOS
os.environ["ONNXRUNTIME_QUIET"] = "1"
# ----------------------------------------

# Try to import music21, but don't fail yet so we can show help
try:
    from music21 import converter, stream, bar, tempo, key, note, chord
    HAS_MUSIC21 = True
except ImportError:
    HAS_MUSIC21 = False

def musicxml_to_truth(score, bpm=120, sample_rate=44100):
    """
    Converts a music21 score to the Harmonium RecordingTruth format (JSON).
    """
    events = []
    
    # Extract BPM if present
    mm = score.flat.getElementsByClass(tempo.MetronomeMark)
    if mm:
        bpm = mm[0].number

    # Extract Key if present
    ks = score.flat.getElementsByClass(key.KeySignature)
    key_root = 0
    if ks:
        key_root = ks[0].tonic.pitchClass

    # Default MusicalParams (matching Rust defaults)
    params = {
        "bpm": float(bpm),
        "master_volume": 1.0,
        "enable_rhythm": True,
        "enable_harmony": True,
        "enable_melody": True,
        "enable_voicing": False,
        "rhythm_mode": "Euclidean",
        "rhythm_steps": 16,
        "rhythm_pulses": 4,
        "rhythm_rotation": 0,
        "rhythm_density": 0.5,
        "rhythm_tension": 0.3,
        "rhythm_secondary_steps": 12,
        "rhythm_secondary_pulses": 3,
        "rhythm_secondary_rotation": 0,
        "fixed_kick": False,
        "harmony_mode": "Driver",
        "harmony_strategy": "Auto",
        "harmony_tension": 0.3,
        "harmony_valence": 0.3,
        "harmony_measures_per_chord": 2,
        "key_root": key_root,
        "melody_smoothness": 0.7,
        "voicing_density": 0.5,
        "voicing_tension": 0.3,
        "melody_octave": 4,
        "gain_lead": 1.0,
        "gain_bass": 0.6,
        "gain_snare": 0.5,
        "gain_hat": 0.4,
        "vel_base_bass": 85,
        "vel_base_snare": 70,
        "channel_routing": [-1] * 16,
        "muted_channels": [False] * 16,
        "record_wav": False,
        "record_midi": False,
        "record_musicxml": False,
        "record_truth": False
    }

    # Iterate through notes and chords
    # offset is in quarter notes, which we map to "steps" in the engine
    for el in score.flat.notes:
        offset_steps = float(el.offset)
        duration_steps = float(el.duration.quarterLength)
        
        notes_to_play = []
        if isinstance(el, chord.Chord):
            notes_to_play = [p.midi for p in el.pitches]
        else:
            notes_to_play = [el.pitch.midi]
            
        for n in notes_to_play:
            # Note: channel 0 for lead/melody by default
            events.append((offset_steps, {"NoteOn": {"note": n, "velocity": 90, "channel": 0}}))
            events.append((offset_steps + duration_steps, {"NoteOff": {"note": n, "channel": 0}}))

    # Sort events by timestamp
    events.sort(key=lambda x: x[0])

    truth = {
        "version": "0.1.0-omr",
        "git_sha": "unknown",
        "params": params,
        "events": events,
        "sample_rate": sample_rate
    }
    return truth

def split_and_convert(xml_dir, output_dir):
    """
    Reads MusicXML files, splits them by 'final' barlines if multiple songs 
    are on one page, and saves them as .musicxml and .truth.json.
    """
    xml_files = sorted([f for f in os.listdir(xml_dir) if f.endswith('.musicxml')])
    
    carol_count = 1
    current_carol = stream.Score()
    
    # We'll use a simple heuristic: if we see a final barline, we finish the carol.
    # Note: Oemer output might put everything in one part.
    
    for xml_file in xml_files:
        print(f"Splitting/Converting: {xml_file}")
        score = converter.parse(os.path.join(xml_dir, xml_file))
        
        # Check if score has parts, otherwise use flat
        parts = score.parts if len(score.parts) > 0 else [score]
        
        for p in parts:
            for m in p.getElementsByClass(stream.Measure):
                current_carol.append(m)
                
                # Check for the end of a song (Double/Final Barline)
                is_final = False
                if m.rightBarline and m.rightBarline.type in ['final', 'double']:
                    is_final = True
                
                if is_final:
                    save_carol(current_carol, carol_count, output_dir)
                    current_carol = stream.Score()
                    carol_count += 1
    
    # Save anything remaining
    if len(current_carol.flat.notes) > 0:
        save_carol(current_carol, carol_count, output_dir)

def save_carol(score, count, output_dir):
    name = f"carol_{count:03d}"
    xml_path = os.path.join(output_dir, f"{name}.musicxml")
    truth_path = os.path.join(output_dir, f"{name}.truth.json")
    
    print(f"  Saving {name}...")
    
    # Save MusicXML
    score.write('musicxml', fp=xml_path)
    
    # Save Truth JSON
    truth_data = musicxml_to_truth(score)
    with open(truth_path, 'w') as f:
        json.dump(truth_data, f, indent=2)

def main():
    parser = argparse.ArgumentParser(description="Parse PDF/Image music sheets to MusicXML and Harmonium Truth format.")
    parser.add_argument("input", help="Path to PDF file or directory of images")
    parser.add_argument("--output", "-o", default="./output_carols", help="Output directory")
    parser.add_argument("--temp", "-t", default="./temp_omr", help="Temporary directory for processing")
    parser.add_argument("--skip-omr", action="store_true", help="Skip Oemer OMR step (use existing XMLs in temp)")
    
    args = parser.parse_args()

    if not HAS_MUSIC21:
        print("Error: music21 not found. Please run the setup script first.")
        sys.exit(1)

    input_path = Path(args.input)
    output_dir = Path(args.output)
    temp_dir = Path(args.temp)
    
    output_dir.mkdir(parents=True, exist_ok=True)
    temp_dir.mkdir(parents=True, exist_ok=True)
    
    img_dir = temp_dir / "images"
    xml_dir = temp_dir / "xml"
    img_dir.mkdir(exist_ok=True)
    xml_dir.mkdir(exist_ok=True)

    if not args.skip_omr:
        # 1. Burst PDF to Images if needed
        if input_path.is_file() and input_path.suffix.lower() == ".pdf":
            print(f"Bursting PDF: {input_path}")
            try:
                from pdf2image import convert_from_path
                images = convert_from_path(str(input_path), dpi=300)
                for i, image in enumerate(images):
                    image.save(img_dir / f"page_{i:03d}.png", "PNG")
            except ImportError:
                print("Error: pdf2image not found. Install with: pip install pdf2image")
                sys.exit(1)
        elif input_path.is_dir():
            print(f"Using images from: {input_path}")
            img_dir = input_path
        else:
            print(f"Error: Invalid input path {input_path}")
            sys.exit(1)
        
        # Check if tensorflow is available for the --use-tf flag
        has_tf = False
        try:
            import tensorflow
            has_tf = True
        except ImportError:
            pass

        # 2. Run Oemer
        print("Running Oemer OMR (with macOS stability wrapper)...")
        images = sorted([f for f in os.listdir(img_dir) if f.endswith(('.png', '.jpg', '.jpeg'))])
        for img in images:
            img_path = Path(img_dir) / img
            print(f"  Processing {img}...")
            
            # Use our wrapper to disable CoreML and avoid crashes
            wrapper_path = Path(__file__).parent / "oemer_wrapper.py"
            cmd = [sys.executable, str(wrapper_path), str(img_path), "-o", str(xml_dir), "-d"]
            
            if has_tf:
                print("    (Using TensorFlow backend)")
                cmd.append("--use-tf")
            else:
                print("    (Using ONNX CPU backend - CoreML patched out)")

            try:
                subprocess.run(cmd, check=True)
            except subprocess.CalledProcessError as e:
                print(f"Error processing {img}: {e}")
                sys.exit(1)

    # 3. Split and Convert to Truth
    print("Splitting and converting to Truth format...")
    split_and_convert(xml_dir, output_dir)
    print(f"Done! Output saved to {output_dir}")

if __name__ == "__main__":
    main()
