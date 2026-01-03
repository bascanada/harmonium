import cv2
from PIL import Image
from transformers import CLIPProcessor, CLIPModel
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

    # 2. Charger le modèle AI (CLIP)
    print("Loading CLIP model...")
    model_id = "openai/clip-vit-base-patch32"
    model = CLIPModel.from_pretrained(model_id)
    processor = CLIPProcessor.from_pretrained(model_id)
    print("Model loaded.")

    # Les "états" possibles (Glossaire)
    # Ces labels seront envoyés à Rust qui utilisera son propre modèle (BERT) pour déduire les paramètres.
    labels = [
        "peaceful forest",
        "intense combat",
        "scary dark cave",
        "victory celebration",
        "sad rain",
        "cyberpunk city"
    ]

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
            # Simulate real-time playback roughly if needed, or just process as fast as possible
            # time.sleep(1/30) 
            continue

        # Convertir BGR (OpenCV) vers RGB (PIL)
        image = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        pil_image = Image.fromarray(image)

        # 4. Analyser l'image avec l'AI
        inputs = processor(text=labels, images=pil_image, return_tensors="pt", padding=True)
        outputs = model(**inputs)
        probs = outputs.logits_per_image.softmax(dim=1)

        # Trouver le label avec le plus haut score
        best_idx = torch.argmax(probs).item()
        best_label = labels[best_idx]
        confidence = probs[0][best_idx].item()

        print(f"Detected: {best_label} ({confidence:.2f})")

        # 5. Envoyer à Rust via OSC
        # On envoie UNIQUEMENT le label. Rust fera le mapping sémantique.
        client.send_message("/harmonium/label", best_label)

        # Petit délai pour ne pas spammer si la vidéo est lue très vite
        time.sleep(0.1)

    cap.release()
    print("Done.")

if __name__ == "__main__":
    main()
