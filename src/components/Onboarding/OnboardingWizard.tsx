// ─── Onboarding Wizard ───
// First-launch walkthrough for new MentorFish users.
// Full-screen modal overlay with 4 steps: Welcome → Skill → Goals → Ready.

import { useState, useCallback, useEffect, useRef } from "react";
import { useAppStore } from "../../stores";
import { cn } from "../../lib/utils";
import { Button } from "../../components/ui/Button";
import { Badge } from "../../components/ui/Badge";
import { Card, CardContent } from "../../components/ui/Card";
import { Separator } from "../../components/ui/Separator";
import { RadioGroup, RadioGroupItem } from "../../components/ui/RadioGroup";
import {
  Fish,
  Swords,
  BarChart3,
  Target,
  BookOpen,
  BrainCircuit,
  ChevronLeft,
  ChevronRight,
  Check,
  Rocket,
  Crown,
  Trophy,
  Puzzle,
  Clock,
  ShieldAlert,
  Compass,
} from "lucide-react";

// ─── Step definitions ───

type SkillLevel = "beginner" | "intermediate" | "advanced" | "expert";

interface SkillOption {
  value: SkillLevel;
  label: string;
  description: string;
  rating: string;
  icon: React.ReactNode;
}

const SKILL_OPTIONS: SkillOption[] = [
  {
    value: "beginner",
    label: "Beginner",
    description: "Knows the rules, still learning basic tactics",
    rating: "< 1000",
    icon: <Puzzle className="w-5 h-5" />,
  },
  {
    value: "intermediate",
    label: "Intermediate",
    description: "Club player, knows basic openings and endgames",
    rating: "1000 – 1600",
    icon: <Target className="w-5 h-5" />,
  },
  {
    value: "advanced",
    label: "Advanced",
    description: "Tournament player, solid in all phases",
    rating: "1600 – 2000",
    icon: <Trophy className="w-5 h-5" />,
  },
  {
    value: "expert",
    label: "Expert",
    description: "2000+ rated, fine-tuning preparation",
    rating: "2000+",
    icon: <Crown className="w-5 h-5" />,
  },
];

interface GoalOption {
  value: string;
  label: string;
  description: string;
  icon: React.ReactNode;
}

const GOAL_OPTIONS: GoalOption[] = [
  {
    value: "tactics",
    label: "Tactics & Calculation",
    description: "Sharpen combinational vision",
    icon: <Zap className="w-4 h-4" />,
  },
  {
    value: "openings",
    label: "Opening Preparation",
    description: "Build a reliable repertoire",
    icon: <BookOpen className="w-4 h-4" />,
  },
  {
    value: "endgame",
    label: "Endgame Technique",
    description: "Convert winning positions",
    icon: <Target className="w-4 h-4" />,
  },
  {
    value: "positional",
    label: "Positional Play",
    description: "Improve strategic understanding",
    icon: <Compass className="w-4 h-4" />,
  },
  {
    value: "time",
    label: "Time Management",
    description: "Move faster under pressure",
    icon: <Clock className="w-4 h-4" />,
  },
  {
    value: "mental",
    label: "Mental Game & Tilt",
    description: "Stay calm and focused",
    icon: <ShieldAlert className="w-4 h-4" />,
  },
];

// ─── Feature cards for Step 1 ───

const FEATURES = [
  {
    icon: <Swords className="w-5 h-5" />,
    text: "Play against Stockfish with adaptive difficulty",
  },
  {
    icon: <BarChart3 className="w-5 h-5" />,
    text: "AI-powered post-game analysis",
  },
  {
    icon: <Target className="w-5 h-5" />,
    text: "Personalized training curriculum",
  },
  {
    icon: <BookOpen className="w-5 h-5" />,
    text: "Opening repertoire builder",
  },
  {
    icon: <BrainCircuit className="w-5 h-5" />,
    text: "Conversational coaching",
  },
];

// ─── Step indicator dots ───

function StepDots({ current, total }: { current: number; total: number }) {
  return (
    <div className="flex items-center justify-center gap-2" aria-hidden="true">
      {Array.from({ length: total }, (_, i) => (
        <div
          key={i}
          className={cn(
            "h-2 rounded-full transition-all duration-300",
            i === current
              ? "bg-primary w-6"
              : i < current
                ? "bg-primary/40 w-2"
                : "bg-border w-2",
          )}
        />
      ))}
    </div>
  );
}

// Need Zap for goal icons — reusing existing imports
function Zap(props: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      {...props}
    >
      <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2" />
    </svg>
  );
}

// ─── Component ───

export function OnboardingWizard() {
  const onboardingCompleted = useAppStore((s) => s.onboardingCompleted);
  const setOnboardingCompleted = useAppStore((s) => s.setOnboardingCompleted);
  const userSkillLevel = useAppStore((s) => s.userSkillLevel);
  const setUserSkillLevel = useAppStore((s) => s.setUserSkillLevel);
  const trainingGoals = useAppStore((s) => s.trainingGoals);
  const setTrainingGoals = useAppStore((s) => s.setTrainingGoals);

  const [step, setStep] = useState(0);
  const [animClass, setAnimClass] = useState("opacity-100 translate-y-0");
  const [localGoals, setLocalGoals] = useState<string[]>(trainingGoals);
  const [localSkill, setLocalSkill] = useState<SkillLevel>(userSkillLevel);
  const [goalError, setGoalError] = useState(false);

  // Ref for focus trapping / initial focus
  const stepRef = useRef<HTMLDivElement>(null);

  // ── Transition helper ──
  const goToStep = useCallback((next: number) => {
    setAnimClass("opacity-0 translate-y-2");
    setTimeout(() => {
      setStep(next);
      setAnimClass("opacity-100 translate-y-0");
      setGoalError(false);
    }, 150);
  }, []);

  const handleNext = useCallback(() => {
    if (step === 2) {
      if (localGoals.length < 1) {
        setGoalError(true);
        return;
      }
      // Persist goals on advance to step 3
      setTrainingGoals(localGoals);
    }
    if (step === 0) {
      // Persist skill on advance from step 1
      setUserSkillLevel(localSkill);
    }
    if (step < 3) {
      goToStep(step + 1);
    }
  }, [
    step,
    localGoals,
    localSkill,
    goToStep,
    setTrainingGoals,
    setUserSkillLevel,
  ]);

  const handleBack = useCallback(() => {
    if (step > 0) {
      goToStep(step - 1);
    }
  }, [step, goToStep]);

  const handleFinish = useCallback(() => {
    setUserSkillLevel(localSkill);
    setTrainingGoals(localGoals);
    setOnboardingCompleted(true);
  }, [
    localSkill,
    localGoals,
    setUserSkillLevel,
    setTrainingGoals,
    setOnboardingCompleted,
  ]);

  // ── Keyboard navigation ──
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        if (step === 3) {
          handleFinish();
        } else {
          handleNext();
        }
      }
      if (e.key === "Escape" && step > 0 && step < 4) {
        e.preventDefault();
        handleBack();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [step, handleNext, handleBack, handleFinish]);

  // ── Focus management ──
  useEffect(() => {
    stepRef.current?.focus();
  }, [step]);

  // ── Toggle goal selection (min 0, max 3) ──
  const toggleGoal = (value: string) => {
    setLocalGoals((prev) => {
      if (prev.includes(value)) {
        return prev.filter((g) => g !== value);
      }
      if (prev.length >= 3) return prev;
      return [...prev, value];
    });
    setGoalError(false);
  };

  // ── Early return if already completed (after all hooks) ──
  if (onboardingCompleted) {
    return null;
  }

  // ── Step rendering ──

  const renderStep = () => {
    switch (step) {
      case 0:
        return (
          <div key="step-0" className="space-y-6">
            {/* Welcome */}
            <div className="text-center space-y-3">
              <div className="flex items-center justify-center">
                <div className="p-4 rounded-2xl bg-primary/10 text-primary">
                  <Fish className="w-10 h-10" />
                </div>
              </div>
              <div className="space-y-1">
                <h2 className="text-2xl font-bold text-foreground tracking-tight">
                  Welcome to MentorFish
                </h2>
                <p className="text-sm text-muted-foreground">
                  Your AI chess mentor
                </p>
              </div>
            </div>

            {/* Feature grid */}
            <div className="grid grid-cols-1 gap-2.5">
              {FEATURES.map((f, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 p-3 rounded-lg bg-muted/50 border border-border/50 text-sm text-foreground/90"
                >
                  <span className="shrink-0 text-primary">{f.icon}</span>
                  <span>{f.text}</span>
                </div>
              ))}
            </div>
          </div>
        );

      case 1:
        return (
          <div key="step-1" className="space-y-5">
            <div className="text-center space-y-1">
              <h2 className="text-xl font-bold text-foreground">
                What&apos;s your chess level?
              </h2>
              <p className="text-sm text-muted-foreground">
                This helps us tailor the difficulty to you.
              </p>
            </div>

            <RadioGroup
              value={localSkill}
              onValueChange={(v) => setLocalSkill(v as SkillLevel)}
              className="grid gap-3"
            >
              {SKILL_OPTIONS.map((opt) => (
                <label
                  key={opt.value}
                  htmlFor={opt.value}
                  className={cn(
                    "flex items-start gap-4 p-4 rounded-xl border-2 cursor-pointer transition-all",
                    localSkill === opt.value
                      ? "border-primary bg-primary/5"
                      : "border-border bg-card hover:border-primary/30",
                  )}
                >
                  <RadioGroupItem
                    value={opt.value}
                    id={opt.value}
                    className="mt-0.5"
                  />
                  <div className="flex items-start gap-3 flex-1">
                    <div
                      className={cn(
                        "p-2 rounded-lg shrink-0",
                        localSkill === opt.value
                          ? "bg-primary/10 text-primary"
                          : "bg-muted text-muted-foreground",
                      )}
                    >
                      {opt.icon}
                    </div>
                    <div className="flex flex-col">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-semibold text-foreground">
                          {opt.label}
                        </span>
                        <Badge
                          variant="secondary"
                          className="text-[10px] font-mono"
                        >
                          {opt.rating}
                        </Badge>
                      </div>
                      <span className="text-xs text-muted-foreground leading-relaxed">
                        {opt.description}
                      </span>
                    </div>
                  </div>
                </label>
              ))}
            </RadioGroup>
          </div>
        );

      case 2:
        return (
          <div key="step-2" className="space-y-5">
            <div className="text-center space-y-1">
              <h2 className="text-xl font-bold text-foreground">
                What do you want to improve?
              </h2>
              <p className="text-sm text-muted-foreground">
                Pick{" "}
                {goalError ? (
                  <span className="text-destructive font-semibold">
                    at least 1
                  </span>
                ) : (
                  "1 to 3 areas"
                )}{" "}
                to focus on (
                <span className="font-medium">{localGoals.length}</span>/3
                selected)
              </p>
            </div>

            <div className="grid grid-cols-1 gap-3">
              {GOAL_OPTIONS.map((opt) => {
                const selected = localGoals.includes(opt.value);
                return (
                  <button
                    key={opt.value}
                    type="button"
                    onClick={() => toggleGoal(opt.value)}
                    className={cn(
                      "flex items-start gap-4 p-4 rounded-xl border-2 text-left cursor-pointer transition-all",
                      selected
                        ? "border-primary bg-primary/5"
                        : "border-border bg-card hover:border-primary/30",
                    )}
                  >
                    <div
                      className={cn(
                        "mt-0.5 shrink-0 w-5 h-5 rounded border-2 flex items-center justify-center transition-colors",
                        selected
                          ? "border-primary bg-primary text-primary-foreground"
                          : "border-border",
                      )}
                    >
                      {selected && <Check className="w-3.5 h-3.5" />}
                    </div>
                    <div className="flex items-start gap-2.5 flex-1">
                      <span
                        className={cn(
                          "shrink-0",
                          selected ? "text-primary" : "text-muted-foreground",
                        )}
                      >
                        {opt.icon}
                      </span>
                      <div className="flex flex-col">
                        <span className="text-sm font-medium text-foreground">
                          {opt.label}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {opt.description}
                        </span>
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>

            {goalError && (
              <p className="text-xs text-destructive text-center">
                Please select at least one training goal.
              </p>
            )}
          </div>
        );

      case 3:
        return (
          <div key="step-3" className="space-y-6">
            <div className="text-center space-y-2">
              <div className="flex items-center justify-center">
                <div className="p-3 rounded-2xl bg-primary/10 text-primary">
                  <Rocket className="w-8 h-8" />
                </div>
              </div>
              <h2 className="text-xl font-bold text-foreground">
                You&apos;re all set!
              </h2>
              <p className="text-sm text-muted-foreground">
                Here&apos;s your personalized training profile.
              </p>
            </div>

            {/* Summary card */}
            <Card className="border-primary/20">
              <CardContent className="p-5 space-y-4">
                {/* Skill level */}
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">
                    Skill Level
                  </span>
                  <Badge
                    variant="default"
                    className="capitalize font-mono text-xs"
                  >
                    {localSkill}
                  </Badge>
                </div>

                <Separator />

                {/* Training goals */}
                <div className="space-y-2">
                  <span className="text-sm text-muted-foreground">
                    Training Goals
                  </span>
                  <div className="flex flex-wrap gap-2">
                    {localGoals.map((g) => {
                      const label =
                        GOAL_OPTIONS.find((o) => o.value === g)?.label ?? g;
                      return (
                        <Badge key={g} variant="secondary" className="text-xs">
                          {label}
                        </Badge>
                      );
                    })}
                  </div>
                </div>
              </CardContent>
            </Card>

            <p className="text-xs text-muted-foreground text-center">
              You can change these anytime in Settings.
            </p>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      {/* ── Modal card ── */}
      <div
        ref={stepRef}
        tabIndex={-1}
        className="w-full max-w-[520px] mx-4 bg-card border border-border rounded-2xl shadow-2xl outline-none"
      >
        {/* Step content */}
        <div className="p-6 pb-4">
          <div className={`transition-all duration-150 ease-out ${animClass}`}>
            {renderStep()}
          </div>
        </div>

        {/* ── Footer: step dots + buttons ── */}
        <div className="px-6 pb-6 space-y-5">
          <StepDots current={step} total={4} />

          <div className="flex items-center justify-between gap-3">
            {/* Back button */}
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={handleBack}
              disabled={step === 0}
              icon={<ChevronLeft className="w-4 h-4" />}
            >
              Back
            </Button>

            {/* Next / Start button */}
            {step < 3 ? (
              <Button type="button" size="sm" onClick={handleNext}>
                Next <ChevronRight className="w-4 h-4 ml-1" />
              </Button>
            ) : (
              <Button
                type="button"
                size="lg"
                onClick={handleFinish}
                icon={<Rocket className="w-4 h-4" />}
              >
                Get Started
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
