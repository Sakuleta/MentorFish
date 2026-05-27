# TECHNICAL PRD — PERSONAL GM TRAINING SYSTEM
**Version:** 2.1 — Hardware-Calibrated, RAG Corpus Defined  
**Status:** Ready for Architecture Review  
**Last Updated:** 2026-05-25

---

# 1. PRODUCT VISION

> Build a fully local, GM-level personal chess coach capable of producing human-quality pedagogical explanations, maintaining long-term user modeling, and teaching opening, middlegame, and endgame theory.

**The system is an Adaptive Chess Intelligence Platform.** Not a GUI, not an analysis tool, not a chatbot.

**Core capabilities:**
- Deep post-game analysis with layered explanation
- Live interactive coaching during play
- Opening, middlegame, and endgame theory instruction
- Long-term user modeling and weakness tracking
- Personalized study plan generation
- Configurable coaching personas
- Playable opponent with adaptive difficulty

**Deployment:** Fully local. No cloud dependency. Cloud connectivity is an optional future extension, never a requirement.

---

# 2. CORE DESIGN PRINCIPLES

## 2.1 Engine Truth Absolutism

All chess factual authority is determined exclusively by **Stockfish**. The LLM has zero authority over:
- Move legality
- Tactical evaluation
- Position assessment

This is enforced architecturally. The LLM never receives raw engine output. It receives only pre-structured feature objects that have been extracted and validated from engine data. The LLM cannot hallucinate a chess fact it was never asked to generate.

## 2.2 LLM as Pedagogical Intelligence

The LLM's exclusive responsibilities:
- Explanation generation
- Pedagogy and abstraction
- Coaching dialogue
- Curriculum generation
- Humanization of validated engine output

## 2.3 Persistent User Modeling

The system builds and maintains a long-term user model across all sessions. The model is updated after every completed game and drives all personalization decisions.

## 2.4 Full Offline Capability

Every system component — inference, engine, database, vector store — runs locally. Zero external API calls in normal operation.

## 2.5 Hierarchical Explanation

Every explanation is assembled in five layers. Shallower contexts (live coaching) use only the first two or three. Full post-game analysis uses all five.

| Layer | Content |
|-------|---------|
| 1 | Move truth — what the engine says and the eval |
| 2 | Tactical logic — the concrete mechanism |
| 3 | Strategic meaning — the positional principle |
| 4 | Human principle — the general chess lesson |
| 5 | Personalized insight — connection to user's specific patterns |

---

# 3. TARGET CAPABILITIES

## 3.1 Opening Theory Teaching

- Opening tree exploration from internal FEN-keyed database
- Theoretical lines and move order variants
- Novelty detection (deviation from known theory beyond depth 8)
- Transposition detection and explanation
- Move-order traps and practical considerations
- Thematic plans per opening and pawn structure
- Famous model games per opening
- User repertoire management and gap identification

## 3.2 Middlegame Teaching

- Positional plans and prophylaxis
- Initiative evaluation
- Imbalance analysis (bishop vs knight, space vs piece activity, etc.)
- Maneuvering and regrouping plans
- Dynamic vs static compensation
- Piece coordination

## 3.3 Endgame Teaching

- Lucena and Philidor positions
- Opposition and triangulation
- Rook activity principles
- Theoretical draws
- Syzygy tablebase integration for positions with 7 or fewer pieces

## 3.4 Live Interactive Coaching

The system monitors the active game and triggers a coaching intervention when any of the following conditions are met:

| Trigger | Condition |
|---------|-----------|
| Blunder alert | Eval swing >= 150cp on the user's move |
| Weakness pattern match | Position matches a known weakness pattern from user profile (similarity > 0.70) |
| Opening theory departure | Move diverges from known theory past move 6 |
| Time pressure | Remaining time < 20% of total AND position complexity score above threshold |
| User request | Explicit question or coaching request at any time |

Intervention is non-intrusive by default. The system generates a coaching note; the user decides whether to expand it.

## 3.5 Post-Game Analysis

- Blunder and missed tactic detection
- Missed opportunity identification (eval improvement potential per move)
- Phase-based evaluation (opening / middlegame / endgame)
- Opening deviation analysis vs user repertoire
- Psychological collapse detection: eval drop >= 300cp over 3 consecutive user moves combined with move time below 50% of the user's session average
- Time trouble analysis: per-phase time expenditure vs historical baseline

## 3.6 Conversational Chess Mentor

Open-ended coaching dialogue. The system answers chess questions, explains positions, discusses plans, and engages in Socratic instruction.

## 3.7 Book Study System

The system provides a structured book-reading experience for the user's imported chess library.

**Library Browser:**
- Browse all imported books with cover, title, author, page count
- Filter by topic: openings, middlegame, endgame, tactics, strategy
- Search within book content via RAG

**Reading Mode:**
- Page-by-page reading with progress tracking
- Highlight key passages and save as personal notes
- "Explain This" button: selects a passage and asks the Pedagogical Agent to elaborate
- "Quiz Me" button: generates tactical/strategic questions from the current page content
- Bookmark positions with notes

**Progress Tracking:**
- Per-book reading progress (page X of Y, percentage)
- Session tracking per study session (time spent, pages read)
- Streak tracking for daily study habit building
- Integration with User Profile: books read contribute to knowledge dimensions

**Study Plan Integration:**
- Curriculum Agent assigns specific book chapters as study tasks
- Weekly reading goals derived from user weakness profile
- "Study Now" button on Dashboard opens the recommended book at the assigned chapter

---

# 4. SYSTEM ARCHITECTURE

```
+--------------------------------------------------+
|           Desktop UI (Tauri + React)             |
|      Board | Chat | Study | Dashboard            |
+------------------------+-------------------------+
                         |  Tauri invoke / emit
                         v
+--------------------------------------------------+
|          Conversation Orchestrator               |
|  Routes pipelines by context type               |
|  Manages agent state and pre-fetch              |
+--------+----------------------------+-----------+
         |                            |
         v                            v
+----------------+        +----------------------+
|  Engine Layer  |        |   Knowledge System   |
|  Stockfish     |        |   RAG + Opening DB   |
|  Syzygy TB     |        |   LanceDB vectors    |
+--------+-------+        +-----------+----------+
         |                            |
         v                            v
+--------------------------------------------------+
|           Feature Extraction Layer              |
|   rule-based (python-chess / shakmaty)          |
|   + Stockfish UCI output parsing                |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
|           Multi-Agent Pipeline                  |
|  Tactical -> Strategic -> Pedagogical           |
|  (+ Memory, Curriculum, Theory as needed)       |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
|           Explanation Assembler                 |
|     Pedagogical Agent: final assembly           |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
|              Memory Layer                       |
|   User profile update after each game          |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
|             Local Storage                       |
|   PostgreSQL | LanceDB | Redis                  |
+--------------------------------------------------+
```

---

# 5. CHESS ENGINE STACK

## 5.1 Primary and Sole Engine: Stockfish

**Source:** https://github.com/official-stockfish/Stockfish  
**Protocol:** UCI (Universal Chess Interface)  
**Runtime:** Managed as a child process by the Rust backend via an async channel-based UCI wrapper.

**Responsibilities:**
- Board evaluation (centipawn scores)
- Principal variation (PV) generation
- Multi-PV candidate move ranking
- Tactical ground truth
- King safety scoring (from eval trace)

**UCI Configuration — Analysis Mode:**

| Option | Value | Rationale |
|--------|-------|-----------|
| Threads | CPU core count minus 2 | Leaves headroom for OS and LLM |
| Hash | 2048 MB | Large hash improves depth and consistency |
| MultiPV | 5 | Provides candidate move range for pedagogy |
| Depth | 22 (default), 28 (post-game) | Sufficient for instructional purposes |

**UCI Configuration — Play Mode:**

| Option | Value |
|--------|-------|
| Threads | 2 |
| Hash | 512 MB |
| Depth | Variable per difficulty mode |

## 5.2 Tablebases: Syzygy

Used for all positions with 7 or fewer pieces. Accessed via Stockfish's built-in SyzygyPath UCI option. Tablebase files stored locally; path is user-configurable in settings.

---

# 6. LOCAL LLM STACK

## 6.1 Architecture Decision: Specialized Multi-Model

Two models serve distinct roles. They are never used interchangeably. Routing is deterministic by task type. Both are calibrated to fit within the 16 GB VRAM constraint of the RX 9070 XT.

The 32B model class is not viable on this hardware. Running a 32B model with ~6 GB of layers offloaded to CPU RAM produces 3–6 tok/s on the offloaded layers, which is unacceptable for any interactive use. The model stack is Qwen3-14B (primary) + Qwen3-8B (fast), both running fully on GPU.

## 6.2 Model A — Primary Reasoning Model: Qwen3-14B

**Role:** All deep reasoning and explanation tasks  
**Use cases:** Post-game analysis, strategic explanation, theory instruction, complex coaching dialogue  
**Latency target:** First token within 6 seconds (acceptable for non-real-time contexts)  
**Quantization:** Q8_0 (~15 GB VRAM) for maximum quality; fallback Q4_K_M (~9 GB) if context window pressure occurs  
**VRAM required:** ~15 GB (Q8_0)

**Rationale:** Qwen3-14B with thinking mode enabled (enabled via `enable_thinking: true` in the Ollama API call) provides explicit chain-of-thought reasoning that maps directly to the layered explanation architecture. At Q8_0, it runs fully in VRAM on the RX 9070 XT with ~1 GB headroom. Q8_0 substantially outperforms Q4_K_M on instruction following and multi-step reasoning tasks — the quality difference at this parameter count justifies the VRAM cost. Thinking mode is enabled for POST_GAME, THEORY, and CONVERSATIONAL pipelines; disabled for LIVE_COACHING and CURRICULUM where latency is the priority.

## 6.3 Model B — Fast Interaction Model: Qwen3-8B

**Role:** All latency-sensitive tasks  
**Use cases:** Live coaching notes, conversational replies, curriculum plan generation, orchestration sub-queries  
**Latency target:** First token within 2 seconds  
**Quantization:** Q4_K_M (~5 GB VRAM)  
**VRAM required:** ~5 GB

**Rationale:** Qwen3-8B at Q4_K_M fits in 5 GB and generates fast enough for real-time coaching annotation. The quality gap vs 14B is acceptable for the task types it handles — brief coaching notes and structured output generation, not deep multi-step reasoning.

## 6.4 Model Swap Strategy

The two models **cannot run simultaneously** in 16 GB VRAM at full quality (Q8_0 + Q4_K_M = ~20 GB). Ollama handles model loading/unloading automatically. Swap time is approximately 8–12 seconds.

To minimize swap frequency, the Orchestrator follows this loading strategy:

| Session Phase | Loaded Model | Reason |
|--------------|-------------|--------|
| Game in progress | Qwen3-8B | Live coaching is real-time; 8B stays warm |
| Post-game analysis starts | Swap to Qwen3-14B | Deep analysis pipeline triggered once |
| Analysis complete, conversation | Qwen3-14B stays warm | User likely to ask follow-up questions |
| New game starts | Swap to Qwen3-8B | Live coaching priority resumes |

Model to keep warm is stored in session state. The Orchestrator signals Ollama to preload the next model 30 seconds before an anticipated swap (e.g., as the last moves of a game are played).

## 6.5 Model Routing Policy

The Orchestrator assigns every task to exactly one model. No dynamic or heuristic routing.

| Task Type | Model | Thinking Mode |
|-----------|-------|--------------|
| Post-game deep analysis | Qwen3-14B | Enabled |
| Opening / endgame theory instruction | Qwen3-14B | Enabled |
| Strategic plan explanation | Qwen3-14B | Enabled |
| Complex coaching dialogue | Qwen3-14B | Enabled |
| Live coaching note (brief) | Qwen3-8B | Disabled |
| Short conversational reply | Qwen3-8B | Disabled |
| Curriculum plan generation | Qwen3-8B | Disabled |
| Orchestration sub-tasks | Qwen3-8B | Disabled |

## 6.6 Inference Abstraction Layer

All LLM calls go through a backend-agnostic `InferenceClient` interface. No application code changes are required to switch runtimes.

```rust
trait InferenceClient: Send + Sync {
    async fn complete(
        &self,
        model: ModelId,
        messages: Vec<Message>,
        options: InferenceOptions,
    ) -> Result<Pin<Box<dyn Stream<Item = Token> + Send>>>;
}

// Implementations:
struct OllamaClient    { base_url: String }  // current runtime
struct VllmClient      { base_url: String }  // future: when RDNA4 kernel support ships
struct LlamaCppClient  { base_url: String }  // fallback / direct access
```

**Config field:** `inference.backend = "ollama" | "vllm" | "llama_cpp"`  
**Runtime:** Ollama (llama.cpp Vulkan backend internally — see Section 17)

## 6.7 Future: Chess-Specific Fine-Tune

**Status:** Deferred. Separate sub-PRD required.  
**Trigger condition:** Core system stable at v1.0 with 200+ annotated user sessions.  
**Planned base model:** Qwen3-8B (smaller iteration cycle, faster eval)  
**Framework:** QLoRA via Unsloth  
**Dataset:** Annotated positions, engine explanations, chess literature extracts, user game history

---

# 7. ORCHESTRATION ARCHITECTURE

## 7.1 Design: In-Process Stateless Pipeline

Agents are stateless functions. They receive a typed context struct and return a typed output struct. The Orchestrator holds all session state and executes agent chains in a defined order per pipeline type.

This is a local single-user desktop application. Microservices and message queues would be overengineering. In-process function calls are correct.

## 7.2 Orchestrator Context Struct

```typescript
interface OrchestratorContext {
  pipeline_type:   "POST_GAME" | "LIVE_COACHING" | "THEORY" | "CURRICULUM" | "CONVERSATIONAL";
  position:        FEN;
  game_history:    Move[];
  engine_output:   EngineOutput;      // pre-fetched before pipeline starts
  features:        FeatureBundle;     // pre-extracted before pipeline starts
  rag_results:     RetrievalBundle;   // pre-fetched before pipeline starts
  user_profile:    UserProfile;
  persona:         PersonaId;
  session:         SessionContext;
}
```

## 7.3 Pipeline Definitions

Each pipeline type defines a fixed, ordered agent execution sequence.

| Pipeline | Agent Chain |
|----------|-------------|
| POST_GAME | Tactical -> Strategic -> Theory -> Memory -> Pedagogical |
| LIVE_COACHING | Tactical -> Pedagogical (fast path, Qwen3-8B, thinking off) |
| THEORY | Theory -> Pedagogical |
| CURRICULUM | Memory (read) -> Curriculum -> Pedagogical |
| CONVERSATIONAL | Pedagogical (direct, with RAG context) |

## 7.4 Agent Specifications

### Agent A: Tactical Agent

```
Input:  { engine_output: EngineOutput, features: FeatureBundle }

Output: TacticalSummary {
  blunders:          BlunderRecord[],
  missed_tactics:    TacticRecord[],
  eval_swings:       EvalSwing[],
  forcing_sequences: ForcingLine[],
  confidence:        ConfidenceScore
}
```

**No LLM call.** Pure computation over structured engine data. Always fast.

---

### Agent B: Strategic Agent

```
Input:  { features: FeatureBundle, tactical_summary: TacticalSummary, rag_results: RetrievalBundle }

Output: StrategicSummary {
  imbalances:        Imbalance[],
  plans:             Plan[],
  pawn_structure:    PawnStructureClassification,
  key_weaknesses:    Square[],
  positional_themes: Theme[],
  confidence:        ConfidenceScore
}
```

**LLM call:** Qwen3-14B with thinking enabled (post-game). Skipped in live coaching fast path.

---

### Agent C: Pedagogical Agent

This agent is the sole explanation assembler. All upstream outputs converge here.

```
Input: {
  tactical_summary?:   TacticalSummary,
  strategic_summary?:  StrategicSummary,
  theory_output?:      TheoryOutput,
  curriculum_output?:  CurriculumOutput,
  user_profile:        UserProfile,
  persona:             PersonaId,
  depth:               "BRIEF" | "STANDARD" | "FULL",
  confidence_flags:    ConfidenceFlag[]
}

Output: FinalExplanation {
  text:                   string,
  layer_breakdown:        LayerContent[],
  confidence:             ConfidenceScore,
  low_confidence_note?:   string
}
```

**LLM call:** Qwen3-14B with thinking enabled (STANDARD / FULL depth). Qwen3-8B with thinking disabled (BRIEF / live coaching).

**Assembly template (slots):**

| Slot | Content | Required |
|------|---------|----------|
| Hook | The critical moment in one sentence | Always |
| What happened | Tactical fact from Tactical Agent | Always |
| Why it matters | Strategic meaning from Strategic Agent | If available |
| The principle | General chess lesson | If available |
| Your pattern | Connection to user profile weakness | If profile confidence >= 0.5 |

---

### Agent D: Memory Agent

```
Input:  { game_record: GameRecord, tactical_summary: TacticalSummary, strategic_summary: StrategicSummary }

Output: ProfileDelta {
  dimension_updates:         DimensionUpdate[],
  new_weakness_flags:        WeaknessFlag[],
  opening_repertoire_events: RepertoireEvent[]
}
```

**No LLM call.** Pure computation. Applied to the database after the full pipeline completes.

---

### Agent E: Curriculum Agent

```
Input:  { user_profile: UserProfile, recent_games: GameRecord[], requested_focus?: Topic }

Output: StudyPlan {
  weekly_sessions:      StudySession[],
  opening_drills:       OpeningDrill[],
  endgame_exercises:    EndgameExercise[],
  tactical_puzzle_theme: TacticTheme,
  rationale:            string
}
```

**LLM call:** Qwen3-14B

---

### Agent F: Theory Agent

```
Input:  { position: FEN, opening_node: OpeningNode, rag_results: RetrievalBundle, repertoire: Repertoire }

Output: TheoryOutput {
  theoretical_lines:  Line[],
  model_games:        GameReference[],
  transpositions:     TranspositionNote[],
  novelty_flag:       boolean,
  historical_context: string | null,
  confidence:         ConfidenceScore
}
```

**LLM call:** Qwen3-14B with thinking enabled

---

## 7.5 Error Handling Policy

| Failure Condition | Behavior |
|------------------|----------|
| Agent returns error | Log error. Skip agent. Mark affected output layers as unavailable. Continue pipeline. |
| LLM call timeout (>15s) | Abort. Return rule-based output from completed agents only. |
| Engine unavailable | Block all pipelines. Surface engine error state in UI. No fallback (engine is source of truth). |
| RAG returns no results | Continue with empty RAG context. Mark theory layers as low-confidence. |
| Any input marked low-confidence | Pedagogical Agent prepends uncertainty note to final explanation. |

---

# 8. KNOWLEDGE / RAG SYSTEM

## 8.1 Knowledge Corpus

The RAG system is only as good as its source material. Books are loaded by the user via the Settings → Knowledge Base screen. The system accepts PDF files and processes them automatically through the ingestion pipeline.

### Tier 1 — Required (RAG backbone, highest pedagogical density)

| Book | Author | Primary Contribution |
|------|--------|---------------------|
| Dvoretsky's Endgame Manual | Mark Dvoretsky | Endgame technique layer — technical gold standard |
| Grandmaster Preparation Series (6 vols.) | Jacob Aagaard | Calculation, Positional Play, Strategic Play, Attack & Defence, Endgame Play, Thinking Inside the Box |
| How to Reassess Your Chess (4th ed.) | Jeremy Silman | Imbalance framework — structural foundation of strategic layer |
| My System + Chess Praxis | Aron Nimzowitsch | Classical positional principle library |
| School of Chess Excellence (4 vols.) | Mark Dvoretsky | Endgame, Tactical Play, Strategic Play, Opening Developments |

### Tier 2 — High Priority

| Book | Author | Primary Contribution |
|------|--------|---------------------|
| Yusupov Training Series (9 vols.) | Artur Yusupov | Structured curriculum grounding — ideal for Curriculum Agent |
| 100 Endgames You Must Know | Jesus de la Villa | Practical endgame — Lucena, Philidor, theoretical draws |
| Endgame Strategy | Mikhail Shereshevsky | Rook endings, conversion technique |
| Secrets of Modern Chess Strategy | John Watson | Modern positional concepts, rule-breaking principles |
| Chess Structures | Mauricio Flores Rios | Per-pawn-structure plan library — critical for opening integration |
| Zurich 1953 | David Bronstein | Model games — deep strategic annotations |
| My 60 Memorable Games | Bobby Fischer | Annotated games — positional precision and endgame |
| Mastering Chess Strategy | Johan Hellsten | Positional concept library |

### Tier 3 — Supplementary

| Book | Author | Primary Contribution |
|------|--------|---------------------|
| Fundamental Chess Endings | Müller & Lamprecht | Comprehensive endgame reference |
| The Art of Attack in Chess | Vladimir Vuković | Attack pattern library |
| Kasparov on Modern Chess (5 vols.) | Garry Kasparov | Modern GM game annotation corpus |
| Think Like a Grandmaster | Alexander Kotov | Calculation methodology |

### User's Own Games (PGN)
The user's personal game archive (exported from Chess.com or Lichess) is ingested separately. These are used for:
- Opening repertoire detection and gap analysis
- User weakness pattern seeding
- Personalized coaching context

### Personal Notes
Markdown or plain text files the user creates. Tagged at ingestion for elevated retrieval weight.

---

## 8.2 Knowledge Ingestion Pipeline

Offline pipeline triggered via Settings → Knowledge Base → "Run Ingestion". Must be run after adding new PDFs or PGN files. Incremental: only new or modified files are re-processed.

### PDF Books (Chess Literature)

```
PDF (user-uploaded via Settings UI)
  -> PyMuPDF: text extraction per page
  -> Chapter / section boundary detection (heading heuristics + font size analysis)
  -> Sentence-aware chunking (max 512 tokens, 64-token overlap)
  -> Chunk type classification: concept | motif | instructive_example | endgame_technique
  -> nomic-embed-text-v1.5: 768-dim embedding (local via Ollama)
  -> Store: LanceDB with source metadata
```

### PGN Archives (Annotated Games)

```
PGN (user-imported via Settings UI)
  -> python-chess: parse headers + movetext + NAG annotations + comments
  -> Per-game: GameRecord (structured) -> PostgreSQL
  -> Per annotated move: annotation chunk -> LanceDB (chunk_type = instructive_example)
  -> Opening moves (first 20 plies): indexed in opening_positions table
```

### Opening Database (Lichess DB)

```
Lichess monthly PGN export (user downloads and places in configured folder)
  -> python-chess: extract moves + frequency + result statistics
  -> Build FEN-keyed opening tree -> PostgreSQL opening_positions table
  -> ECO classification mapped per node
  -> Transpositions: handled by FEN normalization (strip move counters, normalize castling rights)
```

### Personal Notes

```
Markdown / plain text files (user-specified folder)
  -> Sentence-aware chunking
  -> nomic-embed-text-v1.5 embedding -> LanceDB
  -> Tagged source = "personal_notes" (retrieval weight boost: 1.3x)
```

## 8.3 Chunk Schema

```typescript
interface KnowledgeChunk {
  id:            UUID;
  chunk_type:    "concept" | "opening" | "motif" | "instructive_example" | "endgame_technique";
  content:       string;
  source:        string;
  position_fen?: string;
  opening_eco?:  string;
  embedding:     float32[768];
  created_at:    timestamp;
}
```

## 8.4 Opening Position Schema

```sql
CREATE TABLE opening_positions (
  fen              TEXT PRIMARY KEY,
  eco              TEXT,
  opening_name     TEXT,
  parent_fen       TEXT REFERENCES opening_positions(fen),
  move_from_parent TEXT,
  frequency        INTEGER,
  white_score      FLOAT,
  theory_chunk_ids UUID[],
  created_at       TIMESTAMP NOT NULL DEFAULT now()
);
```

## 8.5 Retrieval System

All retrieval executes before the agent pipeline starts. The Orchestrator pre-fetches a RetrievalBundle that all agents receive.

| Method | Implementation | Use Case |
|--------|---------------|----------|
| Semantic (vector) | LanceDB cosine similarity | Concept and explanation retrieval |
| Opening tree lookup | PostgreSQL FEN key lookup | Opening classification, theoretical lines |
| Motif search | LanceDB filtered by chunk_type = "motif" + vector similarity | Tactical pattern matching |
| Position-specific | LanceDB + PostgreSQL filter by position_fen | Exact position lookups |

**Confidence thresholds:**  
- Cosine similarity < 0.60: chunk discarded  
- Cosine similarity >= 0.72: chunk marked high-confidence  
- Between 0.60 and 0.72: included, marked medium-confidence

**Embedding model:** nomic-embed-text-v1.5 (local via Ollama)

---

# 9. FEATURE EXTRACTION LAYER

All feature extraction runs before any agent is invoked. The LLM never sees raw FEN or raw engine output.

## 9.1 Step 1: Rule-Based Extraction (shakmaty / python-chess)

Fast, deterministic, zero LLM cost.

| Feature | Extraction Method |
|---------|-----------------|
| Hanging pieces | Attack count vs defense count per piece |
| Forks | Piece attacks multiple undefended targets simultaneously |
| Pins | Piece on line between attacker and more valuable piece |
| Skewers | Reverse pin — valuable piece exposed after forced move |
| King safety | Pawn shield completeness + open files near king count |
| Pawn structure | Isolated, doubled, backward, passed pawn detection |
| Outposts | Squares unreachable by opponent pawns + occupied/reachable by minor piece |
| Open / half-open files | Per-file pawn presence analysis |
| Bishop pair | Both bishops present for a side |
| Piece mobility | Legal move count per piece, normalized |

## 9.2 Step 2: Stockfish UCI Output Parsing

| Feature | UCI Source |
|---------|-----------|
| Centipawn evaluation | info score cp |
| Mate threat | info score mate |
| Top 5 candidate moves | info multipv |
| Eval swing | Delta between current and previous position eval |
| Search depth | info depth |

## 9.3 FeatureBundle Output Schema

```typescript
interface FeatureBundle {
  position_fen:    string;
  eval_cp:         number;
  eval_swing_cp:   number;
  is_forced_mate:  boolean;
  mate_in?:        number;
  top_moves:       CandidateMove[];
  tactics:         TacticalFeature[];
  positional:      PositionalFeature[];
  dynamic:         DynamicFeature[];
  confidence:      "HIGH" | "MEDIUM";
}
```

Confidence is HIGH when all extraction steps complete without errors. MEDIUM when any step produces incomplete output (e.g., engine timeout, unusual position).

---

# 10. MEMORY SYSTEM

## 10.1 User Profile Dimensions

| Dimension | Description | Data Source |
|-----------|-------------|-------------|
| tactical_accuracy | Accuracy on tactical moments vs engine best | Blunder and miss rate per game |
| positional_accuracy | Accuracy on non-tactical strategic decisions | Eval alignment on quiet moves |
| opening_knowledge | Depth and breadth of theoretical knowledge | Opening deviation analysis |
| endgame_technique | Technique accuracy vs tablebase in endings | Tablebase comparison |
| time_management | Per-phase time distribution | Move time logs |
| tilt_resistance | Performance consistency after errors | Post-blunder eval trend |
| style_profile | Aggression, risk tendency, positional preference | Move complexity distribution |
| weakness_patterns | Specific recurring tactical and positional failures | Clustered error analysis |

## 10.2 Update Mechanism

**Trigger:** After every completed game.

**Algorithm:** Exponential Moving Average

```
new_value = 0.15 * game_result_value + 0.85 * current_value
```

**Minimum sample requirement:** A dimension is not surfaced in the UI or used for personalization until it has 5 or more game samples. Displayed as "collecting data" until threshold is met.

**Confidence score:**

```
confidence = min(1.0, sample_count / 20.0)
```

Full confidence at 20 games. Linear interpolation before that.

**Decay function:** If a dimension has received no game updates in 90 days:

```
decay_per_day = (current_value - 0.5) * 0.005
```

Decays toward neutral (0.5) at 0.5% of the gap per day.

**Conflict handling:** If a new game value deviates more than 2 standard deviations from the running mean, the EMA update is applied normally and the event is written to the audit log. No manual intervention required. The EMA self-corrects over subsequent games.

## 10.3 Weakness Pattern Clustering

After every 5 completed games, an offline clustering job groups error positions by feature similarity. Clusters with 3 or more members become named weakness patterns. These patterns are stored in the weakness_patterns table and used by:
- Live coaching trigger evaluation (Section 3.4)
- Post-game analysis emphasis weighting
- Curriculum Agent study plan generation

---

# 11. UI/UX SYSTEM

## 11.1 Desktop Application

| Component | Technology |
|-----------|-----------|
| Framework | Tauri 2 + React 18 |
| Styling | Tailwind CSS |
| State management | Zustand |
| Board | chessground + Canvas overlay |

**Rationale for Tauri over Electron:**  
Binary size ~5 MB vs ~120 MB. RAM overhead ~50 MB vs ~150 MB. Rust backend provides safe, performant subprocess management for Stockfish and Ollama.

## 11.2 Tauri IPC Architecture

```
React (frontend)
  |
  |-- invoke("analyze_position", { fen })  -->  Tauri command
  |<- on("engine_stream", token_callback)  <--  Tauri event (streaming)
  |
Tauri Core (Rust)
  |
  +-- StockfishManager     Stockfish child process, UCI over stdin/stdout, async channel wrapper
  +-- InferenceClient      Ollama HTTP client (reqwest, async, streaming)
  +-- DatabasePool         PostgreSQL via sqlx, Redis via redis-rs
  +-- LanceDBClient        LanceDB embeddings and retrieval
  +-- OrchestratorService  Pipeline execution and agent dispatch
```

**Communication patterns:**  
- Frontend to backend: Tauri invoke() — typed, async, request/response  
- Backend to frontend: Tauri emit() — streaming LLM tokens, engine updates, live coaching alerts  
- Stockfish: async channel-based UCI wrapper; commands queued, responses parsed from stdout stream

## 11.3 Board System

**Foundation:** chessground (GPL-3.0 npm package — see license note below)  
**Piece sets:** Lichess piece assets from the lila repository (CBurnett SVG and others)  
**Extension:** Custom HTML Canvas overlay on top of chessground for animated arrows, heatmaps, attack maps, and strategic zone highlighting.

**License clarification:** Chessground is distributed under GPL-3.0. Most Lichess piece sets are CC BY-NC-SA 4.0; some are more permissive. For this project — a personal, local, non-distributed desktop application — GPL and CC BY-NC-SA impose no practical constraint. Source-code disclosure and non-commercial clauses apply only to software distribution. If distribution is ever planned, this must be revisited before any release.

**Rationale for chessground:** Building a custom board is a multi-month engineering investment with no competitive advantage. Chessground handles all core board interactions (legal move highlighting, drag/drop, premoves, animations). The canvas overlay provides full visualization control without owning the board implementation.

## 11.4 Application Views

| View | Purpose |
|------|---------|
| Board + Chat | Primary play and coaching interface |
| Post-Game Analysis | Full game review with eval chart and explanation timeline |
| Opening Explorer | Opening tree browser with repertoire management |
| Library | Browse imported books, read with progress tracking, study mode |
| Study Dashboard | User profile, weakness visualization, progress trends |
| Curriculum | Active study plan with session scheduling |
| Knowledge Base | PDF upload, PGN import, ingestion status, corpus inventory |
| Settings | Engine config, model config, inference backend, persona, storage paths |

---

# 12. MULTIMODAL VISUALIZATION SYSTEM

## 12.1 Arrow Overlays

| Arrow Type | Color | Trigger |
|-----------|-------|---------|
| Engine best move | Blue | Always shown in analysis mode |
| Alternative candidates | Gray (opacity = eval loss scaled) | MultiPV output |
| User-played move | Green / Orange / Red | Classification result |
| Suggested plan | Purple animated sequence | Pedagogical Agent plan output |

## 12.2 Board Heatmaps

Three switchable heatmap types.

| Heatmap | Data Source | Update |
|---------|-------------|--------|
| Piece activity | Attacked square count per piece (shakmaty) | Per move, cached by FEN |
| Weak squares | Squares with no pawn cover attacked by opponent | Per move, cached by FEN |
| King danger | Pawn shield gaps + open file proximity to king | Per move, cached by FEN |

Heatmap values are computed in the Rust backend. No engine call required. All values cached per FEN.

## 12.3 Interactive Plan Visualization

When the Pedagogical Agent outputs a multi-move plan, the frontend renders it as a timed, sequential animated arrow series with synchronized text overlay narration.

---

# 13. PLAY SYSTEM

## 13.1 Full Precision Mode

Stockfish at maximum configured depth and threads. For analysis and self-testing only.

## 13.2 Human-like Mode

**Algorithm:** Boltzmann-weighted move selection from Stockfish's top-5 candidates.

```
P(move_i) = exp(-eval_loss_i / T) / sum_j( exp(-eval_loss_j / T) )
```

where eval_loss_i is the centipawn loss vs the best move, and T is the temperature parameter calibrated to produce human-plausible error rates.

| Target ELO | Temperature T |
|-----------|---------------|
| 1200 | 180 |
| 1500 | 120 |
| 1800 | 70 |
| 2000 | 40 |
| 2200 | 20 |

## 13.3 Training Mode

Stockfish selects pedagogically — deliberately creating positions that contain tactical motifs and strategic themes from the user's current weakness profile. This provides practical training material inside real games rather than isolated puzzles.

---

# 14. PERSONALITY SYSTEM

**Scope:** Persona affects the Pedagogical Agent only. All factual outputs (Tactical, Strategic, Theory agents) are persona-independent and never altered by persona settings.

**Default persona:** Modern GM

| Persona | Tone | Vocabulary | Criticism Style | Explanation Depth | Historical References |
|---------|------|-----------|----------------|-------------------|-----------------------|
| Soviet Coach | Demanding, direct | Dense, technical | Unfiltered, stated as fact | Maximum | Heavy |
| Modern GM | Pragmatic, efficient | Mixed technical/casual | Balanced, practical | Standard | Moderate |
| Calm Teacher | Patient, encouraging | Clear, accessible | Constructive, reframed as opportunity | Adaptive to user level | Light |
| Brutal Analyst | Cold, precise | Purely technical | No softening whatsoever | Maximum | Minimal |
| Psychological Mentor | Reflective, process-focused | Plain, empathetic | Reframed as recurring pattern | Metacognitive layer added | Rare |

Persona is implemented as a prompt template injected at Pedagogical Agent invocation. No other agents receive persona context.

---

# 15. EXPLANATION GENERATION SYSTEM

## 15.1 Structured Reasoning Pipeline (Hallucination Mitigation)

The LLM runs only after all of the following are prepared and validated:

```
1. Engine output extracted and parsed            (Feature Extraction Layer)
2. Tactical summary computed                     (Tactical Agent — no LLM)
3. RAG retrieval executed and confidence-scored  (Retrieval System)
4. User profile loaded with confidence scores    (Memory Layer)
5. Persona template loaded                       (Personality System)
```

Every claim the Pedagogical Agent makes is grounded in the above structured inputs. The LLM is not given an open-ended chess question. It is given structured facts and asked to explain them.

## 15.2 Pedagogical Agent Prompt Structure

```
SYSTEM:
  You are a chess coach. Persona: [PERSONA_DESCRIPTION].
  Explain positions using ONLY the provided structured facts.
  Do not assert evaluations, move sequences, or rules not in the facts.
  If a section has low-confidence input, say so explicitly.

INPUT FACTS:
  Engine evaluation: [eval_cp] centipawns at depth [depth]
  Best move: [best_move]
  Candidate moves: [top_5_moves_with_eval_loss]
  Tactical features: [structured_tactics_json]
  Positional features: [structured_positional_json]
  Relevant theory (max 3 chunks): [rag_chunks]
  User weakness note: [weakness_flag if confidence >= 0.5]
  Low-confidence flags: [list if any]

TASK:
  Explain this position at depth [BRIEF | STANDARD | FULL].
  Structure: Hook -> What happened -> Why it matters -> Principle -> Personal note (if applicable).
```

## 15.3 Confidence Communication

When any upstream input is low-confidence (RAG similarity below 0.60, incomplete feature extraction, or user profile confidence below 0.3), the final explanation includes a visible user-facing note:

> "Note: Limited reference material was found for this specific position type. The explanation is based on general principles."

This is surfaced in the UI as a subtle indicator — informative, not alarming.

---

# 16. DATABASE ARCHITECTURE

## 16.1 Technology Decisions

| Store | Technology | Rationale |
|-------|-----------|-----------|
| Structured data | PostgreSQL 16 | Relational integrity, complex queries, game history |
| Vector store | LanceDB | Native local persistence, no wrapper needed, simple API, active development |
| Cache | Redis 7 | Sub-millisecond access for active session state and hot position cache |

**LanceDB over FAISS:** FAISS requires a separate persistence layer (HNSWlib or disk serialization). LanceDB persists natively and has better ergonomics for a local single-user context. No meaningful performance difference at this scale.

## 16.2 Core Database Schema

```sql
CREATE TABLE users (
  id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE games (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id      UUID NOT NULL REFERENCES users(id),
  pgn          TEXT NOT NULL,
  result       TEXT,
  played_at    TIMESTAMP NOT NULL,
  source       TEXT,        -- 'live' | 'imported' | 'training'
  opening_eco  TEXT,
  time_control TEXT,
  created_at   TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE moves (
  id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  game_id        UUID NOT NULL REFERENCES games(id),
  move_number    INTEGER NOT NULL,
  color          TEXT NOT NULL,
  uci_move       TEXT NOT NULL,
  fen_before     TEXT NOT NULL,
  eval_cp        INTEGER,
  eval_cp_after  INTEGER,
  eval_swing     INTEGER,
  move_time_ms   INTEGER,
  classification TEXT,     -- 'best' | 'good' | 'inaccuracy' | 'mistake' | 'blunder'
  created_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE user_profiles (
  user_id              UUID PRIMARY KEY REFERENCES users(id),
  tactical_accuracy    FLOAT NOT NULL DEFAULT 0.5,
  positional_accuracy  FLOAT NOT NULL DEFAULT 0.5,
  opening_knowledge    FLOAT NOT NULL DEFAULT 0.5,
  endgame_technique    FLOAT NOT NULL DEFAULT 0.5,
  time_management      FLOAT NOT NULL DEFAULT 0.5,
  tilt_resistance      FLOAT NOT NULL DEFAULT 0.5,
  style_profile        JSONB NOT NULL DEFAULT '{}',
  sample_counts        JSONB NOT NULL DEFAULT '{}',
  last_updated         TIMESTAMP
);

CREATE TABLE weakness_patterns (
  id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id          UUID NOT NULL REFERENCES users(id),
  pattern_name     TEXT NOT NULL,
  description      TEXT,
  example_fens     TEXT[],
  occurrence_count INTEGER NOT NULL DEFAULT 0,
  last_seen        TIMESTAMP,
  created_at       TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE study_sessions (
  id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id      UUID NOT NULL REFERENCES users(id),
  session_type TEXT,    -- 'post_game' | 'theory' | 'tactics' | 'endgame'
  started_at   TIMESTAMP NOT NULL,
  ended_at     TIMESTAMP,
  notes        TEXT
);

CREATE TABLE user_repertoire (
  user_id      UUID NOT NULL REFERENCES users(id),
  fen          TEXT NOT NULL REFERENCES opening_positions(fen),
  color        TEXT NOT NULL,
  familiarity  FLOAT NOT NULL DEFAULT 0.0,
  last_played  TIMESTAMP,
  PRIMARY KEY (user_id, fen, color)
);

CREATE TABLE opening_positions (
  fen              TEXT PRIMARY KEY,
  eco              TEXT,
  opening_name     TEXT,
  parent_fen       TEXT REFERENCES opening_positions(fen),
  move_from_parent TEXT,
  frequency        INTEGER,
  white_score      FLOAT,
  theory_chunk_ids UUID[],
  created_at       TIMESTAMP NOT NULL DEFAULT now()
);
```

---

# 17. INFERENCE ARCHITECTURE

## 17.1 Runtime: Ollama with llama.cpp Vulkan Backend

**Endpoint:** http://localhost:11434  
**Models loaded:**
- `qwen3:14b-q8_0` — primary reasoning model
- `qwen3:8b-q4_K_M` — fast interaction model
- `nomic-embed-text` — embedding model (always resident, 274 MB)

**API:** `/api/chat` with streaming enabled. Thinking mode controlled via `options.enable_thinking` per request.

**Why Ollama over direct llama.cpp invocation:** Ollama provides model management (download, versioning, VRAM-based load/unload), a stable OpenAI-compatible HTTP API, and automatic Vulkan backend selection on RDNA4. It is the correct abstraction layer here.

**Why Vulkan over ROCm/vLLM on this hardware:**

As of May 2026, vLLM has no native RDNA4 (gfx1201) kernel support. Running vLLM on RDNA4 falls back to FP32 dequantization on every operation. Real-world benchmarks on the RX 9070 XT show llama.cpp with Vulkan is 29% faster in throughput than vLLM ROCm on this GPU. Ollama on RDNA4 uses llama.cpp with Vulkan compute shaders internally, which correctly utilizes RDNA4's hardware.

The `InferenceClient` abstraction (Section 6.6) remains in place. When vLLM ships RDNA4 FP8 kernel support — actively developed by the community — migration is a single config line change.

## 17.2 Embedding Model

**Model:** nomic-embed-text-v1.5  
**Dimensions:** 768  
**Runtime:** Ollama (same process, always loaded)  
**Used for:** All LanceDB ingestion and retrieval queries

## 17.3 Model Management Policy

- Models stored locally and versioned by exact tag (e.g., `qwen3:14b-q8_0`)
- Config specifies exact model tags; no floating `latest` references
- No auto-update; model updates require explicit user action in Settings
- Ollama handles VRAM load/unload automatically per the swap strategy in Section 6.4

---

# 18. HARDWARE CONFIGURATION

## 18.1 Target Hardware (User's Current System)

| Component | Specification |
|-----------|--------------|
| GPU | AMD Radeon RX 9070 XT — 16 GB GDDR6, 256-bit bus, ~640 GB/s bandwidth, RDNA4 (gfx1201) |
| RAM | 32 GB DDR5 |
| Architecture | RDNA4 — FP8 support, 2nd-gen AI Accelerators, 58 TFLOPS FP16 |

## 18.2 VRAM Constraint Analysis

The RX 9070 XT has **16 GB VRAM**. This is a hard constraint that directly drives the model selection in Section 6.

| Model | Quantization | VRAM Required | Fits in 16 GB? |
|-------|-------------|---------------|----------------|
| DeepSeek-R1-32B | Q4_K_M | ~22 GB | No — requires ~6 GB CPU offload, severely degrades token speed |
| Qwen3-14B | Q4_K_M | ~9 GB | Yes — with comfortable headroom |
| Qwen3-14B | Q8_0 | ~15 GB | Yes — tightly, best quality for 14B |
| Qwen3-8B | Q4_K_M | ~5 GB | Yes — very fast |

**Decision:** The 32B model is dropped from the primary configuration. Running 32B with ~6 GB of layers offloaded to CPU RAM produces 3–6 tok/s on offloaded layers, which is unusable for interactive coaching. The model stack is recalibrated to 14B + 8B. See Section 6 for the updated model decisions.

Both 14B (Q8_0, ~15 GB) and 8B (Q4_K_M, ~5 GB) models **cannot run simultaneously** in 16 GB VRAM at the highest quantization. The system uses model swapping, which is managed automatically by Ollama. Swap time between models is approximately 8–12 seconds, which is acceptable given the pipeline types (fast tasks always use the 8B; deep analysis tasks use the 14B and can tolerate the load time).

## 18.3 Inference Runtime: llama.cpp via Ollama (Vulkan Backend)

**Critical finding from RDNA4 research:**

As of May 2026, vLLM has **no native RDNA4 (gfx1201) kernel support**. Running vLLM on RDNA4 falls back to FP32 dequantization on every operation, which is significantly slower than the Vulkan path. Real-world benchmarks on the RX 9070 XT show llama.cpp with Vulkan is **29% faster** in throughput than vLLM ROCm on this specific hardware.

**Inference runtime decision: llama.cpp with Vulkan backend, accessed via Ollama.**

Ollama on RDNA4 uses llama.cpp internally with Vulkan compute shaders, which properly utilize the RDNA4 GPU. This is the production-correct choice for this hardware today.

The `InferenceClient` abstraction (Section 6.5) remains. When vLLM ships RDNA4 FP8 kernel support (in active development by the community), migration requires only a config change.

## 18.4 Storage

| Use | Estimated Size |
|-----|---------------|
| OS and application | ~50 GB |
| LLM models (14B Q8 + 8B Q4) | ~20 GB |
| Embedding model | ~1 GB |
| Vector DB (LanceDB) | ~5–20 GB depending on library size |
| PGN archives and chess books | Variable, user-dependent |
| Minimum recommended | 512 GB NVMe |
| Comfortable | 1 TB NVMe |

---

# 19. TESTING STRATEGY

## 19.1 Engine Integration Tests

- UCI command/response cycle validation
- Eval consistency across repeated analysis of identical positions
- Stockfish subprocess recovery after crash

## 19.2 Feature Extraction Tests

- Known tactical positions (forks, pins, skewers) verified against expected feature output
- Edge cases: stalemate, zugzwang, fortress positions

## 19.3 Pipeline Integration Tests

- Each pipeline type executed with a synthetic OrchestratorContext
- Verified that agent outputs conform to their typed schemas
- Error handling paths tested: engine timeout, LLM timeout, empty RAG results

## 19.4 Memory System Tests

- EMA update correctness across N games with known values
- Confidence score progression verified at sample thresholds
- Decay function verified at 90-day boundary

## 19.5 Regression Suite (Pre-Release)

- Set of 50 annotated positions with expected explanation layer content
- Each pre-release build runs this suite; explanations reviewed for factual grounding
- No automated LLM output scoring — manual review by a rated player

---

# 20. FUTURE SYSTEMS (BACKLOG)

All items below are deferred to post-v1.0 milestones. No implementation scope is defined until v1.0 reaches stable release.

| Feature | Target Milestone | Key Dependencies |
|---------|-----------------|-----------------|
| Voice coaching (TTS + STT) | v2.0 | Whisper local STT + Piper/Kokoro local TTS |
| Chess-specific fine-tune (Qwen3-14B base) | v2.0 | 200+ annotated sessions; QLoRA infrastructure |
| Opening repertoire AI builder | v2.0 | v1.0 opening DB complete and stable |
| Tournament preparation mode | v2.1 | Opponent profiling engine |
| Opponent profiling (PGN import + analysis) | v2.1 | Batch analysis pipeline |
| Self-play analysis loop | v2.2 | Stable training mode |
| Psychological training module | v2.2 | Tilt detection reliable at 50+ game baseline |
| Multi-user support | v3.0 | Schema already multi-user-ready |

---

# 21. TECHNOLOGY STACK SUMMARY

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri 2 + React 18 |
| UI styling | Tailwind CSS |
| Frontend state | Zustand |
| Board | chessground (GPL-3.0) + Lichess piece assets + HTML Canvas overlay |
| Backend language | Rust |
| Chess computation (runtime) | shakmaty (Rust crate) |
| Chess computation (ingestion) | python-chess |
| Primary engine | Stockfish (latest stable) |
| Tablebases | Syzygy (local storage, user-configured path) |
| LLM runtime | Ollama (llama.cpp Vulkan backend on RDNA4) |
| Primary reasoning model | Qwen3-14B Q8_0 — thinking mode enabled |
| Fast interaction model | Qwen3-8B Q4_K_M — thinking mode disabled |
| Embedding model | nomic-embed-text-v1.5 (768-dim, via Ollama) |
| Inference abstraction | InferenceClient trait (Ollama now; vLLM-ready when RDNA4 kernels ship) |
| Relational database | PostgreSQL 16 |
| Vector store | LanceDB |
| Cache | Redis 7 |
| Ingestion pipeline | Python (PyMuPDF, python-chess, sentence-transformers, nomic-embed-text) |
| GPU | AMD Radeon RX 9070 XT — 16 GB GDDR6, RDNA4, Vulkan inference |
| System RAM | 32 GB DDR5 |

---

*End of Document — Version 2.1*
