"""
MentorFish Opening Tree Builder
Builds a FEN-keyed opening tree from PGN files.
Extracts first 20 plies, counts frequencies, computes win rates.

Usage:
    python scripts/build_openings.py knowledge/pgn/
    python scripts/build_openings.py knowledge/openings/
"""

import argparse
import io
import json
import os
import re
import sys
from collections import defaultdict
from pathlib import Path


def parse_game_opening(pgn_text):
    try:
        import chess.pgn
    except ImportError:
        print("ERROR: pip install python-chess", file=sys.stderr)
        return None
    try:
        game = chess.pgn.read_game(io.StringIO(pgn_text))
    except Exception:
        return None
    if game is None:
        return None

    headers = game.headers
    result = headers.get("Result", "*")
    eco = headers.get("ECO", "")
    opening_name = headers.get("Opening", "")

    board = game.board()
    moves = []
    node = game
    ply_count = 0

    while node and ply_count < 20:
        if node.variations:
            next_node = node.variations[0]
            move = next_node.move
            if move:
                fen = board.fen()
                uci = board.uci(move)
                san = board.san(move)
                moves.append({"fen": fen, "uci": uci, "san": san, "ply": ply_count})
                board.push(move)
                ply_count += 1
            node = next_node
        else:
            break

    if not moves:
        return None

    white_score = None
    if result == "1-0":
        white_score = 1.0
    elif result == "0-1":
        white_score = 0.0
    elif result == "1/2-1/2":
        white_score = 0.5

    return {
        "eco": eco,
        "opening_name": opening_name,
        "result": result,
        "white_score": white_score,
        "moves": moves,
    }


def build_tree(pgn_dir, output_path, max_games=0):
    fen_stats = defaultdict(
        lambda: {
            "count": 0,
            "white_score_sum": 0.0,
            "white_score": 0.0,
            "eco": "",
            "opening_name": "",
            "children": defaultdict(lambda: {"uci": "", "san": "", "count": 0}),
        }
    )

    total_games = 0
    processed = 0

    # Handle .pgn.zst files (Lichess format)
    zst_files = list(Path(pgn_dir).glob("*.pgn.zst"))
    if zst_files:
        print(f"Found {len(zst_files)} compressed Lichess DB files")
        try:
            import zstandard as zstd

            for zf in zst_files:
                print(f"  Decompressing {zf.name}...")
                with open(zf, "rb") as f:
                    dctx = zstd.ZstdDecompressor()
                    with dctx.stream_reader(f) as reader:
                        text = io.TextIOWrapper(reader, encoding="utf-8")
                        pgn_text = text.read()
                out_p = Path(pgn_dir) / (zf.stem + ".pgn")
                with open(out_p, "w", encoding="utf-8") as f:
                    f.write(pgn_text)
                print(f"  -> {out_p.name}")
        except ImportError:
            print("  WARNING: pip install zstandard to process Lichess DB files")

    pgn_files = list(Path(pgn_dir).glob("*.pgn"))
    if not pgn_files:
        print(f"No PGN files found in {pgn_dir}")
        return

    for pgn_file in pgn_files:
        print(f"Processing: {pgn_file.name}")
        with open(pgn_file, "r", encoding="utf-8", errors="replace") as f:
            content = f.read()

        games = re.split(r"\n\n(?=\[Event)", content)
        file_games = len(games)
        total_games += file_games
        print(f"  {file_games} games")

        for game_text in games:
            game_text = game_text.strip()
            if not game_text:
                continue

            opening = parse_game_opening(game_text)
            if opening is None:
                continue
            processed += 1

            for i, md in enumerate(opening["moves"]):
                fen = md["fen"]
                stats = fen_stats[fen]
                stats["count"] += 1
                if opening["white_score"] is not None:
                    stats["white_score_sum"] += opening["white_score"]
                if opening["eco"]:
                    stats["eco"] = opening["eco"]
                if opening["opening_name"]:
                    stats["opening_name"] = opening["opening_name"]

                if i + 1 < len(opening["moves"]):
                    nm = opening["moves"][i + 1]
                    child = stats["children"][nm["uci"]]
                    child["uci"] = nm["uci"]
                    child["san"] = nm["san"]
                    child["count"] += 1

            if max_games > 0 and processed >= max_games:
                break
        if max_games > 0 and processed >= max_games:
            break

    # Calculate averages
    for stats in fen_stats.values():
        if stats["count"] > 0:
            stats["white_score"] = round(stats["white_score_sum"] / stats["count"], 4)

    # Build output
    output_lines = []
    for fen, stats in sorted(fen_stats.items(), key=lambda x: -x[1]["count"]):
        if stats["count"] < 2:
            continue

        children = []
        for uci, child in sorted(
            stats["children"].items(), key=lambda x: -x[1]["count"]
        ):
            children.append(
                {"uci": child["uci"], "san": child["san"], "frequency": child["count"]}
            )

        output_lines.append(
            {
                "fen": fen,
                "eco": stats["eco"],
                "opening_name": stats["opening_name"],
                "frequency": stats["count"],
                "white_score": stats["white_score"],
                "children": children[:10],
            }
        )

    out_path = Path(output_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(
            {
                "version": "1.0",
                "source": str(pgn_dir),
                "total_games": total_games,
                "processed_games": processed,
                "unique_positions": len(output_lines),
                "positions": output_lines,
            },
            f,
            indent=2,
            ensure_ascii=False,
        )

    print(
        f"\nDone. {processed}/{total_games} games -> {len(output_lines)} positions -> {output_path}"
    )


def main():
    p = argparse.ArgumentParser(description="MentorFish Opening Tree Builder")
    p.add_argument("pgn_dir", type=str, help="Directory with PGN files")
    p.add_argument("--output", type=str, default="knowledge/openings_tree.json")
    p.add_argument("--max-games", type=int, default=0, help="Max games (0=all)")
    args = p.parse_args()
    build_tree(args.pgn_dir, args.output, args.max_games)


if __name__ == "__main__":
    main()
