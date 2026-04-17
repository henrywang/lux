#!/bin/bash
# Create an ollama model from the fine-tuned GGUF output.
#
# Usage:
#   bash finetune/create_ollama_model.sh
#
# Then test:
#   python bench/run_bench.py lux-qwen3

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GGUF_DIR="$SCRIPT_DIR/output/lux-qwen3-1.7b-gguf"

# Find the GGUF file
GGUF_FILE=$(find "$GGUF_DIR" -name "*.gguf" -type f | head -1)

if [ -z "$GGUF_FILE" ]; then
    echo "Error: No GGUF file found in $GGUF_DIR"
    echo "Run 'python finetune/train.py' first."
    exit 1
fi

echo "Found GGUF: $GGUF_FILE"

# Create Modelfile for ollama
cat > "$GGUF_DIR/Modelfile" <<EOF
FROM $GGUF_FILE

PARAMETER temperature 0.1
PARAMETER num_ctx 4096
PARAMETER stop <|im_end|>

SYSTEM """You are lux, an AI system agent for Linux desktop. You help users manage their system by calling tools. Always call a tool — never respond with only text."""
EOF

echo "Creating ollama model 'lux-qwen3'..."
ollama create lux-qwen3 -f "$GGUF_DIR/Modelfile"

echo ""
echo "Done! Test with:"
echo "  python bench/run_bench.py lux-qwen3"
