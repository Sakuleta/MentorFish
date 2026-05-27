// ─── Curriculum View ───
// Active study plan with weekly session scheduling (PRD Section 11.4).

import { useEffect, useCallback } from "react";
import { useAppStore } from "../../stores";
import { getTauri } from "../../lib/tauriBridge";
import type { StudyPlan, StudySession, UserProfile } from "../../lib/types";
import { cn } from "../../lib/utils";
import {
  Button,
  Card,
  CardContent,
  Badge,
  Progress,
  Spinner,
} from "../../components/ui";
import {
  Target,
  Crosshair,
  Crown,
  BookOpen,
  Brain,
  Clock,
  Heart,
  Check,
  Play,
  RotateCw,
  CalendarDays,
} from "lucide-react";

// ─── Icons per focus area ───

const FOCUS_ICONS: Record<string, React.ReactNode> = {
  tactics: <Crosshair className="h-4 w-4" />,
  tactical: <Crosshair className="h-4 w-4" />,
  endgame: <Crown className="h-4 w-4" />,
  opening: <BookOpen className="h-4 w-4" />,
  strategy: <Brain className="h-4 w-4" />,
  positional: <Brain className="h-4 w-4" />,
  time: <Clock className="h-4 w-4" />,
  psychology: <Heart className="h-4 w-4" />,
};

function focusIcon(focus: string): React.ReactNode {
  const key = focus.toLowerCase();
  for (const [k, icon] of Object.entries(FOCUS_ICONS)) {
    if (key.includes(k)) return icon;
  }
  return <BookOpen className="h-4 w-4" />;
}

// ─── Session key for completion tracking ───

function sessionKey(s: StudySession): string {
  return `${s.day}:${s.focus}:${s.description}`;
}

// ─── Focus target definitions (fallback when profile unavailable) ───

interface FocusTarget {
  label: string;
  dimension: keyof Pick<
    UserProfile,
    | "tactical_accuracy"
    | "positional_accuracy"
    | "opening_knowledge"
    | "endgame_technique"
    | "time_management"
    | "tilt_resistance"
  >;
  target: number;
}

const FOCUS_TARGETS: FocusTarget[] = [
  { label: "Tactical Accuracy", dimension: "tactical_accuracy", target: 0.6 },
  {
    label: "Positional Accuracy",
    dimension: "positional_accuracy",
    target: 0.6,
  },
  { label: "Opening Knowledge", dimension: "opening_knowledge", target: 0.5 },
  { label: "Endgame Technique", dimension: "endgame_technique", target: 0.65 },
  { label: "Time Management", dimension: "time_management", target: 0.55 },
  { label: "Tilt Resistance", dimension: "tilt_resistance", target: 0.7 },
];

const DAY_COLORS: Record<string, string> = {
  Monday: "bg-primary/10 text-primary",
  Tuesday: "bg-accent/10 text-accent",
  Wednesday: "bg-success/10 text-success",
  Thursday: "bg-warning/10 text-warning",
  Friday: "bg-destructive/10 text-destructive",
  Saturday: "bg-muted/30 text-muted-foreground",
  Sunday: "bg-secondary text-secondary-foreground",
};

// ─── Component ───

export function CurriculumView() {
  const curriculumPlan = useAppStore((s) => s.curriculumPlan);
  const completedSessions = useAppStore((s) => s.completedSessions);
  const isGeneratingPlan = useAppStore((s) => s.isGeneratingPlan);
  const setCurriculumPlan = useAppStore((s) => s.setCurriculumPlan);
  const toggleSessionComplete = useAppStore((s) => s.toggleSessionComplete);
  const setIsGeneratingPlan = useAppStore((s) => s.setIsGeneratingPlan);

  // ── Load profile & plan on mount ──

  const loadPlan = useCallback(async () => {
    const tauri = await getTauri();
    if (!tauri) return;

    if (curriculumPlan) return;

    setIsGeneratingPlan(true);
    try {
      const plan = await tauri.invoke<StudyPlan>("cmd_generate_curriculum");
      setCurriculumPlan(plan);
    } catch (err) {
      console.error("Failed to generate curriculum:", err);
    } finally {
      setIsGeneratingPlan(false);
    }
  }, [curriculumPlan, setCurriculumPlan, setIsGeneratingPlan]);

  useEffect(() => {
    loadPlan();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // ── Generate new plan (explicit user action) ──

  const handleNewPlan = useCallback(async () => {
    const tauri = await getTauri();
    if (!tauri) return;

    setIsGeneratingPlan(true);
    try {
      const plan = await tauri.invoke<StudyPlan>("cmd_generate_curriculum");
      setCurriculumPlan(plan);
    } catch (err) {
      console.error("Failed to generate curriculum:", err);
    } finally {
      setIsGeneratingPlan(false);
    }
  }, [setCurriculumPlan, setIsGeneratingPlan]);

  // ── Loading state ──

  if (isGeneratingPlan && !curriculumPlan) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-3">
          <Spinner size="lg" />
          <p className="text-sm text-muted-foreground">
            Generating your personalized curriculum...
          </p>
        </div>
      </div>
    );
  }

  // ── Empty state ──

  if (!curriculumPlan) {
    return (
      <div className="flex-1 flex items-center justify-center p-6">
        <div className="text-center max-w-md">
          <CalendarDays className="h-12 w-12 text-muted-foreground/30 mx-auto mb-4" />
          <h2 className="text-lg font-semibold mb-2 text-foreground">
            No Training Plan Yet
          </h2>
          <p className="text-sm text-muted-foreground mb-4">
            Generate a personalized study plan based on your strength and
            weakness profile. Our AI curriculum designer will create focused
            sessions for the week ahead.
          </p>
          <Button
            onClick={handleNewPlan}
            disabled={isGeneratingPlan}
            icon={<RotateCw className="h-4 w-4" />}
          >
            {isGeneratingPlan ? "Generating..." : "Generate Your Plan"}
          </Button>
        </div>
      </div>
    );
  }

  // ── Derived values ──

  const sessions = curriculumPlan.weekly_sessions;
  const totalSessions = sessions.length;
  const completedCount = sessions.filter((s) =>
    completedSessions.includes(sessionKey(s)),
  ).length;
  const progressPercent =
    totalSessions > 0 ? Math.round((completedCount / totalSessions) * 100) : 0;

  return (
    <div className="h-full flex flex-col overflow-hidden">
      {/* Header */}
      <div className="shrink-0 p-5 pb-4 border-b border-border">
        <div className="flex items-start justify-between">
          <div className="min-w-0">
            <h2 className="text-xl font-semibold tracking-tight text-foreground">
              Training Curriculum
            </h2>
            <p className="text-xs text-muted-foreground mt-1 max-w-xl">
              {curriculumPlan.rationale
                ? curriculumPlan.rationale
                : "Based on your weakness profile"}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleNewPlan}
            disabled={isGeneratingPlan}
            className="shrink-0 ml-4"
            icon={
              isGeneratingPlan ? (
                <Spinner size="sm" />
              ) : (
                <RotateCw className="h-3.5 w-3.5" />
              )
            }
          >
            {isGeneratingPlan ? "Generating..." : "New Plan"}
          </Button>
        </div>
      </div>

      {/* Progress */}
      <div className="shrink-0 px-5 py-4 border-b border-border bg-card/30">
        <div className="flex items-center justify-between mb-2">
          <span className="text-xs font-medium text-muted-foreground">
            Weekly Progress
          </span>
          <span className="text-xs font-semibold text-foreground">
            {completedCount}/{totalSessions} sessions
          </span>
        </div>
        <Progress value={progressPercent} />
        <p className="text-[11px] text-muted-foreground mt-1">
          {progressPercent}% complete
        </p>
      </div>

      {/* Session List */}
      <div className="flex-1 overflow-y-auto p-5 space-y-3">
        {sessions.map((session) => {
          const key = sessionKey(session);
          const isCompleted = completedSessions.includes(key);
          const icon = focusIcon(session.focus);
          const dayColor =
            DAY_COLORS[session.day] ?? "bg-muted/30 text-muted-foreground";

          return (
            <Card
              key={key}
              className={cn(
                "transition-all duration-300 border-l-4",
                isCompleted
                  ? "border-l-success bg-success/5 border-border"
                  : "border-l-transparent bg-card border-border hover:border-primary/20",
              )}
            >
              <CardContent className="flex items-start gap-4 p-4">
                {/* Day badge */}
                <div
                  className={cn(
                    "shrink-0 w-10 h-10 rounded-full flex items-center justify-center",
                    dayColor,
                  )}
                >
                  <span className="text-[11px] font-bold">
                    {session.day.slice(0, 3)}
                  </span>
                </div>

                {/* Session content */}
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 mb-1 flex-wrap">
                    <div className="flex items-center gap-1.5">
                      <span className="text-muted-foreground">{icon}</span>
                      <h3 className="text-sm font-semibold text-foreground">
                        {session.focus}
                      </h3>
                    </div>
                    <Badge variant="secondary" className="text-[10px]">
                      {session.duration_minutes} min
                    </Badge>
                    {isCompleted && (
                      <Badge
                        variant="success"
                        className="text-[10px] flex items-center gap-1"
                      >
                        <Check className="h-3 w-3" />
                        Completed
                      </Badge>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed">
                    {session.description}
                  </p>
                </div>

                {/* Complete / Start button */}
                <Button
                  variant={isCompleted ? "outline" : "default"}
                  size="sm"
                  onClick={() => toggleSessionComplete(key)}
                  className="shrink-0"
                  icon={
                    isCompleted ? (
                      <Check className="h-3.5 w-3.5" />
                    ) : (
                      <Play className="h-3.5 w-3.5" />
                    )
                  }
                >
                  {isCompleted ? "Completed" : "Start"}
                </Button>
              </CardContent>
            </Card>
          );
        })}
      </div>

      {/* Footer: Focus Areas */}
      <div className="shrink-0 border-t border-border p-5 pt-4 bg-card/30">
        <div className="flex items-center gap-1.5 mb-3">
          <Target className="h-3.5 w-3.5 text-muted-foreground" />
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            Focus Areas
          </h4>
        </div>
        <div className="grid grid-cols-2 lg:grid-cols-3 gap-3">
          {FOCUS_TARGETS.map((ft) => {
            const cv = (curriculumPlan as unknown as Record<string, unknown>)[
              ft.dimension
            ] as number | undefined;
            const current = cv ?? 0.0;
            const pct = Math.round(current * 100);
            const targetPct = Math.round(ft.target * 100);
            const barPct = Math.min(
              100,
              Math.round((current / ft.target) * 100),
            );

            return (
              <Card key={ft.dimension} className="border-border/60 bg-card p-3">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-[11px] text-muted-foreground">
                    {ft.label}
                  </span>
                  <span className="text-[11px] font-medium text-foreground">
                    {pct}% / {targetPct}%
                  </span>
                </div>
                <Progress
                  value={barPct}
                  className={cn(
                    "h-1.5",
                    barPct >= 80
                      ? "[&>div]:bg-success"
                      : barPct >= 50
                        ? "[&>div]:bg-warning"
                        : "[&>div]:bg-destructive",
                  )}
                />
              </Card>
            );
          })}
        </div>

        {curriculumPlan.rationale && (
          <p className="text-[11px] text-muted-foreground italic leading-relaxed mt-3">
            {curriculumPlan.rationale}
          </p>
        )}
      </div>
    </div>
  );
}
