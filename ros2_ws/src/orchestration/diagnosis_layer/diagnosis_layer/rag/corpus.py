from __future__ import annotations

import os
import re
from dataclasses import dataclass
from typing import List

_CHUNK_OVERLAP_LINES = 2


@dataclass
class Chunk:
    text: str
    source: str
    section: str


def load_corpus(corpus_dir: str) -> List[Chunk]:
    """Load all markdown/text files under ``corpus_dir`` and split into chunks.

    Returns an empty list when the directory is missing or empty so that the
    diagnosis layer can still run with RAG disabled (RFC-009 risk 3).
    """
    chunks: List[Chunk] = []
    if not corpus_dir or not os.path.isdir(corpus_dir):
        return chunks
    for root, _dirs, files in os.walk(corpus_dir):
        for fn in sorted(files):
            if not fn.lower().endswith((".md", ".markdown", ".txt")):
                continue
            path = os.path.join(root, fn)
            try:
                with open(path, "r", encoding="utf-8") as fh:
                    content = fh.read()
            except (OSError, UnicodeDecodeError):
                continue
            chunks.extend(_chunk_file(content, source=fn))
    return chunks


def _chunk_file(content: str, source: str) -> List[Chunk]:
    chunks: List[Chunk] = []
    section = ""
    paragraph: List[str] = []

    def flush() -> None:
        nonlocal paragraph
        if paragraph:
            text = "\n".join(paragraph).strip()
            if text:
                chunks.append(Chunk(text=text, source=source, section=section))
            paragraph = []

    for line in content.splitlines():
        heading = re.match(r"^#{1,6}\s+(.*)$", line.strip())
        if heading:
            flush()
            section = heading.group(1).strip()
            continue
        if line.strip() == "":
            flush()
        else:
            paragraph.append(line)
    flush()
    return chunks
