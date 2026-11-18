"""Retrieval over a replica directory with LlamaIndex: every Markdown file (pages, sources, recon map, architecture,
build log) embedded by a local Hugging Face model on the GPU when PyTorch sees one, persisted in <dir>/index.

The embedding model is REPLICA_EMBED_MODEL, else the Hub's most downloaded sentence-similarity model for
sentence-transformers, resolved when the index is built and recorded with it."""

from __future__ import annotations

import json
import os
import urllib.request
from pathlib import Path


def embed_model_id() -> str:
    if os.environ.get("REPLICA_EMBED_MODEL"):
        return os.environ["REPLICA_EMBED_MODEL"]
    hub = os.environ.get("HF_ENDPOINT", "https://huggingface.co").rstrip("/")
    url = f"{hub}/api/models?pipeline_tag=sentence-similarity&library=sentence-transformers&sort=downloads&limit=1"
    with urllib.request.urlopen(url, timeout=30) as r:
        return json.load(r)[0]["id"]


def _settings(model: str) -> str:
    import torch
    from llama_index.core import Settings
    from llama_index.embeddings.huggingface import HuggingFaceEmbedding

    device = "cuda" if torch.cuda.is_available() else "cpu"
    Settings.embed_model = HuggingFaceEmbedding(model_name=model, device=device)
    Settings.llm = None  # retrieval only: the agent stages write the answers
    return device


def build(d: Path) -> dict:
    from llama_index.core import SimpleDirectoryReader, VectorStoreIndex

    model = embed_model_id()
    device = _settings(model)
    files = [p for p in d.rglob("*.md") if "index" not in p.relative_to(d).parts]
    docs = SimpleDirectoryReader(input_files=[str(p) for p in files]).load_data() if files else []
    index = VectorStoreIndex.from_documents(docs)
    index.storage_context.persist(str(d / "index"))
    meta = {"model": model, "device": device, "documents": len(docs), "files": len(files)}
    (d / "index" / "model.json").write_text(json.dumps(meta), encoding="utf-8")
    return meta


def query(d: Path, question: str, k: int) -> list[dict]:
    from llama_index.core import StorageContext, load_index_from_storage

    meta = json.loads((d / "index" / "model.json").read_text(encoding="utf-8"))
    _settings(meta["model"])
    index = load_index_from_storage(StorageContext.from_defaults(persist_dir=str(d / "index")))
    nodes = index.as_retriever(similarity_top_k=k).retrieve(question)
    return [{"score": round(float(n.score or 0), 4), "file": n.metadata.get("file_path"), "text": n.get_content()[:1500]} for n in nodes]
