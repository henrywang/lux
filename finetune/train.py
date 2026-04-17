#!/usr/bin/env python3
"""LoRA fine-tuning script for Qwen3 1.7B using unsloth.

Usage:
    pip install unsloth
    python finetune/train.py

Output:
    finetune/output/lux-qwen3-1.7b-lora/  — LoRA adapter weights
    finetune/output/lux-qwen3-1.7b-gguf/  — GGUF merged model for ollama
"""

import json
import os
from pathlib import Path
from datasets import Dataset
from unsloth import FastLanguageModel
from unsloth.chat_templates import get_chat_template
from trl import SFTTrainer, SFTConfig

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
BASE_MODEL = "unsloth/Qwen3-1.7B"
DATASET_PATH = Path(__file__).parent / "dataset.jsonl"

# When COLAB_OUTPUT is set, write to Google Drive so outputs survive session end.
# Example: COLAB_OUTPUT=/content/drive/MyDrive/lux-finetune
_out_base = Path(os.environ["COLAB_OUTPUT"]) if "COLAB_OUTPUT" in os.environ else Path(__file__).parent / "output"
OUTPUT_DIR = _out_base / "lux-qwen3-1.7b-lora"
GGUF_DIR = _out_base / "lux-qwen3-1.7b-gguf"

# LoRA hyperparameters
LORA_R = 16               # rank
LORA_ALPHA = 32            # scaling factor, typically 2x rank
LORA_DROPOUT = 0.05
TARGET_MODULES = [
    "q_proj", "k_proj", "v_proj", "o_proj",
    "gate_proj", "up_proj", "down_proj",
]

# Training hyperparameters
EPOCHS = 5
BATCH_SIZE = 2
GRAD_ACCUM = 4             # effective batch size = 2 * 4 = 8
LEARNING_RATE = 2e-4
MAX_SEQ_LEN = 2048
WARMUP_STEPS = 10
WEIGHT_DECAY = 0.01


# ---------------------------------------------------------------------------
# Load dataset
# ---------------------------------------------------------------------------
def load_dataset():
    """Load JSONL training data into HuggingFace Dataset."""
    examples = []
    with open(DATASET_PATH) as f:
        for line in f:
            examples.append(json.loads(line))
    return Dataset.from_list(examples)


# ---------------------------------------------------------------------------
# Format conversations for training
# ---------------------------------------------------------------------------
def format_conversation(example, tokenizer):
    """Apply chat template to a conversation example."""
    messages = example["messages"]

    # Convert tool_calls in assistant messages to the format the model expects
    formatted_messages = []
    for msg in messages:
        if msg["role"] == "assistant" and "tool_calls" in msg and msg["tool_calls"]:
            # Format tool calls as the assistant's content
            tool_calls = msg["tool_calls"]
            tool_text_parts = []
            for tc in tool_calls:
                func = tc["function"]
                tool_text_parts.append(json.dumps({
                    "name": func["name"],
                    "arguments": func["arguments"],
                }, ensure_ascii=False))
            formatted_messages.append({
                "role": "assistant",
                "content": "<tool_call>\n" + "\n".join(tool_text_parts) + "\n</tool_call>",
            })
        else:
            formatted_messages.append({
                "role": msg["role"],
                "content": msg.get("content", ""),
            })

    text = tokenizer.apply_chat_template(
        formatted_messages,
        tokenize=False,
        add_generation_prompt=False,
    )
    return {"text": text}


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main():
    print(f"Loading base model: {BASE_MODEL}")
    model, tokenizer = FastLanguageModel.from_pretrained(
        model_name=BASE_MODEL,
        max_seq_length=MAX_SEQ_LEN,
        load_in_4bit=True,          # QLoRA — 4-bit base model
        dtype=None,                  # auto-detect
    )

    # Apply chat template
    tokenizer = get_chat_template(tokenizer, chat_template="qwen-2.5")

    # Add LoRA adapters
    print("Adding LoRA adapters...")
    model = FastLanguageModel.get_peft_model(
        model,
        r=LORA_R,
        lora_alpha=LORA_ALPHA,
        lora_dropout=LORA_DROPOUT,
        target_modules=TARGET_MODULES,
        bias="none",
        use_gradient_checkpointing="unsloth",  # memory optimization
        random_state=42,
    )

    # Load and format dataset
    print(f"Loading dataset from {DATASET_PATH}")
    dataset = load_dataset()
    print(f"Loaded {len(dataset)} examples")

    dataset = dataset.map(
        lambda x: format_conversation(x, tokenizer),
        remove_columns=dataset.column_names,
    )

    # Training config
    training_args = SFTConfig(
        output_dir=str(OUTPUT_DIR),
        num_train_epochs=EPOCHS,
        per_device_train_batch_size=BATCH_SIZE,
        gradient_accumulation_steps=GRAD_ACCUM,
        learning_rate=LEARNING_RATE,
        warmup_steps=WARMUP_STEPS,
        weight_decay=WEIGHT_DECAY,
        fp16=True,
        logging_steps=5,
        save_strategy="epoch",
        seed=42,
        max_seq_length=MAX_SEQ_LEN,
        dataset_text_field="text",
        packing=False,              # packing corrupts tool-call format across examples
    )

    # Trainer
    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        args=training_args,
    )

    # Train
    print("Starting training...")
    stats = trainer.train()
    print(f"Training complete. Loss: {stats.training_loss:.4f}")

    # Save LoRA adapter
    print(f"Saving LoRA adapter to {OUTPUT_DIR}")
    model.save_pretrained(str(OUTPUT_DIR))
    tokenizer.save_pretrained(str(OUTPUT_DIR))

    # Export to GGUF for ollama
    print(f"Exporting merged GGUF to {GGUF_DIR}")
    model.save_pretrained_gguf(
        str(GGUF_DIR),
        tokenizer,
        quantization_method="q4_k_m",
    )

    print("\nDone! To use with ollama:")
    print(f"  ollama create lux-qwen3 -f {GGUF_DIR}/Modelfile")
    print("  python bench/run_bench.py lux-qwen3")


if __name__ == "__main__":
    main()
