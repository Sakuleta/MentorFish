// ─── Settings View ───
// Persona selector, play mode, engine config, knowledge base status.

import { useState } from "react";
import { useAppStore } from "../../stores";
import { getTauri } from "../../lib/tauriBridge";
import { cn } from "../../lib/utils";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardContent,
  CardFooter,
} from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { Badge } from "../../components/ui/Badge";
import { Slider } from "../../components/ui/Slider";
import { Label } from "../../components/ui/Label";
import { Input } from "../../components/ui/Input";
import { Separator } from "../../components/ui/Separator";
import { Switch } from "../../components/ui/Switch";
import { RadioGroup, RadioGroupItem } from "../../components/ui/RadioGroup";
import { ScrollArea } from "../../components/ui/ScrollArea";
import {
  UserCircle,
  Gamepad2,
  Cpu,
  BookOpen,
  User,
  Save,
  FlaskConical,
  MonitorCog,
  GraduationCap,
  Zap,
} from "lucide-react";

// ─── Types ───

type Persona =
  | "Soviet"
  | "ModernGM"
  | "CalmTeacher"
  | "BrutalAnalyst"
  | "PsychologicalMentor";

interface PersonaInfo {
  value: Persona;
  label: string;
  description: string;
  icon: React.ReactNode;
}

const PERSONAS: PersonaInfo[] = [
  {
    value: "Soviet",
    label: "Soviet School",
    description:
      "Disciplined, systematic training approach. Emphasizes endgame technique and positional understanding.",
    icon: <FlaskConical className="w-5 h-5" />,
  },
  {
    value: "ModernGM",
    label: "Modern GM",
    description:
      "Cutting-edge engine-assisted analysis. Focuses on concrete calculation and opening preparation.",
    icon: <MonitorCog className="w-5 h-5" />,
  },
  {
    value: "CalmTeacher",
    label: "Calm Teacher",
    description:
      "Patient, encouraging instruction. Breaks down complex ideas into digestible concepts.",
    icon: <GraduationCap className="w-5 h-5" />,
  },
  {
    value: "BrutalAnalyst",
    label: "Brutal Analyst",
    description:
      "Unfiltered, brutally honest feedback. Highlights every mistake with surgical precision.",
    icon: <Zap className="w-5 h-5" />,
  },
  {
    value: "PsychologicalMentor",
    label: "Psychological Mentor",
    description:
      "Focuses on mental resilience, tilt management, and practical decision-making under pressure.",
    icon: <UserCircle className="w-5 h-5" />,
  },
];

type SectionKey = "coach" | "play" | "engine" | "knowledge" | "account";

interface SectionDef {
  key: SectionKey;
  label: string;
  icon: React.ReactNode;
}

const SECTIONS: SectionDef[] = [
  { key: "coach", label: "Coach", icon: <UserCircle className="w-4 h-4" /> },
  { key: "play", label: "Play", icon: <Gamepad2 className="w-4 h-4" /> },
  { key: "engine", label: "Engine", icon: <Cpu className="w-4 h-4" /> },
  {
    key: "knowledge",
    label: "Knowledge Base",
    icon: <BookOpen className="w-4 h-4" />,
  },
  { key: "account", label: "Account", icon: <User className="w-4 h-4" /> },
];

// ─── Status dot ───

function StatusDot({ ok }: { ok: boolean }) {
  return (
    <span
      className={cn(
        "inline-block w-2.5 h-2.5 rounded-full ring-2 ring-background",
        ok ? "bg-success" : "bg-destructive",
      )}
    />
  );
}

// ─── Component ───

export function SettingsView() {
  const persona = useAppStore((s) => s.persona);
  const setPersona = useAppStore((s) => s.setPersona);
  const playMode = useAppStore((s) => s.playMode);
  const setPlayMode = useAppStore((s) => s.setPlayMode);
  const playStrength = useAppStore((s) => s.playStrength);
  const setPlayStrength = useAppStore((s) => s.setPlayStrength);
  const engineStatus = useAppStore((s) => s.engineStatus);
  const engineThreads = useAppStore((s) => s.engineThreads);
  const setEngineThreads = useAppStore((s) => s.setEngineThreads);
  const engineHash = useAppStore((s) => s.engineHash);
  const setEngineHash = useAppStore((s) => s.setEngineHash);
  const engineDepth = useAppStore((s) => s.engineDepth);
  const setEngineDepth = useAppStore((s) => s.setEngineDepth);
  const engineMultiPv = useAppStore((s) => s.engineMultiPv);
  const setEngineMultiPv = useAppStore((s) => s.setEngineMultiPv);
  const theme = useAppStore((s) => s.theme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);

  const [activeSection, setActiveSection] = useState<SectionKey>("coach");
  const [ingesting, setIngesting] = useState(false);
  const [saveIndicator, setSaveIndicator] = useState(false);

  const handleIngest = async () => {
    setIngesting(true);
    try {
      const tauri = await getTauri();
      if (tauri) {
        await tauri.invoke("cmd_run_ingestion");
      }
    } catch {
      // Ingestion failed — the button re-enables so the user can retry.
    } finally {
      setIngesting(false);
    }
  };

  const showSaved = () => {
    setSaveIndicator(true);
    setTimeout(() => setSaveIndicator(false), 1500);
  };

  const renderSection = () => {
    switch (activeSection) {
      case "coach":
        return (
          <div className="space-y-4 animate-fade-in">
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-1">
                Coach Persona
              </h3>
              <p className="text-xs text-muted-foreground">
                Choose the coaching style that matches how you learn best.
              </p>
            </div>
            <RadioGroup
              value={persona}
              onValueChange={(v) => {
                setPersona(v);
                showSaved();
              }}
              className="grid gap-3"
            >
              {PERSONAS.map((p) => (
                <label
                  key={p.value}
                  htmlFor={p.value}
                  className={cn(
                    "flex items-start gap-4 p-4 rounded-xl border-2 cursor-pointer transition-all",
                    persona === p.value
                      ? "border-primary bg-primary/5"
                      : "border-border bg-card hover:border-primary/30",
                  )}
                >
                  <RadioGroupItem
                    value={p.value}
                    id={p.value}
                    className="mt-0.5"
                  />
                  <div className="flex items-start gap-3">
                    <div
                      className={cn(
                        "p-2 rounded-lg",
                        persona === p.value
                          ? "bg-primary/10 text-primary"
                          : "bg-muted text-muted-foreground",
                      )}
                    >
                      {p.icon}
                    </div>
                    <div className="flex flex-col">
                      <span className="text-sm font-semibold text-foreground">
                        {p.label}
                      </span>
                      <span className="text-xs text-muted-foreground leading-relaxed">
                        {p.description}
                      </span>
                    </div>
                  </div>
                </label>
              ))}
            </RadioGroup>
          </div>
        );

      case "play":
        return (
          <div className="space-y-4 animate-fade-in">
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-1">
                Play Mode
              </h3>
              <p className="text-xs text-muted-foreground">
                Configure how Stockfish behaves during practice games.
              </p>
            </div>
            <RadioGroup
              value={playMode}
              onValueChange={(v) => {
                setPlayMode(v as "full" | "human" | "training");
                showSaved();
              }}
              className="grid gap-3"
            >
              {(
                [
                  {
                    value: "full" as const,
                    label: "Full Precision",
                    description:
                      "Full strength Stockfish — uncompromising, top-level play.",
                  },
                  {
                    value: "human" as const,
                    label: "Human-like",
                    description:
                      "Boltzmann selection with adjustable ELO strength (1200–2200).",
                  },
                  {
                    value: "training" as const,
                    label: "Training",
                    description:
                      "Pedagogical position selection for focused learning.",
                  },
                ] as const
              ).map((mode) => (
                <label
                  key={mode.value}
                  htmlFor={mode.value}
                  className={cn(
                    "flex items-start gap-4 p-4 rounded-xl border-2 cursor-pointer transition-all",
                    playMode === mode.value
                      ? "border-primary bg-primary/5"
                      : "border-border bg-card hover:border-primary/30",
                  )}
                >
                  <RadioGroupItem
                    value={mode.value}
                    id={mode.value}
                    className="mt-0.5"
                  />
                  <div className="flex flex-col">
                    <span className="text-sm font-semibold text-foreground">
                      {mode.label}
                    </span>
                    <span className="text-xs text-muted-foreground leading-relaxed">
                      {mode.description}
                    </span>
                  </div>
                </label>
              ))}
            </RadioGroup>

            {playMode === "human" && (
              <Card className="mt-3 border-primary/20">
                <CardContent className="p-4 space-y-3">
                  <div className="flex items-center justify-between">
                    <Label className="text-sm">Engine Strength</Label>
                    <Badge variant="secondary" className="font-mono">
                      {playStrength} ELO
                    </Badge>
                  </div>
                  <Slider
                    min={1200}
                    max={2200}
                    step={50}
                    value={[playStrength]}
                    onValueChange={(v) => {
                      setPlayStrength(v[0]);
                      showSaved();
                    }}
                  />
                  <div className="flex justify-between text-[10px] text-muted-foreground font-mono">
                    <span>1200</span>
                    <span>2200</span>
                  </div>
                </CardContent>
              </Card>
            )}
          </div>
        );

      case "engine":
        return (
          <div className="space-y-4 animate-fade-in">
            <div className="flex items-center gap-2">
              <StatusDot ok={engineStatus} />
              <h3 className="text-sm font-semibold text-foreground">
                Engine Configuration
              </h3>
            </div>
            <p className="text-xs text-muted-foreground">
              Fine-tune Stockfish parameters for your hardware.
            </p>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="threads">Stockfish Threads</Label>
                <Input
                  id="threads"
                  type="number"
                  min={1}
                  max={16}
                  value={engineThreads}
                  onChange={(e) => {
                    setEngineThreads(
                      Math.max(1, Math.min(16, parseInt(e.target.value) || 1)),
                    );
                    showSaved();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="hash">Hash (MB)</Label>
                <Input
                  id="hash"
                  type="number"
                  min={64}
                  max={8192}
                  step={64}
                  value={engineHash}
                  onChange={(e) => {
                    setEngineHash(
                      Math.max(
                        64,
                        Math.min(8192, parseInt(e.target.value) || 64),
                      ),
                    );
                    showSaved();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="depth">Analysis Depth (plies)</Label>
                <Input
                  id="depth"
                  type="number"
                  min={8}
                  max={99}
                  value={engineDepth}
                  onChange={(e) => {
                    setEngineDepth(
                      Math.max(8, Math.min(99, parseInt(e.target.value) || 8)),
                    );
                    showSaved();
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="multipv">MultiPV Lines</Label>
                <Input
                  id="multipv"
                  type="number"
                  min={1}
                  max={10}
                  value={engineMultiPv}
                  onChange={(e) => {
                    setEngineMultiPv(
                      Math.max(1, Math.min(10, parseInt(e.target.value) || 1)),
                    );
                    showSaved();
                  }}
                />
              </div>
            </div>
          </div>
        );

      case "knowledge":
        return (
          <div className="space-y-4 animate-fade-in">
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-1">
                Knowledge Base
              </h3>
              <p className="text-xs text-muted-foreground">
                Manage your training library and RAG corpus.
              </p>
            </div>

            <Card>
              <CardHeader>
                <CardTitle className="text-sm">Corpus Statistics</CardTitle>
                <CardDescription>
                  Current state of the embedded knowledge base.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">Books indexed</span>
                  <span className="font-mono font-medium text-foreground">
                    50
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">
                    Opening positions
                  </span>
                  <span className="font-mono font-medium text-foreground">
                    30,043
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">
                    Knowledge chunks
                  </span>
                  <span className="font-mono font-medium text-foreground">
                    152,997
                  </span>
                </div>
              </CardContent>
              <CardFooter>
                <Button
                  onClick={handleIngest}
                  disabled={ingesting}
                  loading={ingesting}
                  className="w-full sm:w-auto"
                >
                  {ingesting ? "Ingesting..." : "Re-Ingest Knowledge Base"}
                </Button>
              </CardFooter>
            </Card>
          </div>
        );

      case "account":
        return (
          <div className="space-y-4 animate-fade-in">
            <div>
              <h3 className="text-sm font-semibold text-foreground mb-1">
                Account & Appearance
              </h3>
              <p className="text-xs text-muted-foreground">
                Manage your preferences and display settings.
              </p>
            </div>

            <Card>
              <CardContent className="p-4">
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label htmlFor="theme-toggle" className="text-sm">
                      Dark Mode
                    </Label>
                    <p className="text-xs text-muted-foreground">
                      Toggle between light and dark appearance.
                    </p>
                  </div>
                  <Switch
                    id="theme-toggle"
                    checked={theme === "dark"}
                    onCheckedChange={() => toggleTheme()}
                  />
                </div>
              </CardContent>
            </Card>
          </div>
        );

      default:
        return null;
    }
  };

  return (
    <div className="flex h-full">
      {/* Left Sidebar */}
      <aside className="w-56 border-r border-border bg-card/50 shrink-0 hidden md:flex flex-col">
        <div className="p-4 border-b border-border">
          <h2 className="text-lg font-bold text-foreground">Settings</h2>
        </div>
        <ScrollArea className="flex-1">
          <nav className="p-2 space-y-1">
            {SECTIONS.map((section) => (
              <button
                key={section.key}
                onClick={() => setActiveSection(section.key)}
                className={cn(
                  "w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer",
                  activeSection === section.key
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground",
                )}
              >
                {section.icon}
                {section.label}
              </button>
            ))}
          </nav>
        </ScrollArea>
      </aside>

      {/* Mobile nav */}
      <div className="md:hidden w-full border-b border-border bg-card/50 p-2 flex gap-1 overflow-x-auto">
        {SECTIONS.map((section) => (
          <button
            key={section.key}
            onClick={() => setActiveSection(section.key)}
            className={cn(
              "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors cursor-pointer whitespace-nowrap",
              activeSection === section.key
                ? "bg-primary/10 text-primary"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
          >
            {section.icon}
            {section.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-2xl mx-auto p-5 space-y-6">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-foreground md:hidden">
              Settings
            </h2>
            {saveIndicator && (
              <Badge
                variant="success"
                className="animate-fade-in flex items-center gap-1"
              >
                <Save className="w-3 h-3" />
                Saved
              </Badge>
            )}
          </div>

          <Separator />

          {renderSection()}
        </div>
      </main>
    </div>
  );
}
