// ─── Study Dashboard ───
//
// Per PRD Section 11.4.
// Displays user profile dimensions, weakness patterns,
// recent games, and weekly study plan.

import { useEffect, useState, useCallback, useRef, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { UserProfile, WeaknessPattern } from "../../lib/types";
import { cn } from "../../lib/utils";
import {
  Card,
  CardHeader,
  CardTitle,
  CardContent,
} from "../../components/ui/Card";
import { Badge } from "../../components/ui/Badge";
import { Progress } from "../../components/ui/Progress";
import { Button } from "../../components/ui/Button";
import { Spinner } from "../../components/ui/Spinner";
import {
  RefreshCw,
  Swords,
  TrendingUp,
  Target,
  Flame,
  CalendarDays,
  AlertTriangle,
} from "lucide-react";

// ─── Types for Tauri responses ───

interface UserProfileResponse {
  profile: UserProfile;
}

interface GameSummary {
  game_id: string;
  opponent: string;
  result: string;
  played_at: string;
  opening: string;
  move_count: number;
}

// ─── Dimension display helpers ───

interface DimensionDef {
  key: keyof UserProfile;
  label: string;
  color: string;
}

const DIMENSIONS: DimensionDef[] = [
  { key: "tactical_accuracy", label: "Tactical", color: "#2dd4bf" },
  { key: "positional_accuracy", label: "Positional", color: "#34d399" },
  { key: "opening_knowledge", label: "Opening", color: "#fbbf24" },
  { key: "endgame_technique", label: "Endgame", color: "#a78bfa" },
  { key: "time_management", label: "Time Mgmt", color: "#f87171" },
  { key: "tilt_resistance", label: "Tilt Resist", color: "#38bdf8" },
];

// ─── SVG Radar Chart ───

function SkillRadar({ profile }: { profile: UserProfile }) {
  const dims = DIMENSIONS.map((d) => ({
    ...d,
    value: (profile[d.key] as number) * 100,
  }));

  const cx = 160;
  const cy = 160;
  const r = 130;
  const sides = 6;
  const angleStep = (2 * Math.PI) / sides;
  const startAngle = -Math.PI / 2;

  const gridLevels = [0.25, 0.5, 0.75, 1.0];

  const polarToCartesian = (angle: number, radius: number) => ({
    x: cx + radius * Math.cos(angle),
    y: cy + radius * Math.sin(angle),
  });

  const dataPoints = dims.map((_d, i) => {
    const angle = startAngle + i * angleStep;
    const level = dims[i].value / 100;
    const point = polarToCartesian(angle, r * level);
    return { ...point, angle };
  });

  const dataPolygon = dataPoints.map((p) => `${p.x},${p.y}`).join(" ");

  return (
    <svg viewBox="0 0 320 320" className="w-full h-auto max-w-[320px] mx-auto">
      {gridLevels.map((level) => {
        const points = Array.from({ length: sides }, (_, i) => {
          const angle = startAngle + i * angleStep;
          const p = polarToCartesian(angle, r * level);
          return `${p.x},${p.y}`;
        }).join(" ");
        return (
          <polygon
            key={level}
            points={points}
            fill="none"
            stroke="var(--color-border)"
            strokeWidth="1"
            opacity="0.4"
          />
        );
      })}

      {dims.map((_d, i) => {
        const angle = startAngle + i * angleStep;
        const outer = polarToCartesian(angle, r);
        return (
          <line
            key={i}
            x1={cx}
            y1={cy}
            x2={outer.x}
            y2={outer.y}
            stroke="var(--color-border)"
            strokeWidth="1"
            opacity="0.3"
          />
        );
      })}

      <polygon
        points={dataPolygon}
        fill="var(--color-primary)"
        fillOpacity="0.15"
        stroke="var(--color-primary)"
        strokeWidth="2.5"
      />

      {dataPoints.map((p, i) => (
        <circle
          key={i}
          cx={p.x}
          cy={p.y}
          r="5"
          fill={dims[i].color}
          stroke="var(--color-background)"
          strokeWidth="2"
        />
      ))}

      {dims.map((d, i) => {
        const angle = startAngle + i * angleStep;
        const labelR = r + 28;
        const lp = polarToCartesian(angle, labelR);
        const val = d.value.toFixed(0);
        return (
          <g key={`label-${i}`}>
            <text
              x={lp.x}
              y={lp.y}
              textAnchor="middle"
              dominantBaseline="central"
              fill="var(--color-muted-foreground)"
              fontSize="11"
              fontFamily="var(--font-sans, sans-serif)"
              fontWeight="500"
            >
              {d.label}
            </text>
            <text
              x={lp.x}
              y={lp.y + 15}
              textAnchor="middle"
              dominantBaseline="central"
              fill={d.color}
              fontSize="12"
              fontWeight="700"
              fontFamily="var(--font-mono, monospace)"
            >
              {val}%
            </text>
          </g>
        );
      })}
    </svg>
  );
}

// ─── Stat Card ───

function StatCard({
  title,
  value,
  icon,
  trend,
  trendUp,
}: {
  title: string;
  value: string;
  icon: React.ReactNode;
  trend?: string;
  trendUp?: boolean;
}) {
  return (
    <Card className="hover:shadow-md transition-shadow">
      <CardContent className="p-5">
        <div className="flex items-start justify-between">
          <div className="space-y-1">
            <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              {title}
            </p>
            <p className="text-2xl font-bold text-foreground font-mono">
              {value}
            </p>
            {trend && (
              <p
                className={cn(
                  "text-xs font-medium",
                  trendUp ? "text-success" : "text-muted-foreground",
                )}
              >
                {trend}
              </p>
            )}
          </div>
          <div className="p-2.5 rounded-lg bg-primary/10 text-primary">
            {icon}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// ─── Weakness Pattern Item ───

function WeaknessItem({ pattern }: { pattern: WeaknessPattern }) {
  const severity =
    pattern.occurrence_count >= 5
      ? "destructive"
      : pattern.occurrence_count >= 3
        ? "warning"
        : "secondary";

  return (
    <li className="flex items-center justify-between py-2.5 border-b border-border last:border-b-0">
      <div className="flex-1 min-w-0">
        <span className="text-sm text-foreground truncate block">
          {pattern.pattern_name}
        </span>
        {pattern.description && (
          <span className="text-xs text-muted-foreground truncate block mt-0.5">
            {pattern.description}
          </span>
        )}
      </div>
      <Badge
        variant={severity as "destructive" | "warning" | "secondary"}
        className="shrink-0 ml-3"
      >
        {pattern.occurrence_count}
        {pattern.occurrence_count === 1 ? " time" : " times"}
      </Badge>
    </li>
  );
}

// ─── Game Summary Row ───

function GameRow({ game }: { game: GameSummary }) {
  const resultVariant =
    game.result === "1-0"
      ? "success"
      : game.result === "0-1"
        ? "destructive"
        : "secondary";

  return (
    <div className="flex items-center justify-between py-3 border-b border-border last:border-b-0 hover:bg-muted/50 px-3 -mx-3 rounded-md transition-colors cursor-default group">
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2.5">
          <span className="text-sm font-medium text-foreground truncate">
            vs {game.opponent}
          </span>
          <Badge
            variant={resultVariant as "success" | "destructive" | "secondary"}
          >
            {game.result}
          </Badge>
        </div>
        <div className="text-xs text-muted-foreground mt-0.5">
          {game.opening} · {game.move_count} moves
        </div>
      </div>
      <span className="text-xs text-muted-foreground ml-3 shrink-0 font-mono">
        {game.played_at}
      </span>
    </div>
  );
}

// ─── Main Dashboard View ───

export function DashboardView() {
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [recentGames, setRecentGames] = useState<GameSummary[]>([]);

  const initialized = useRef(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const { profile: p } = await invoke<UserProfileResponse>(
        "cmd_get_user_profile",
      );
      setProfile(p);

      try {
        const games = await invoke<GameSummary[]>("cmd_get_recent_games");
        setRecentGames(games);
      } catch {
        setRecentGames([]);
      }
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to load profile");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (!initialized.current) {
      initialized.current = true;
      loadData();
    }
  }, [loadData]);

  const stats = useMemo(() => {
    const gamesPlayed = recentGames.length;
    const wins = recentGames.filter(
      (g) => g.result === "1-0" || g.result === "0-1",
    ).length; // Simplified
    const winRate =
      gamesPlayed > 0 ? Math.round((wins / gamesPlayed) * 100) : 0;
    const avgAcc = profile
      ? Math.round(
          ((profile.tactical_accuracy +
            profile.positional_accuracy +
            profile.opening_knowledge +
            profile.endgame_technique) /
            4) *
            100,
        )
      : 0;
    return {
      gamesPlayed,
      winRate,
      avgAcc,
      streak: 3, // Placeholder until backend provides real streak
    };
  }, [recentGames, profile]);

  // ─── Loading State ───

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center space-y-3">
          <Spinner size="lg" />
          <p className="text-sm text-muted-foreground">Loading profile…</p>
        </div>
      </div>
    );
  }

  // ─── Error State ───

  if (error || !profile) {
    return (
      <div className="flex-1 flex items-center justify-center p-6">
        <div className="text-center max-w-sm space-y-4">
          <div className="flex items-center justify-center">
            <AlertTriangle className="w-12 h-12 text-muted-foreground/30" />
          </div>
          <div className="space-y-1">
            <p className="text-lg font-semibold text-destructive">
              Failed to Load
            </p>
            <p className="text-sm text-muted-foreground">
              {error ?? "No profile data available"}
            </p>
          </div>
          <Button
            onClick={loadData}
            icon={<RefreshCw className="w-3.5 h-3.5" />}
          >
            Retry
          </Button>
        </div>
      </div>
    );
  }

  // ─── Study Plan Placeholder ───

  const studyPlan = [
    { day: "Mon", activity: "Tactics", duration: "45min" },
    { day: "Tue", activity: "Opening Repertoire", duration: "30min" },
    { day: "Wed", activity: "Endgame", duration: "30min" },
    { day: "Thu", activity: "Tactics", duration: "45min" },
    { day: "Fri", activity: "Opening Repertoire", duration: "40min" },
    { day: "Sat", activity: "Game Review", duration: "60min" },
    { day: "Sun", activity: "Rest", duration: "—" },
  ];

  return (
    <div className="flex-1 overflow-y-auto p-5">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h2 className="text-xl font-bold text-foreground tracking-tight">
            Study Dashboard
          </h2>
          <p className="text-sm text-muted-foreground mt-0.5">
            {new Date().toLocaleDateString("en-US", {
              weekday: "long",
              year: "numeric",
              month: "long",
              day: "numeric",
            })}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={loadData}
          icon={<RefreshCw className="w-3.5 h-3.5" />}
        >
          Refresh
        </Button>
      </div>

      {/* Top Stats Row */}
      <div className="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 mb-6">
        <StatCard
          title="Games Played"
          value={String(stats.gamesPlayed)}
          icon={<Swords className="w-5 h-5" />}
          trend="+2 this week"
          trendUp
        />
        <StatCard
          title="Win Rate"
          value={`${stats.winRate}%`}
          icon={<TrendingUp className="w-5 h-5" />}
          trend="Stable"
          trendUp={false}
        />
        <StatCard
          title="Avg Accuracy"
          value={`${stats.avgAcc}%`}
          icon={<Target className="w-5 h-5" />}
          trend="+4% vs last week"
          trendUp
        />
        <StatCard
          title="Training Streak"
          value={`${stats.streak} days`}
          icon={<Flame className="w-5 h-5" />}
          trend="Keep it up"
          trendUp
        />
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-5">
        {/* Skill Profile */}
        <Card className="xl:col-span-1 lg:row-span-2">
          <CardHeader>
            <CardTitle>Skill Profile</CardTitle>
          </CardHeader>
          <CardContent>
            <SkillRadar profile={profile} />
          </CardContent>
        </Card>

        {/* Weakness Patterns */}
        <Card>
          <CardHeader>
            <CardTitle>Weakness Patterns</CardTitle>
          </CardHeader>
          <CardContent>
            {profile.weakness_patterns.length === 0 ? (
              <div className="py-8 text-center space-y-1">
                <p className="text-sm text-muted-foreground">
                  No weakness patterns detected yet.
                </p>
                <p className="text-xs text-muted-foreground">
                  Play more games to reveal patterns.
                </p>
              </div>
            ) : (
              <ul className="list-none m-0 p-0">
                {[...profile.weakness_patterns]
                  .sort((a, b) => b.occurrence_count - a.occurrence_count)
                  .slice(0, 6)
                  .map((p) => (
                    <WeaknessItem key={p.id} pattern={p} />
                  ))}
              </ul>
            )}
          </CardContent>
        </Card>

        {/* Recent Games */}
        <Card className="lg:col-span-1 xl:col-span-1">
          <CardHeader>
            <CardTitle>Recent Games</CardTitle>
          </CardHeader>
          <CardContent>
            {recentGames.length === 0 ? (
              <div className="py-6 text-center space-y-1">
                <p className="text-sm text-muted-foreground">
                  No games recorded yet.
                </p>
                <p className="text-xs text-muted-foreground">
                  Play a game against the engine to see it here.
                </p>
              </div>
            ) : (
              <div>
                {recentGames.map((g) => (
                  <GameRow key={g.game_id} game={g} />
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Dimension Breakdown */}
        <Card className="lg:col-span-2 xl:col-span-2">
          <CardHeader>
            <CardTitle>Dimension Breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-x-8 gap-y-4">
              {DIMENSIONS.map((d) => {
                const pct = Math.round((profile[d.key] as number) * 100);
                return (
                  <div key={d.key} className="space-y-1.5">
                    <div className="flex justify-between text-xs">
                      <span className="text-muted-foreground font-medium">
                        {d.label}
                      </span>
                      <span
                        className="font-mono font-semibold"
                        style={{ color: d.color }}
                      >
                        {pct}%
                      </span>
                    </div>
                    <Progress value={pct} className="h-2" />
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>

        {/* Weekly Study Plan */}
        <Card className="lg:col-span-2 xl:col-span-3">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <CalendarDays className="w-4 h-4 text-primary" />
              Weekly Study Plan
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-7 gap-3">
              {studyPlan.map((slot) => (
                <div
                  key={slot.day}
                  className={cn(
                    "flex flex-col gap-2 rounded-lg border border-border p-3 transition-colors hover:border-primary/30 hover:bg-muted/30",
                    slot.day === "Sun" && "opacity-60",
                  )}
                >
                  <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                    {slot.day}
                  </span>
                  <div className="flex items-center justify-between gap-2 min-w-0">
                    <Badge
                      variant="outline"
                      className="text-[10px] truncate max-w-[calc(100%-3rem)]"
                    >
                      {slot.activity}
                    </Badge>
                    <span className="text-xs text-muted-foreground font-mono shrink-0">
                      {slot.duration}
                    </span>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
