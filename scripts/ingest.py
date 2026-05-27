"""
MentorFish Knowledge Ingestion Pipeline — Section 8.2 of the PRD.
Processes PDF chess books, PGN archives, and personal notes into
sentence-aware chunks ready for embedding and LanceDB storage.

Usage:
    python scripts/ingest.py --pdf knowledge/books/
    python scripts/ingest.py --all

Dependencies: pip install pymupdf python-chess
"""

import argparse
import io
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# ─── PDF Processing ───


def extract_pdf_chunks(pdf_path, source_title):
    try:
        import fitz
    except ImportError:
        print("ERROR: PyMuPDF not installed. Run: pip install pymupdf", file=sys.stderr)
        return []

    chunks = []
    doc = fitz.open(pdf_path)
    full_text = ""

    for page_num, page in enumerate(doc):
        text = page.get_text("text")
        if text.strip():
            full_text += f"\n--- Page {page_num + 1} ---\n" + text
    doc.close()

    if not full_text.strip():
        print(f"  No embedded text — trying OCR...")
        full_text = extract_pdf_ocr(pdf_path)

    if not full_text.strip():
        print(f"  WARNING: No text from {pdf_path} (OCR also failed)")
        return []

    paragraphs = re.split(r"\n\s*\n", full_text)
    for para in paragraphs:
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
                    chunks.append(make_chunk(current.strip(), source_title))
                current = s
            else:
                current = (current + " " + s).strip()
        if current.strip():
            chunks.append(make_chunk(current.strip(), source_title))

    print(f"  {len(chunks)} chunks from {os.path.basename(pdf_path)}")
    return chunks


def extract_pdf_ocr(pdf_path):
    """Extract text from a scanned PDF using OCR (PyMuPDF -> pytesseract)."""
    try:
        import io as io_mod

        import fitz
        import pytesseract
        from PIL import Image
    except ImportError as e:
        print(f"    OCR dependencies missing: {e}")
        return ""

    # Configure tesseract path for Windows
    import platform

    if platform.system() == "Windows":
        tesseract_path = r"C:\Program Files\Tesseract-OCR\tesseract.exe"
        if Path(tesseract_path).exists():
            pytesseract.pytesseract.tesseract_cmd = tesseract_path

    doc = fitz.open(pdf_path)
    total_pages = len(doc)
    full_text = ""

    for page_num, page in enumerate(doc):
        # Render page as image (300 DPI for good OCR quality)
        mat = fitz.Matrix(2.0, 2.0)  # ~144 DPI (faster)
        pix = page.get_pixmap(matrix=mat)
        img = Image.open(io_mod.BytesIO(pix.tobytes("png")))

        try:
            text = pytesseract.image_to_string(img, lang="eng")
        except Exception as e:
            print(f"    OCR error on page {page_num + 1}: {e}")
            continue

        if text.strip():
            full_text += f"\n--- Page {page_num + 1} ---\n" + text

        if (page_num + 1) % 10 == 0:
            print(f"    OCR: {page_num + 1}/{total_pages} pages")

    doc.close()

    if full_text.strip():
        print(f"    OCR extracted {len(full_text)} chars from {total_pages} pages")

    return full_text


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
            "pawn ending",
            "king and pawn",
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
            "opening",
            "variation",
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
            "double check",
            "deflection",
            "decoy",
            "zwischenzug",
            "sacrifice",
            "combination",
        ]
    ):
        return "motif"
    if re.search(r"\d+\.\s*[A-Za-z0-9]", text):
        return "instructive_example"
    return "concept"


def make_chunk(content, source):
    return {
        "content": content,
        "source": source,
        "chunk_type": classify_chunk(content),
        "token_count": len(content.split()),
    }


# ─── PGN Processing ───


def extract_pgn_chunks(pgn_path):
    try:
        import chess.pgn
    except ImportError:
        print(
            "ERROR: python-chess not installed. Run: pip install python-chess",
            file=sys.stderr,
        )
        return []

    chunks = []
    with open(pgn_path, "r", encoding="utf-8", errors="replace") as f:
        pgn_text = f.read()

    games = re.split(r"\n\n(?=\[Event)", pgn_text)
    for game_text in games:
        game_text = game_text.strip()
        if not game_text:
            continue
        try:
            game = chess.pgn.read_game(io.StringIO(game_text))
        except Exception as e:
            continue
        if game is None:
            continue

        headers = game.headers
        white = headers.get("White", "Unknown")
        black = headers.get("Black", "Unknown")
        event = headers.get("Event", "Unknown")

        node = game
        while node:
            if node.comment and len(node.comment.strip()) > 30:
                chunks.append(
                    {
                        "content": node.comment.strip(),
                        "source": f"{white} vs {black}, {event}",
                        "chunk_type": "instructive_example",
                        "position_fen": node.board().fen(),
                        "token_count": len(node.comment.split()),
                    }
                )
            if node.variations:
                node = node.variations[0]
            else:
                break

    print(f"  {len(chunks)} annotated chunks from {os.path.basename(pgn_path)}")
    return chunks


# ─── Notes Processing ───


def extract_note_chunks(notes_path):
    with open(notes_path, "r", encoding="utf-8", errors="replace") as f:
        text = f.read()

    chunks = []
    for para in re.split(r"\n\s*\n", text):
        para = para.strip()
        if len(para) < 30:
            continue
        sentences = re.split(r"(?<=[.!?])\s+", para)
        current = ""
        for s in sentences:
            s = s.strip()
            if not s:
                continue
            if len(current.split()) + len(s.split()) > 512:
                if current.strip():
                    chunks.append(
                        make_chunk(
                            current.strip(),
                            f"personal_notes:{os.path.basename(notes_path)}",
                        )
                    )
                current = s
            else:
                current = (current + " " + s).strip()
        if current.strip():
            chunks.append(
                make_chunk(
                    current.strip(), f"personal_notes:{os.path.basename(notes_path)}"
                )
            )

    print(f"  {len(chunks)} chunks from {os.path.basename(notes_path)}")
    return chunks


# ─── Main ───


def main():
    p = argparse.ArgumentParser(description="MentorFish Knowledge Ingestion")
    p.add_argument("--pdf", type=str, help="PDF file or directory")
    p.add_argument("--pgn", type=str, help="PGN file or directory")
    p.add_argument("--notes", type=str, help="Notes file or directory")
    p.add_argument("--all", action="store_true", help="Process all")
    p.add_argument("--output", type=str, default="knowledge/chunks.json")
    args = p.parse_args()

    all_chunks = []
    base = Path(__file__).resolve().parent.parent

    if args.all or args.pdf:
        d = Path(args.pdf) if args.pdf else base / "knowledge" / "books"
        if d.exists():
            for f in d.glob("*.pdf"):
                print(f"PDF: {f.name}")
                all_chunks.extend(extract_pdf_chunks(str(f), f.stem.replace("_", " ")))

    if args.all or args.pgn:
        d = Path(args.pgn) if args.pgn else base / "knowledge" / "pgn"
        if d.exists():
            for f in d.glob("*.pgn"):
                print(f"PGN: {f.name}")
                all_chunks.extend(extract_pgn_chunks(str(f)))

    if args.all or args.notes:
        d = Path(args.notes) if args.notes else base / "knowledge" / "notes"
        if d.exists():
            for f in d.glob("*"):
                if f.suffix in (".md", ".txt"):
                    print(f"Notes: {f.name}")
                    all_chunks.extend(extract_note_chunks(str(f)))

    out = Path(args.output)
    out.parent.mkdir(parents=True, exist_ok=True)
    result = {
        "version": "1.0",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "total_chunks": len(all_chunks),
        "chunks": all_chunks,
    }
    with open(out, "w", encoding="utf-8") as f:
        json.dump(result, f, indent=2, ensure_ascii=False)

    print(f"\nDone. {len(all_chunks)} chunks -> {out}")


if __name__ == "__main__":
    main()
