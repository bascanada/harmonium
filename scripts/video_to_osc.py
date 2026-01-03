import cv2
from PIL import Image
from transformers import BlipProcessor, BlipForConditionalGeneration
from pythonosc import udp_client
import torch
import time
import argparse

def main():
    parser = argparse.ArgumentParser(description="Video to OSC for Harmonium")
    parser.add_argument("--video", type=str, default="demo.mp4", help="Path to video file")
    parser.add_argument("--ip", type=str, default="127.0.0.1", help="OSC Server IP")
    parser.add_argument("--port", type=int, default=8080, help="OSC Server Port")
    args = parser.parse_args()

    # 1. Configurer le client OSC
    client = udp_client.SimpleUDPClient(args.ip, args.port)
    print(f"OSC Client sending to {args.ip}:{args.port}")

    # 2. Charger le modèle AI (BLIP Image Captioning)
    print("Loading BLIP model (Image Captioning)...")
    model_id = "Salesforce/blip-image-captioning-base"
    processor = BlipProcessor.from_pretrained(model_id)
    model = BlipForConditionalGeneration.from_pretrained(model_id)
    print("Model loaded.")

    # 3. Lire la vidéo
    cap = cv2.VideoCapture(args.video)
    if not cap.isOpened():
        print(f"Error: Could not open video {args.video}")
        return

    frame_count = 0
    skip_frames = 30  # Analyse toutes les 30 frames

    print("Starting analysis...")

    while cap.isOpened():
        ret, frame = cap.read()
        if not ret:
            break

        frame_count += 1
        if frame_count % skip_frames != 0:
            continue

        # Convertir BGR (OpenCV) vers RGB (PIL)
        image = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        pil_image = Image.fromarray(image)

        # 4. Analyser l'image avec l'AI (Génération de description)
        inputs = processor(pil_image, return_tensors="pt")
        
        # Générer la caption
        out = model.generate(**inputs, max_new_tokens=20)
        caption = processor.decode(out[0], skip_special_tokens=True)

        print(f"Generated: {caption}")

        # 5. Envoyer à Rust via OSC
        # On envoie la description générée. Rust fera le mapping sémantique.
        client.send_message("/harmonium/label", caption)

        # Petit délai pour ne pas spammer si la vidéo est lue très vite
        # Augmenté à 1.0s pour éviter de surcharger le moteur audio Rust (BERT inference)
        time.sleep(1.0)

    cap.release()
    print("Done.")

if __name__ == "__main__":
    main()
