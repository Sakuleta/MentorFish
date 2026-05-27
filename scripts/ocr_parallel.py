"""
Parallel OCR for ALL scanned chess PDFs.
Each worker processes one PDF and writes its own output file.
Then all outputs are merged into chunks_ocr.json.

Usage:
    python scripts/ocr_parallel.py knowledge/books/
    python scripts/ocr_parallel.py knowledge/books/ --workers 8
"""

import argparse
import io
import json
import os
import re
import sys
from datetime import datetime, timezone
from multiprocessing import Pool, cpu_count
from pathlib import Path


def classify_chunk(text):
    t = text.lower()
    if any(
        kw in t
        for kw in [
            "lucena",
            "philidor",
            "opposition",
            "triangulation",
            "zugzwang",
            "fortress",
            "theoretical draw",
            "tablebase",
            "rook ending",
        ]
    ):
        return "endgame_technique"
    if any(
        kw in t
        for kw in [
            "sicilian",
            "ruy lopez",
            "french defence",
            "queen's gambit",
            "king's indian",
            "nimzo-indian",
            "caro-kann",
            "slav",
            "1.e4",
            "1.d4",
        ]
    ):
        return "opening"
    if any(
        kw in t
        for kw in [
            "fork",
            "pin",
            "skewer",
            "discovered attack",
            "deflection",
            "sacrifice",
            "combination",
        ]
    ):
        return "motif"
    if re.search(r"\d+\.\s*[A-Za-z0-9]", text):
        return "instructive_example"
    return "concept"


def process_pdf(args_tuple):
    """Process ONE PDF. Returns (source_title, chunk_count, pages, error_or_chunks_json_path)."""
    pdf_path, source_title, out_dir = args_tuple

    try:
        import fitz
        import pytesseract
        from PIL import Image
    except ImportError as e:
        return (source_title, 0, 0, f"Import error: {e}")

    tesseract_path = r"C:\Program Files\Tesseract-OCR\tesseract.exe"
    if Path(tesseract_path).exists():
        pytesseract.pytesseract.tesseract_cmd = tesseract_path

    try:
        doc = fitz.open(pdf_path)
    except Exception as e:
        return (source_title, 0, 0, f"Failed to open: {e}")

    total_pages = len(doc)
    full_text = ""

    # Fast path: check for embedded text
    for page in doc:
        text = page.get_text("text")
        if text.strip():
            full_text += "\n" + text

    # OCR fallback
    if len(full_text.strip()) < 500:
        full_text = ""
        for page_num, page in enumerate(doc):
            mat = fitz.Matrix(2.0, 2.0)
            pix = page.get_pixmap(matrix=mat)
            img = Image.open(io.BytesIO(pix.tobytes("png")))
            try:
                text = pytesseract.image_to_string(img, lang="eng")
                if text.strip():
                    full_text += f"\n--- P{page_num + 1} ---\n" + text
            except:
                pass

    doc.close()

    if not full_text.strip():
        return (source_title, 0, total_pages, "No text")

    # Chunk the text
    chunks = []
    for para in re.split(r"\n\s*\n", full_text):
        para = para.strip()
        if len(para) < 50:
            continue
        sentences = re.split(r"(?<=[.!?])\s+", para)
        current = ""
        for s in sentences:
            s = s.strip()
            if not s:
                continue
            if len(current.split()) + len(s.split()) > 512:
                if current.strip():
                    ctype = classify_chunk(current)
                    chunks.append(
                        {
                            "content": current.strip(),
                            "source": source_title,
                            "chunk_type": ctype,
                            "token_count": len(current.split()),
                        }
                    )
                current = s
            else:
                current = (current + " " + s).strip()
        if current.strip():
            ctype = classify_chunk(current)
            chunks.append(
                {
                    "content": current.strip(),
                    "source": source_title,
                    "chunk_type": ctype,
                    "token_count": len(current.split()),
                }
            )

    # Write chunks to individual output file
    safe_name = re.sub(r"[^a-zA-Z0-9_]", "_", source_title)[:50]
    out_file = Path(out_dir) / f"_ocr_{safe_name}.json"
    with open(out_file, "w", encoding="utf-8") as f:
        json.dump({"source": source_title, "chunks": chunks, "count": len(chunks)}, f)

    return (source_title, len(chunks), total_pages, None)


def main():
    p = argparse.ArgumentParser(description="Parallel PDF OCR - All Books")
    p.add_argument("path", help="PDF file or directory")
    p.add_argument("--workers", type=int, default=max(1, cpu_count() - 1))
    p.add_argument("--output", default="knowledge/chunks_ocr.json")
    p.add_argument("--temp-dir", default="knowledge/.ocr_temp")
    args = p.parse_args()

    target = Path(args.path)
    if target.is_file():
        pdfs = [(str(target), target.stem.replace("_", " "))]
    else:
        pdfs = [
            (str(f), f.stem.replace("_", " ")) for f in sorted(target.glob("*.pdf"))
        ]

    # Create temp directory
    temp_dir = Path(args.temp_dir)
    temp_dir.mkdir(parents=True, exist_ok=True)

    # Add temp_dir to each tuple
    jobs = [(pdf, title, str(temp_dir)) for pdf, title in pdfs]

    print(f"Processing {len(pdfs)} PDFs with {args.workers} parallel workers...")
    print(f"Estimated time: ~{len(pdfs) * 2 // args.workers + 1} minutes\n")

    success = 0
    failed = 0
    total_chunks = 0

    with Pool(processes=args.workers) as pool:
        results = pool.imap_unordered(process_pdf, jobs)

        for title, chunk_count, pages, error in results:
            if error:
                print(f"  FAIL: {title[:60]} - {error}")
                failed += 1
            else:
                print(f"  OK: {title[:60]} ({chunk_count} chunks, {pages}p)")
                success += 1
                total_chunks += chunk_count

    # Merge all individual files into final output
    print(f"\nMerging {success} outputs...")
    all_chunks = []
    for f in sorted(temp_dir.glob("_ocr_*.json")):
        with open(f, "r", encoding="utf-8") as fh:
            data = json.load(fh)
            all_chunks.extend(data["chunks"])

    out_path = Path(args.output)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(
            {
                "version": "1.0",
                "generated_at": datetime.now(timezone.utc).isoformat(),
                "total_chunks": len(all_chunks),
                "chunks": all_chunks,
            },
            f,
            indent=2,
            ensure_ascii=False,
        )

    # Clean up temp files
    for f in temp_dir.glob("_ocr_*.json"):
        f.unlink()
    try:
        temp_dir.rmdir()
    except:
        pass

    print(f"\n{'=' * 50}")
    print(f"COMPLETE: {success} books, {failed} failed")
    print(f"Total chunks: {len(all_chunks)}")
    print(f"Output: {args.output}")
    print(f"{'=' * 50}")


if __name__ == "__main__":
    main()
