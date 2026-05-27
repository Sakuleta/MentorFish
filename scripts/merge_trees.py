"""
Merge multiple opening tree JSONs into a single comprehensive database.

Merges by FEN key: frequencies and scores are summed across sources.
ECO and opening_name are NOT propagated from individual sources (they
would conflict across ABK books which don't contain reliable ECO data).
Children lists are merged and deduplicated by UCI; the SAN from the
first source that provides it is preserved.
"""

import json
from pathlib import Path


def merge_trees(files, output_path):
    """Merge multiple opening tree JSONs into a single comprehensive database."""
    merged = {}

    for fpath in files:
        path = Path(fpath)
        if not path.exists():
            print(f"  SKIP: {path.name} (not found)")
            continue

        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)

        positions = data.get("positions", [])
        print(f"  {path.name}: {len(positions)} positions")

        for pos in positions:
            fen = pos["fen"]
            if fen not in merged:
                merged[fen] = {
                    "fen": fen,
                    "frequency": 0,
                    "white_score_sum": 0.0,
                    "children": {},
                }

            m = merged[fen]
            m["frequency"] += pos.get("frequency", 0)
            if pos.get("white_score") is not None:
                m["white_score_sum"] += pos["white_score"] * pos.get("frequency", 0)

            for child in pos.get("children", []):
                uci = child["uci"]
                if uci not in m["children"]:
                    m["children"][uci] = {
                        "uci": uci,
                        "san": child.get("san", uci),
                        "frequency": 0,
                    }
                m["children"][uci]["frequency"] += child.get("frequency", 0)

    # Calculate averages and sort
    output = []
    for fen, m in sorted(merged.items(), key=lambda x: -x[1]["frequency"]):
        ws = (
            round(m["white_score_sum"] / m["frequency"], 4)
            if m["frequency"] > 0
            else 0.5
        )

        children = []
        for uci, c in sorted(m["children"].items(), key=lambda x: -x[1]["frequency"]):
            children.append(
                {"uci": c["uci"], "san": c["san"], "frequency": c["frequency"]}
            )

        output.append(
            {
                "fen": fen,
                "eco": "",  # Deliberately empty — ABK books don't have ECO per position
                "opening_name": "",
                "frequency": m["frequency"],
                "white_score": ws,
                "children": children[:15],
            }
        )

    out_path = Path(output_path)
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(
            {
                "version": "1.0",
                "source": "merged_abk_books",
                "unique_positions": len(output),
                "positions": output,
            },
            f,
            indent=2,
            ensure_ascii=False,
        )

    print(f"\n  MERGED: {len(output)} unique positions -> {output_path}")


def main():
    import argparse

    p = argparse.ArgumentParser(
        description="Merge multiple opening tree JSONs into one"
    )
    p.add_argument("files", nargs="+", help="JSON tree files to merge")
    p.add_argument("--output", default="knowledge/openings_tree_merged.json")
    args = p.parse_args()

    print("Merging opening trees...")
    merge_trees(args.files, args.output)
    print("Done!")


if __name__ == "__main__":
    main()
