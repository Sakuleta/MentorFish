// ─── Library View ───
// Browse imported books, filter by topic, track reading progress.

import { useState } from "react";
import { Search, BookOpen, Download, Upload } from "lucide-react";
import { BookReaderView } from "./BookReaderView";
import { getTauri } from "../../lib/tauriBridge";
import { cn } from "../../lib/utils";
import {
  Button,
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
  Badge,
} from "../../components/ui";

interface BookInfo {
  title: string;
  author: string;
  chunks: number;
  topic: string;
}

const BOOKS: BookInfo[] = [
  {
    title: "Bobby Fischer Teaches Chess",
    author: "Bobby Fischer",
    chunks: 732,
    topic: "Training",
  },
  {
    title: "My Great Predecessors (Vol 1-5)",
    author: "Garry Kasparov",
    chunks: 72516,
    topic: "Game Collections",
  },
  {
    title: "100 Endgames You Must Know",
    author: "Jesus de la Villa",
    chunks: 272,
    topic: "Endgame",
  },
  {
    title: "1001 Brilliant Chess Sacrifices",
    author: "Fred Reinfeld",
    chunks: 654,
    topic: "Tactics",
  },
  {
    title: "1001 Brilliant Ways to Checkmate",
    author: "Fred Reinfeld",
    chunks: 1122,
    topic: "Tactics",
  },
  {
    title: "Silman's Complete Endgame Course",
    author: "Jeremy Silman",
    chunks: 582,
    topic: "Endgame",
  },
  {
    title: "Karpov's Strategic Wins",
    author: "Tibor Karolyi",
    chunks: 1073,
    topic: "Game Collections",
  },
  {
    title: "GM Prep: Attack and Defence",
    author: "Jacob Aagaard",
    chunks: 381,
    topic: "Tactics",
  },
  {
    title: "GM Prep: Calculation",
    author: "Jacob Aagaard",
    chunks: 1601,
    topic: "Training",
  },
  {
    title: "GM Prep: Endgame Play",
    author: "Jacob Aagaard",
    chunks: 502,
    topic: "Endgame",
  },
  {
    title: "GM Prep: Positional Play",
    author: "Jacob Aagaard",
    chunks: 2309,
    topic: "Strategy",
  },
  {
    title: "GM Prep: Strategic Play",
    author: "Jacob Aagaard",
    chunks: 303,
    topic: "Strategy",
  },
  {
    title: "GM Prep: Thinking Inside The Box",
    author: "Jacob Aagaard",
    chunks: 483,
    topic: "Training",
  },
  {
    title: "Art of Attack in Chess",
    author: "Vladimir Vukovic",
    chunks: 2363,
    topic: "Tactics",
  },
  {
    title: "Attack With Mikhail Tal",
    author: "Mikhail Tal",
    chunks: 1750,
    topic: "Game Collections",
  },
  {
    title: "Boost Your Chess 1: Fundamentals",
    author: "Artur Yusupov",
    chunks: 232,
    topic: "Training",
  },
  {
    title: "Boost Your Chess 2: Beyond Basics",
    author: "Artur Yusupov",
    chunks: 282,
    topic: "Training",
  },
  {
    title: "Boost Your Chess 3: Mastery",
    author: "Artur Yusupov",
    chunks: 308,
    topic: "Training",
  },
  {
    title: "Build Up Your Chess 1",
    author: "Artur Yusupov",
    chunks: 266,
    topic: "Training",
  },
  {
    title: "Build Up Your Chess 2",
    author: "Artur Yusupov",
    chunks: 1852,
    topic: "Training",
  },
  {
    title: "Build Up Your Chess 3",
    author: "Artur Yusupov",
    chunks: 296,
    topic: "Training",
  },
  {
    title: "Chess Fundamentals",
    author: "J.R. Capablanca",
    chunks: 201,
    topic: "General",
  },
  {
    title: "Chess Evolution 1",
    author: "Artur Yusupov",
    chunks: 255,
    topic: "Training",
  },
  {
    title: "Chess Evolution 2",
    author: "Artur Yusupov",
    chunks: 293,
    topic: "Training",
  },
  {
    title: "Chess Evolution 3",
    author: "Artur Yusupov",
    chunks: 329,
    topic: "Training",
  },
  {
    title: "Dvoretsky's Endgame Manual",
    author: "Mark Dvoretsky",
    chunks: 4898,
    topic: "Endgame",
  },
  {
    title: "Endgame Strategy",
    author: "Mikhail Shereshevsky",
    chunks: 1893,
    topic: "Endgame",
  },
  {
    title: "FCO: Fundamental Chess Openings",
    author: "Paul van der Sterren",
    chunks: 810,
    topic: "Openings",
  },
  {
    title: "Chess Structures",
    author: "Mauricio Flores Rios",
    chunks: 40,
    topic: "Strategy",
  },
  {
    title: "Fundamental Chess Endings",
    author: "Muller & Lamprecht",
    chunks: 3762,
    topic: "Endgame",
  },
  {
    title: "Mastering the Chess Openings Vol 1",
    author: "John Watson",
    chunks: 3046,
    topic: "Openings",
  },
  {
    title: "Kasparov on Modern Chess Part 1",
    author: "Garry Kasparov",
    chunks: 3603,
    topic: "Game Collections",
  },
  {
    title: "Kasparov on Modern Chess Part 2",
    author: "Garry Kasparov",
    chunks: 570,
    topic: "Game Collections",
  },
  {
    title: "Kasparov on Modern Chess Part 3",
    author: "Garry Kasparov",
    chunks: 563,
    topic: "Game Collections",
  },
  {
    title: "Kasparov on Modern Chess Part 4",
    author: "Garry Kasparov",
    chunks: 555,
    topic: "Game Collections",
  },
  {
    title: "How to Reassess Your Chess",
    author: "Jeremy Silman",
    chunks: 659,
    topic: "Strategy",
  },
  {
    title: "Mastering Chess Strategy",
    author: "Johan Hellsten",
    chunks: 5487,
    topic: "Strategy",
  },
  {
    title: "Modern Chess Openings",
    author: "Nick de Firmian",
    chunks: 6883,
    topic: "Openings",
  },
  {
    title: "My 60 Memorable Games",
    author: "Bobby Fischer",
    chunks: 2092,
    topic: "Game Collections",
  },
  {
    title: "My System",
    author: "Aron Nimzowitsch",
    chunks: 1746,
    topic: "Strategy",
  },
  {
    title: "SCE: Attack and Defence",
    author: "Mark Dvoretsky",
    chunks: 286,
    topic: "Tactics",
  },
  {
    title: "SCE: Endgame Analysis",
    author: "Mark Dvoretsky",
    chunks: 2002,
    topic: "Endgame",
  },
  {
    title: "SCE: Opening Developments",
    author: "Mark Dvoretsky",
    chunks: 1480,
    topic: "Openings",
  },
  {
    title: "SCE: Strategic Play",
    author: "Mark Dvoretsky",
    chunks: 1880,
    topic: "Strategy",
  },
  {
    title: "SCE: Tactical Play",
    author: "Mark Dvoretsky",
    chunks: 2234,
    topic: "Tactics",
  },
  {
    title: "Secrets of Modern Chess Strategy",
    author: "John Watson",
    chunks: 468,
    topic: "Strategy",
  },
  {
    title: "Tal-Botvinnik Moscow 1960",
    author: "Mikhail Tal",
    chunks: 1316,
    topic: "Game Collections",
  },
  {
    title: "The Life and Games of Mikhail Tal",
    author: "Mikhail Tal",
    chunks: 3508,
    topic: "Game Collections",
  },
  {
    title: "Think Like a Grandmaster",
    author: "Alexander Kotov",
    chunks: 200,
    topic: "Training",
  },
  {
    title: "Zurich International Chess Tournament",
    author: "David Bronstein",
    chunks: 335,
    topic: "Game Collections",
  },
];

const TOPICS = [
  "All",
  "Endgame",
  "Openings",
  "Tactics",
  "Strategy",
  "Training",
  "Game Collections",
  "General",
];

const TOPIC_COLORS: Record<string, string> = {
  Endgame: "bg-success/15 text-success",
  Openings: "bg-primary/15 text-primary",
  Tactics: "bg-destructive/15 text-destructive",
  Strategy: "bg-accent/15 text-accent",
  Training: "bg-warning/15 text-warning",
  "Game Collections": "bg-muted/30 text-muted-foreground",
  General: "bg-secondary text-secondary-foreground",
};

export function LibraryView() {
  const [filter, setFilter] = useState("All");
  const [search, setSearch] = useState("");
  const [selectedBook, setSelectedBook] = useState<string | null>(null);

  // ── PGN Import ──

  const handleImportPgn = async () => {
    const tauri = await getTauri();
    if (!tauri) {
      alert(
        "Tauri bridge not available. Run the desktop app to import PGN files.",
      );
      return;
    }

    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".pgn";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const result = await tauri.invoke<{
          gamesImported: number;
          errors: string[];
        }>("cmd_import_pgn", { pgnText: text });
        alert(
          `Imported ${result.gamesImported} game${result.gamesImported !== 1 ? "s" : ""}${result.errors.length ? ` (${result.errors.length} error${result.errors.length !== 1 ? "s" : ""})` : ""}`,
        );
      } catch (e) {
        alert(`Import failed: ${e}`);
      }
    };
    input.click();
  };

  // ── PGN Export ──

  const handleExportPgn = async () => {
    const tauri = await getTauri();
    if (!tauri) {
      alert("Tauri bridge not available. Run the desktop app to export games.");
      return;
    }

    try {
      const result = await tauri.invoke<{ pgn: string; gameCount: number }>(
        "cmd_export_pgn",
        {},
      );
      if (result.gameCount === 0) {
        alert("No games to export.");
        return;
      }
      const blob = new Blob([result.pgn], { type: "application/x-chess-pgn" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `mentorfish_export_${new Date().toISOString().slice(0, 10)}.pgn`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      alert(`Export failed: ${e}`);
    }
  };

  // ── Book Reader mode ──

  if (selectedBook) {
    return (
      <BookReaderView
        bookTitle={selectedBook}
        onBack={() => setSelectedBook(null)}
      />
    );
  }

  // ── Library Browser mode ──

  const filtered = BOOKS.filter((b) => {
    if (filter !== "All" && b.topic !== filter) return false;
    if (
      search &&
      !b.title.toLowerCase().includes(search.toLowerCase()) &&
      !b.author.toLowerCase().includes(search.toLowerCase())
    )
      return false;
    return true;
  });

  const totalChunks = BOOKS.reduce((s, b) => s + b.chunks, 0);

  return (
    <div className="h-full flex flex-col p-5 overflow-hidden">
      {/* Header */}
      <div className="flex items-start justify-between mb-1 shrink-0">
        <div>
          <h2 className="text-xl font-semibold tracking-tight text-foreground">
            Chess Library
          </h2>
          <p className="text-xs text-muted-foreground mt-0.5">
            {BOOKS.length} books &middot; {totalChunks.toLocaleString()}{" "}
            knowledge chunks
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleImportPgn}
            icon={<Upload className="h-3.5 w-3.5" />}
          >
            Import PGN
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={handleExportPgn}
            icon={<Download className="h-3.5 w-3.5" />}
          >
            Export All
          </Button>
        </div>
      </div>

      {/* Search */}
      <div className="relative mt-4 mb-3 shrink-0">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground pointer-events-none" />
        <input
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search books..."
          className={cn(
            "w-full rounded-lg border border-border bg-background py-2 pl-9 pr-3 text-sm text-foreground shadow-sm transition-colors",
            "placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          )}
        />
      </div>

      {/* Topic filters */}
      <div className="flex gap-2 mb-4 overflow-x-auto pb-1 shrink-0 scrollbar-hide">
        {TOPICS.map((topic) => (
          <button
            key={topic}
            onClick={() => setFilter(topic)}
            className="shrink-0"
          >
            <Badge
              variant={filter === topic ? "default" : "secondary"}
              className="cursor-pointer select-none"
            >
              {topic}
            </Badge>
          </button>
        ))}
      </div>

      {/* Book grid */}
      <div className="flex-1 overflow-y-auto -mx-1 px-1">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-4">
          {filtered.map((book) => (
            <Card
              key={book.title}
              className="group flex flex-col border-border bg-card transition-all duration-200 hover:shadow-md hover:border-primary/30"
            >
              <CardHeader className="pb-2">
                <div className="flex items-start justify-between gap-2">
                  <Badge
                    variant="outline"
                    className={cn(
                      "shrink-0 text-[10px]",
                      TOPIC_COLORS[book.topic] ??
                        "bg-muted text-muted-foreground",
                    )}
                  >
                    {book.topic}
                  </Badge>
                </div>
                <CardTitle className="text-sm leading-snug mt-2 line-clamp-2">
                  {book.title}
                </CardTitle>
                <p className="text-xs text-muted-foreground truncate">
                  {book.author}
                </p>
              </CardHeader>
              <CardContent className="flex-1">
                <p className="text-[11px] text-muted-foreground tabular-nums">
                  {book.chunks.toLocaleString()} chunks
                </p>
              </CardContent>
              <CardFooter className="gap-2 pt-0">
                <Button
                  variant="outline"
                  size="sm"
                  className="flex-1"
                  onClick={() => setSelectedBook(book.title)}
                  icon={<BookOpen className="h-3.5 w-3.5" />}
                >
                  Read
                </Button>
                <Button variant="secondary" size="sm" className="flex-1">
                  Study
                </Button>
              </CardFooter>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
