// ─── App Layout ───
// Navigation sidebar + content area that renders the active view.

import { useEffect, useState, useCallback } from "react";
import {
  Swords,
  BarChart3,
  Globe,
  LayoutDashboard,
  BookOpen,
  Library,
  Brain,
  Settings,
  Fish,
  Sun,
  Moon,
  Keyboard,
  PanelLeft,
} from "lucide-react";
import { PlayView } from "../Play/PlayView";
import { ExplorerView } from "../Explorer/ExplorerView";
import { SettingsView } from "../Settings/SettingsView";
import { LibraryView } from "../Library/LibraryView";
import { KnowledgeBaseView } from "../KnowledgeBase/KnowledgeBaseView";
import { CurriculumView } from "../Curriculum/CurriculumView";
import { OnboardingWizard } from "../Onboarding/OnboardingWizard";
import { DashboardView } from "../Dashboard/DashboardView";
import { AnalysisView } from "../Analysis/AnalysisView";
import { useAppStore } from "../../stores";
import type { View } from "../../stores";
import { getTauri } from "../../lib/tauriBridge";
import type { HealthCheckResponse } from "../../lib/types";
import { Button } from "../ui/Button";
import {
  Tooltip,
  TooltipProvider,
  TooltipTrigger,
  TooltipContent,
} from "../ui/Tooltip";
import { ScrollArea } from "../ui/ScrollArea";
import { cn } from "../../lib/utils";

// ─── Nav items ───

interface NavItem {
  view: View;
  label: string;
  icon: React.ElementType;
}

const NAV_ITEMS: NavItem[] = [
  { view: "board", label: "Play", icon: Swords },
  { view: "analysis", label: "Analysis", icon: BarChart3 },
  { view: "explorer", label: "Explorer", icon: Globe },
  { view: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { view: "curriculum", label: "Curriculum", icon: BookOpen },
  { view: "knowledge", label: "Library", icon: Library },
  { view: "knowledgebase", label: "Knowledge", icon: Brain },
  { view: "settings", label: "Settings", icon: Settings },
];

// ─── Keyboard shortcut labels ───

const SHORTCUTS: Record<View, string> = {
  board: "Ctrl+1",
  analysis: "Ctrl+2",
  explorer: "Ctrl+3",
  dashboard: "Ctrl+4",
  curriculum: "Ctrl+5",
  knowledge: "Ctrl+6",
  knowledgebase: "Ctrl+7",
  settings: "Ctrl+8",
};

// ─── Starting position ───

const STARTING_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

// ─── Health dot indicator ───

function HealthDot({ label, ok }: { label: string; ok: boolean }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <span
          className={cn(
            "inline-block h-2 w-2 shrink-0 rounded-full",
            ok ? "bg-success" : "bg-destructive",
          )}
        />
      </TooltipTrigger>
      <TooltipContent side="top">
        <p className="text-xs">
          {label}: {ok ? "Online" : "Offline"}
        </p>
      </TooltipContent>
    </Tooltip>
  );
}

// ─── Component ───

export function AppLayout() {
  const activeView = useAppStore((s) => s.activeView);
  const setActiveView = useAppStore((s) => s.setActiveView);
  const onboardingCompleted = useAppStore((s) => s.onboardingCompleted);
  const engineStatus = useAppStore((s) => s.engineStatus);
  const inferenceStatus = useAppStore((s) => s.inferenceStatus);
  const databaseStatus = useAppStore((s) => s.databaseStatus);
  const setHealthStatus = useAppStore((s) => s.setHealthStatus);
  const theme = useAppStore((s) => s.theme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);

  const [collapsed, setCollapsed] = useState(() => {
    if (typeof window !== "undefined") {
      return window.innerWidth < 1024;
    }
    return false;
  });

  // ── Responsive auto-collapse ──
  useEffect(() => {
    const onResize = () => {
      if (window.innerWidth < 1024) {
        setCollapsed(true);
      }
    };
    onResize();
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);

  const toggleSidebar = useCallback(() => {
    setCollapsed((c) => !c);
  }, []);

  // ── Health check ──
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const tauri = await getTauri();
      if (!tauri || cancelled) return;
      try {
        const h = await tauri.invoke<HealthCheckResponse>("cmd_health_check");
        if (!cancelled) {
          setHealthStatus(h.engine_ok, h.inference_ok, h.database_ok);
        }
      } catch {
        // Backend not reachable — status stays at store defaults (false).
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [setHealthStatus]);

  // ── Global keyboard shortcuts ──
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const ctrl = e.ctrlKey || e.metaKey;
      if (!ctrl) return;

      switch (e.key) {
        case "1":
          e.preventDefault();
          setActiveView("board");
          break;
        case "2":
          e.preventDefault();
          setActiveView("analysis");
          break;
        case "3":
          e.preventDefault();
          setActiveView("explorer");
          break;
        case "4":
          e.preventDefault();
          setActiveView("dashboard");
          break;
        case "5":
          e.preventDefault();
          setActiveView("curriculum");
          break;
        case "6":
          e.preventDefault();
          setActiveView("knowledge");
          break;
        case "7":
          e.preventDefault();
          setActiveView("knowledgebase");
          break;
        case "8":
          e.preventDefault();
          setActiveView("settings");
          break;
        case "n":
        case "N":
          e.preventDefault();
          useAppStore.getState().setCurrentFen(STARTING_FEN);
          break;
        case "r":
        case "R":
          e.preventDefault();
          {
            const { boardOrientation, setBoardOrientation } =
              useAppStore.getState();
            setBoardOrientation(
              boardOrientation === "white" ? "black" : "white",
            );
          }
          break;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [setActiveView]);

  const renderView = () => {
    switch (activeView) {
      case "board":
        return <PlayView />;
      case "analysis":
        return <AnalysisView />;
      case "explorer":
        return <ExplorerView />;
      case "dashboard":
        return <DashboardView />;
      case "curriculum":
        return <CurriculumView />;
      case "knowledge":
        return <LibraryView />;
      case "knowledgebase":
        return <KnowledgeBaseView />;
      case "settings":
        return <SettingsView />;
    }
  };

  const allOnline = engineStatus && inferenceStatus && databaseStatus;

  return (
    <TooltipProvider delayDuration={200}>
      <div className="flex h-full bg-background text-foreground font-sans">
        {/* ── Onboarding Wizard ── */}
        {!onboardingCompleted && <OnboardingWizard />}

        {/* ── Sidebar ── */}
        <nav
          className={cn(
            "flex flex-col shrink-0 border-r border-sidebar-border bg-sidebar transition-all duration-200 ease-in-out",
            collapsed ? "w-14" : "w-60",
          )}
        >
          {/* Brand */}
          <div
            className={cn(
              "flex h-14 items-center border-b border-sidebar-border",
              collapsed ? "justify-center px-2" : "justify-start px-3",
            )}
          >
            <Fish className="h-6 w-6 shrink-0 text-primary" />
            {!collapsed && (
              <>
                <span className="ml-2 truncate text-base font-bold text-sidebar-foreground">
                  MentorFish
                </span>
                <Button
                  variant="ghost"
                  size="icon"
                  className="ml-auto h-7 w-7 text-muted-foreground hover:text-sidebar-foreground"
                  onClick={toggleSidebar}
                  title="Collapse sidebar"
                >
                  <PanelLeft className="h-4 w-4" />
                </Button>
              </>
            )}
          </div>

          {/* Nav */}
          <ScrollArea className="flex-1">
            <div className="flex flex-col gap-0.5 py-2">
              {NAV_ITEMS.map((item) => {
                const isActive = activeView === item.view;
                const button = (
                  <button
                    key={item.view}
                    onClick={() => setActiveView(item.view)}
                    className={cn(
                      "flex items-center gap-3 rounded-lg py-2.5 text-sm font-medium outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring",
                      collapsed
                        ? "mx-1 justify-center px-0"
                        : "mx-2 justify-start px-3",
                      isActive
                        ? "bg-primary/10 text-primary"
                        : "text-sidebar-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                    title={collapsed ? item.label : undefined}
                  >
                    <item.icon className="h-5 w-5 shrink-0" />
                    {!collapsed && (
                      <>
                        <span className="truncate">{item.label}</span>
                        <span className="ml-auto text-[10px] text-muted-foreground/60">
                          {SHORTCUTS[item.view]}
                        </span>
                      </>
                    )}
                  </button>
                );

                if (collapsed) {
                  return (
                    <Tooltip key={item.view}>
                      <TooltipTrigger asChild>{button}</TooltipTrigger>
                      <TooltipContent side="right">
                        <p className="text-xs">{item.label}</p>
                      </TooltipContent>
                    </Tooltip>
                  );
                }

                return button;
              })}
            </div>
          </ScrollArea>

          {/* Footer */}
          <div className="flex flex-col gap-2 border-t border-sidebar-border p-2">
            {/* Health dots */}
            <div
              className={cn(
                "flex items-center gap-2",
                collapsed ? "justify-center" : "justify-start px-1",
              )}
            >
              <HealthDot label="Engine" ok={engineStatus} />
              <HealthDot label="LLM" ok={inferenceStatus} />
              <HealthDot label="DB" ok={databaseStatus} />
            </div>

            {/* Theme toggle */}
            <Button
              variant="ghost"
              size={collapsed ? "icon" : "sm"}
              onClick={toggleTheme}
              className={cn(
                "w-full text-sidebar-foreground hover:bg-accent hover:text-accent-foreground",
                collapsed
                  ? "h-8 w-8 justify-center self-center p-0"
                  : "justify-start gap-2 px-2",
              )}
              title={theme === "dark" ? "Switch to light" : "Switch to dark"}
            >
              {theme === "dark" ? (
                <Sun className="h-4 w-4 shrink-0" />
              ) : (
                <Moon className="h-4 w-4 shrink-0" />
              )}
              {!collapsed && (
                <span className="text-sm">
                  {theme === "dark" ? "Light" : "Dark"}
                </span>
              )}
            </Button>

            {/* Toggle / Version */}
            <div
              className={cn(
                "flex items-center",
                collapsed ? "justify-center" : "justify-between px-1",
              )}
            >
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-muted-foreground hover:text-sidebar-foreground"
                onClick={toggleSidebar}
                title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
              >
                <PanelLeft className="h-4 w-4" />
              </Button>
              {!collapsed && (
                <span className="text-[10px] text-muted-foreground">
                  v0.2.0
                </span>
              )}
            </div>
          </div>
        </nav>

        {/* ── Content Area ── */}
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background">
          {/* ── Status bar ── */}
          <div className="flex h-8 shrink-0 items-center gap-3 border-b border-border bg-background/80 px-4 text-xs backdrop-blur-sm">
            <div className="flex items-center gap-2">
              {allOnline ? (
                <span className="flex items-center gap-1.5 font-medium text-success">
                  <span className="h-2 w-2 animate-pulse rounded-full bg-success" />
                  All systems online
                </span>
              ) : (
                <>
                  {!engineStatus && (
                    <span className="flex items-center gap-1.5 text-destructive">
                      <span className="h-2 w-2 rounded-full bg-destructive" />
                      Engine offline
                    </span>
                  )}
                  {!inferenceStatus && (
                    <span className="flex items-center gap-1.5 text-destructive">
                      <span className="h-2 w-2 rounded-full bg-destructive" />
                      LLM offline
                    </span>
                  )}
                  {!databaseStatus && (
                    <span className="flex items-center gap-1.5 text-destructive">
                      <span className="h-2 w-2 rounded-full bg-destructive" />
                      DB offline
                    </span>
                  )}
                </>
              )}
            </div>

            <div className="ml-auto flex items-center gap-3 text-muted-foreground">
              <span className="hidden items-center gap-1 sm:flex">
                <Keyboard className="h-3 w-3" />
                <kbd className="rounded border border-border bg-muted px-1 py-0.5 font-mono text-[10px]">
                  ⌘K
                </kbd>
                <span className="text-[10px]">Command Palette</span>
              </span>
            </div>
          </div>

          <div className="flex-1 overflow-auto">{renderView()}</div>
        </main>
      </div>
    </TooltipProvider>
  );
}
