#!/bin/bash
# Build WASM et lancer le serveur de développement

echo "🔧 Building WASM..."
wasm-pack build --target web --out-dir pkg

if [ $? -ne 0 ]; then
    echo "❌ WASM build failed!"
    exit 1
fi

echo "✅ WASM build successful!"
echo ""
echo "🌐 Starting development server..."
echo "   Open http://localhost:5173 in your browser"
echo ""

cd web && npm run dev
