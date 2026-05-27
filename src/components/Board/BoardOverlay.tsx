// ─── Board Overlay Composition Wrapper ───
//
// Renders the Board together with arrow and heatmap overlays.
// Provides a toggle UI to switch between heatmap types.

import { useState, useCallback } from "react";
import { Board } from "./Board";
import { BoardArrows } from "./BoardArrows";
import { BoardHeatmap } from "./BoardHeatmap";
import { PlanVisualizer } from "./PlanVisualizer";
import { Button } from "../ui/Button";
import { Badge } from "../ui/Badge";
import { cn } from "../../lib/utils";
import { Activity, ShieldAlert, Crown, Target, MoveRight } from "lucide-react";
import type {
  CandidateLine,
  MoveClassification,
  FeatureBundle,
  PlanMove,
} from "../../lib/types";

type HeatmapType = "activity" | "weak_squares" | "king_danger";

const HEATMAP_CONFIG: Record<
  HeatmapType,
  { label: string; icon: React.ReactNode }
> = {
  activity: {
    label: "Activity",
    icon: <Activity className="w-3 h-3" />,
  },
  weak_squares: {
    label: "Weak Sq.",
    icon: <ShieldAlert className="w-3 h-3" />,
  },
  king_danger: {
    label: "King Danger",
    icon: <Crown className="w-3 h-3" />,
  },
};

interface Props {
  fen?: string;
  onMove?: (from: string, to: string) => void;
  lastMove?: [string, string] | null;
  dests?: Map<string, string[]>;
  turnColor?: "white" | "black";

  // ── Arrow overlay props ──
  /** Engine best move in UCI format */
  bestMove?: string | null;
  /** MultiPV candidate lines */
  candidates?: CandidateLine[];
  /** Last user-played move in UCI */
  userMove?: string | null;
  /** Classification of userMove */
  classification?: MoveClassification | null;

  // ── Heatmap props ──
  /** Feature bundle for heatmap rendering */
  featureBundle?: FeatureBundle | null;

  // ── Plan visualization props ──
  /** Multi-move pedagogical plan to visualize */
  plan?: PlanMove[] | null;
  /** Whether the plan visualization is actively playing */
  planPlaying?: boolean;
  /** Called when the plan animation finishes */
  onPlanComplete?: () => void;

  /** CSS class for the outer wrapper */
  className?: string;
  /** CSS class passed through to the inner Board component */
  boardClassName?: string;
}

/**
 * BoardOverlay composes Board + BoardArrows + BoardHeatmap + PlanVisualizer.
 * It manages the chessground API ref and heatmap toggle state.
 */
export function BoardOverlay({
  fen,
  onMove,
  lastMove,
  dests,
  turnColor,
  bestMove,
  candidates,
  userMove,
  classification,
  featureBundle,
  plan,
  planPlaying = false,
  onPlanComplete,
  className,
  boardClassName,
}: Props) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const [cgApi, setCgApi] = useState<any | null>(null);
  const [heatmapType, setHeatmapType] = useState<HeatmapType | null>(null);

  const handleApiReady = useCallback((api: unknown) => {
    setCgApi(api);
  }, []);

  const hasOverlays = !!(
    bestMove ||
    (candidates && candidates.length > 0) ||
    userMove
  );
  const hasHeatmap = !!featureBundle;
  const hasPlan = !!(plan && plan.length > 0);

  return (
    <div
      className={cn(
        "flex flex-col items-center gap-3 w-full h-full",
        className,
      )}
    >
      {/* Board with overlays */}
      <div className="relative flex-1 flex items-center justify-center min-h-0 w-full overflow-hidden">
        <Board
          fen={fen}
          onMove={onMove}
          lastMove={lastMove}
          dests={dests}
          turnColor={turnColor}
          onApiReady={handleApiReady}
          className={boardClassName}
        />

        {/* Invisible overlay components drive chessground imperatively */}
        {cgApi && (
          <>
            <BoardArrows
              api={cgApi}
              bestMove={bestMove}
              candidates={candidates}
              userMove={userMove}
              classification={classification}
            />
            {heatmapType && (
              <BoardHeatmap
                api={cgApi}
                heatmapType={heatmapType}
                featureBundle={featureBundle ?? null}
              />
            )}
            {/* Plan visualizer: draws numbered arrows + narration overlay.
                The key ensures the component remounts (resetting all state)
                whenever playback is toggled or the plan changes. */}
            {hasPlan && (
              <PlanVisualizer
                key={`plan-${planPlaying ? "on" : "off"}-${plan!.length}`}
                api={cgApi}
                plan={plan!}
                isPlaying={planPlaying}
                onComplete={onPlanComplete}
              />
            )}
          </>
        )}
      </div>

      {/* Toggle UI bar */}
      {(hasOverlays || hasHeatmap) && (
        <div className="flex items-center gap-2 flex-wrap justify-center">
          {/* Heatmap toggles */}
          {hasHeatmap && (
            <>
              <span className="text-[10px] text-muted-foreground font-medium mr-1">
                Heatmap
              </span>
              {(Object.keys(HEATMAP_CONFIG) as HeatmapType[]).map((type) => (
                <Button
                  key={type}
                  variant={heatmapType === type ? "default" : "outline"}
                  size="sm"
                  onClick={() =>
                    setHeatmapType((prev) => (prev === type ? null : type))
                  }
                  className={cn(
                    "h-7 text-[10px] gap-1",
                    heatmapType === type && "ring-1 ring-ring",
                  )}
                >
                  {HEATMAP_CONFIG[type].icon}
                  {HEATMAP_CONFIG[type].label}
                </Button>
              ))}
            </>
          )}

          {/* Active overlay indicators */}
          {hasOverlays && (
            <div className="flex items-center gap-1.5 ml-1">
              {bestMove && (
                <Badge variant="secondary" className="text-[10px] gap-1">
                  <Target className="w-3 h-3" />
                  Best
                </Badge>
              )}
              {candidates && candidates.length > 0 && (
                <Badge variant="secondary" className="text-[10px] gap-1">
                  <MoveRight className="w-3 h-3" />
                  Alts
                </Badge>
              )}
              {userMove && (
                <Badge variant="secondary" className="text-[10px] gap-1">
                  <Activity className="w-3 h-3" />
                  Played
                </Badge>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
