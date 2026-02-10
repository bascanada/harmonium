import sys
import os
import importlib

# Force TensorFlow to use Keras 2 compatibility mode
os.environ["TF_USE_LEGACY_KERAS"] = "1"

# Prevent oemer from trying to set None to environment variable
if "INFERENCE_WITH_TF" not in os.environ:
    os.environ["INFERENCE_WITH_TF"] = "0"

# Monkeypatch onnxruntime before oemer imports it
try:
    import onnxruntime as rt
    
    # Save the original method
    original_get_available_providers = rt.get_available_providers
    
    def patched_get_available_providers():
        providers = original_get_available_providers()
        # Remove CoreML to avoid the "Operation not permitted" crashes on macOS
        if 'CoreMLExecutionProvider' in providers:
            providers.remove('CoreMLExecutionProvider')
        return providers

    # Apply the patch
    rt.get_available_providers = patched_get_available_providers
    
    # Also disable CoreML via environment just in case
    os.environ["ORT_COREML_FLAGS"] = "0"
    
except ImportError:
    pass

# Now import and run oemer
from oemer.ete import main

if __name__ == "__main__":
    sys.exit(main())
