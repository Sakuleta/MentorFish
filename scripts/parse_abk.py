"""
MentorFish ABK Opening Book Parser
Parses Arena .abk files into the opening tree format.
ABK format: 28-byte entries, root at index 900.
"""

import json
import struct
from pathlib import Path

# Square mapping: index 0-63 (a1=0, b1=1, ..., h8=63)
SQUARES = [f"{chr(97 + c)}{r}" for r in range(1, 9) for c in range(8)]


def idx_to_square(idx):
    if 0 <= idx < 64:
        return SQUARES[idx]
    return f"?{idx}"


def square_to_uci(from_idx, to_idx, promotion):
    """Convert from/to indices + promotion char to UCI move."""
    prom_map = {1: "q", 2: "r", 3: "b", 4: "n", -1: "q", -2: "r", -3: "b", -4: "n"}
    uci = idx_to_square(from_idx) + idx_to_square(to_idx)
    if promotion != 0:
        uci += prom_map.get(promotion, "q")
    return uci


def parse_abk(filepath):
    """Parse an Arena .abk file and return a list of entries."""
    with open(filepath, "rb") as f:
        data = f.read()

    if len(data) < 28 * 901:
        print(f"ERROR: {filepath} too small ({len(data)} bytes)")
        return []

    entries = []
    num_entries = len(data) // 28

    for i in range(num_entries):
        offset = i * 28
        from_sq, to_sq, promo, priority = struct.unpack_from("<bbbb", data, offset)
        ngames, nwon, nlost, plycount = struct.unpack_from("<iiii", data, offset + 4)
        next_move, next_sibling = struct.unpack_from("<ii", data, offset + 20)

        entries.append(
            {
                "index": i,
                "from": from_sq,
                "to": to_sq,
                "promotion": promo,
                "priority": priority,
                "ngames": ngames,
                "nwon": nwon,
                "nlost": nlost,
                "plycount": plycount,
                "next_move": next_move,
                "next_sibling": next_sibling,
            }
        )

    return entries


def abk_to_opening_tree(entries):
    """Convert ABK entries to FEN-keyed opening tree with frequencies.

    Uses an iterative DFS with explicit board state management.
    Each stack entry carries its own board copy so the state is never
    corrupted by sibling/child traversal order or invalid moves.
    """
    import chess

    if len(entries) <= 900:
        print("ERROR: No root entry found")
        return {}

    positions = {}
    start_board = chess.Board()
    stack = [(900, start_board)]
    visited = set()

    while stack:
        idx, board = stack.pop()
        if idx <= 0 or idx >= len(entries) or idx in visited:
            continue
        visited.add(idx)

        entry = entries[idx]

        # Handle sentinel root entry (index 900) — it has no move itself,
        # just walk its children as the starting positions.
        if entry["from"] < 0 or entry["to"] < 0:
            if entry["next_move"] > 0:
                stack.append((entry["next_move"], board.copy()))
            if entry["next_sibling"] > 0:
                stack.append((entry["next_sibling"], board.copy()))
            continue

        # Parse the UCI move and advance the board
        uci = square_to_uci(entry["from"], entry["to"], entry["promotion"])
        try:
            move = board.parse_uci(uci)
            fen_before = board.fen()
            # Generate SAN BEFORE pushing — board.san() needs the pre-move state
            try:
                san = board.san(move)
            except Exception:
                san = uci
            board.push(move)
        except Exception:
            # Invalid move for this board state — skip this entry
            continue

        # Record position (FEN before the move was played)
        if fen_before not in positions:
            positions[fen_before] = {
                "fen": fen_before,
                "frequency": 0,
                "white_score_sum": 0.0,
                "children": {},
            }
        pos = positions[fen_before]
        pos["frequency"] += entry["ngames"]
        total = entry["nwon"] + entry["nlost"]
        if total > 0:
            pos["white_score_sum"] += (entry["nwon"] / total) * entry["ngames"]

        if uci not in pos["children"]:
            pos["children"][uci] = {"uci": uci, "san": san, "frequency": 0}
        pos["children"][uci]["frequency"] += entry["ngames"]

        # Push children (board already advanced by this move)
        if entry["next_move"] > 0:
            stack.append((entry["next_move"], board.copy()))

        # Restore board and push siblings (same pre-move position, different move)
        board.pop()
        if entry["next_sibling"] > 0:
            stack.append((entry["next_sibling"], board.copy()))

    # Calculate weighted averages
    for pos in positions.values():
        if pos["frequency"] > 0:
            pos["white_score"] = round(pos["white_score_sum"] / pos["frequency"], 4)
        else:
            pos["white_score"] = 0.5

    return positions


def export_tree(positions, output_path, source_name):
    """Export to JSON format compatible with build_openings.py output."""
    output = []
    for fen, pos in sorted(positions.items(), key=lambda x: -x[1].get("frequency", 0)):
        children = []
        for uci, child in sorted(
            pos.get("children", {}).items(), key=lambda x: -x[1].get("frequency", 0)
        ):
            children.append(
                {
                    "uci": child["uci"],
                    "san": child["san"],
                    "frequency": child["frequency"],
                }
            )

        output.append(
            {
                "fen": fen,
                "eco": "",  # ABK files don't have ECO codes
                "opening_name": "",  # Don't apply source name as opening name
                "frequency": pos.get("frequency", 0),
                "white_score": pos.get("white_score", 0.5),
                "children": children[:15],
            }
        )

    out_path = Path(output_path)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(
            {
                "version": "1.0",
                "source": source_name,
                "unique_positions": len(output),
                "positions": output,
            },
            f,
            indent=2,
            ensure_ascii=False,
        )

    print(f"  {len(output)} positions -> {output_path}")


def main():
    import argparse

    p = argparse.ArgumentParser(description="ABK to Opening Tree Converter")
    p.add_argument("abk_file", type=str, help="Path to .abk file")
    p.add_argument("--output", type=str, help="Output JSON path")
    args = p.parse_args()

    abk_path = Path(args.abk_file)
    output = args.output or f"knowledge/{abk_path.stem}_tree.json"

    print(f"Parsing: {abk_path.name}")
    entries = parse_abk(str(abk_path))
    print(f"  {len(entries)} entries")

    positions = abk_to_opening_tree(entries)
    export_tree(positions, output, abk_path.stem)


if __name__ == "__main__":
    main()
