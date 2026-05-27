// ─── Coach Chat ───
// Multi-turn conversational coaching panel with streaming responses.
// Supports FEN-based position analysis and pure Q&A about chess concepts.

import { useState, useEffect, useRef, useCallback } from "react";
import { useAppStore } from "../../stores";
import type {
  StreamingTokenEvent,
  ChatMessageRequest,
  ChatMessageResponse,
  ChatHistoryEntry,
} from "../../lib/types";
import { getTauri } from "../../lib/tauriBridge";
import { cn } from "../../lib/utils";
import { Button, Input, ScrollArea, Badge } from "../../components/ui";
import {
  Send,
  Lightbulb,
  Compass,
  BookOpen,
  Zap,
  MessageSquare,
  User,
  Bot,
  AlertTriangle,
  BarChart3,
} from "lucide-react";

// ─── Types ───

interface ChatMessage {
  role: "user" | "assistant" | "system" | "error";
  content: string;
}

interface CoachChatProps {
  fen: string;
}

// ─── Suggestions ───

const SUGGESTIONS = [
  {
    icon: <Lightbulb className="h-3.5 w-3.5" />,
    text: "Explain this position",
  },
  { icon: <Compass className="h-3.5 w-3.5" />, text: "What is the plan here?" },
  { icon: <Zap className="h-3.5 w-3.5" />, text: "Find the best move" },
  { icon: <BookOpen className="h-3.5 w-3.5" />, text: "What opening is this?" },
];

// ─── Component ───

export function CoachChat({ fen }: CoachChatProps) {
  const [msgs, setMsgs] = useState<ChatMessage[]>([]);
  const [history, setHistory] = useState<ChatHistoryEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [inputValue, setInputValue] = useState("");
  const [isAnalyzingFen, setIsAnalyzingFen] = useState(false);

  const streamingRef = useRef("");
  const streamingOccurredRef = useRef(false);
  const msgRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // ── Store streaming state ──
  const streamingTokens = useAppStore((s) => s.streamingTokens);
  const isStreaming = useAppStore((s) => s.isStreaming);
  const appendStreamToken = useAppStore((s) => s.appendStreamToken);
  const setStreaming = useAppStore((s) => s.setStreaming);
  const persona = useAppStore((s) => s.persona);
  const resetAnalysis = useAppStore((s) => s.resetAnalysis);

  // ── Listen for streaming-token events from backend ──
  const addMsgRef = useRef<(content: string) => void>(() => {});
  useEffect(() => {
    addMsgRef.current = addAssistantMessage;
  });

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      const tauri = await getTauri();
      if (!tauri) return;
      const unlistenFn = await tauri.listen<StreamingTokenEvent>(
        "streaming-token",
        (event) => {
          if (event.payload.token) {
            appendStreamToken(event.payload.token);
            streamingOccurredRef.current = true;
          }
          if (event.payload.is_final) {
            const finalText = streamingRef.current + event.payload.token;
            addMsgRef.current(finalText);
            setStreaming(false);
            streamingRef.current = "";
          }
        },
      );
      unlisten = unlistenFn;
    })();
    return () => {
      unlisten?.();
    };
  }, [appendStreamToken, setStreaming]);

  // Track streaming content via ref
  useEffect(() => {
    streamingRef.current = streamingTokens;
  }, [streamingTokens]);

  // Auto-scroll to bottom
  useEffect(() => {
    msgRef.current?.scrollTo({
      top: msgRef.current.scrollHeight,
      behavior: "smooth",
    });
  }, [msgs, streamingTokens]);

  // ── Add assistant message and update history ──
  const addAssistantMessage = useCallback((content: string) => {
    setMsgs((m) => [...m, { role: "assistant", content }]);
    setHistory((h) => [...h, { role: "assistant", content }]);
  }, []);

  // ── Send a chat message ──
  const sendMessage = useCallback(
    async (text: string, includeFen: boolean) => {
      if (!text.trim()) return;

      const userMsg = text.trim();
      setLoading(true);
      setIsAnalyzingFen(includeFen && !!fen);
      resetAnalysis();
      streamingOccurredRef.current = false;
      setStreaming(true);

      // Add user message immediately
      setMsgs((m) => [...m, { role: "user", content: userMsg }]);
      const userHistoryEntry: ChatHistoryEntry = {
        role: "user",
        content: userMsg,
      };
      const updatedHistory = [...history, userHistoryEntry];
      setHistory(updatedHistory);

      setInputValue("");

      try {
        const tauri = await getTauri();
        if (!tauri) {
          setMsgs((m) => [
            ...m,
            {
              role: "assistant",
              content:
                "Coach requires Tauri backend. Run with `npm run tauri:dev`.",
            },
          ]);
          setLoading(false);
          setIsAnalyzingFen(false);
          setStreaming(false);
          return;
        }

        const request: ChatMessageRequest = {
          message: userMsg,
          fen: includeFen ? fen : undefined,
          history: history,
          persona: persona,
        };

        const INVOKE_TIMEOUT_MS = 120_000;
        const r = await Promise.race([
          tauri.invoke<ChatMessageResponse>("cmd_chat_message", {
            request,
          }),
          new Promise<never>((_, reject) =>
            setTimeout(
              () =>
                reject(
                  new Error(
                    "Coach response timed out. Check that Ollama is running.",
                  ),
                ),
              INVOKE_TIMEOUT_MS,
            ),
          ),
        ]);

        if (!streamingOccurredRef.current) {
          addAssistantMessage(r.reply);
        }

        setStreaming(false);
      } catch (e: unknown) {
        setMsgs((m) => [
          ...m,
          {
            role: "error",
            content: `Error: ${e instanceof Error ? e.message : String(e)}`,
          },
        ]);
        setStreaming(false);
      } finally {
        setLoading(false);
        setIsAnalyzingFen(false);
      }
    },
    [fen, history, persona, addAssistantMessage, resetAnalysis, setStreaming],
  );

  // ── Send with FEN context (current position analysis) ──
  const askWithFen = useCallback(() => {
    if (!inputValue.trim()) return;
    sendMessage(inputValue, true);
  }, [inputValue, sendMessage]);

  // ── Handle Enter / Shift+Enter ──
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        sendMessage(inputValue, true);
      }
    },
    [inputValue, sendMessage],
  );

  // ── Render helpers ──
  const positionActive =
    fen && fen !== "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="shrink-0 flex items-center justify-between px-4 py-3 border-b border-border bg-card/30">
        <div className="flex items-center gap-2">
          <MessageSquare className="h-4 w-4 text-primary" />
          <h2 className="text-sm font-semibold text-foreground">Coach</h2>
          <Badge variant="secondary" className="text-[10px]">
            {persona ?? "Default"}
          </Badge>
        </div>
        <Button
          variant="outline"
          size="sm"
          className="h-7 px-2 text-[11px]"
          onClick={() => sendMessage("Analyze this position", true)}
          disabled={loading || isStreaming}
          icon={<BarChart3 className="h-3.5 w-3.5" />}
        >
          Analyze Position
        </Button>
      </div>

      {/* Messages */}
      <ScrollArea className="flex-1" ref={msgRef}>
        <div className="px-4 py-4 space-y-4">
          {msgs.length === 0 && !isStreaming && (
            <div className="flex flex-col items-center justify-center pt-12 pb-8 text-center">
              <div className="flex h-12 w-12 items-center justify-center rounded-full bg-primary/10 mb-4">
                <Bot className="h-6 w-6 text-primary" />
              </div>
              <p className="text-sm font-medium text-foreground mb-1">
                Ask the coach anything about chess
              </p>
              <p className="text-xs text-muted-foreground max-w-xs mb-6">
                Get position analysis, opening explanations, tactical insights,
                and personalized training advice.
              </p>
              <div className="flex flex-wrap gap-2 justify-center max-w-sm">
                {SUGGESTIONS.map((s, i) => (
                  <button
                    key={i}
                    onClick={() => sendMessage(s.text, true)}
                    className={cn(
                      "flex items-center gap-1.5 px-3 py-1.5 text-[11px] rounded-lg border border-border bg-card",
                      "text-muted-foreground hover:text-foreground hover:border-primary/30 transition-colors cursor-pointer",
                    )}
                  >
                    {s.icon}
                    {s.text}
                  </button>
                ))}
              </div>
            </div>
          )}

          {msgs.map((m, i) => (
            <div
              key={i}
              className={cn(
                "flex flex-col",
                m.role === "user" && "items-end",
                m.role === "assistant" && "items-start",
                (m.role === "system" || m.role === "error") && "items-center",
              )}
            >
              {/* Label */}
              <div
                className={cn(
                  "flex items-center gap-1 mb-1",
                  m.role === "user" && "flex-row-reverse",
                  m.role === "assistant" && "flex-row",
                  (m.role === "system" || m.role === "error") &&
                    "justify-center",
                )}
              >
                {m.role === "user" && (
                  <User className="h-3 w-3 text-muted-foreground" />
                )}
                {m.role === "assistant" && (
                  <Bot className="h-3 w-3 text-muted-foreground" />
                )}
                {m.role === "error" && (
                  <AlertTriangle className="h-3 w-3 text-destructive" />
                )}
                <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                  {m.role === "user"
                    ? "You"
                    : m.role === "assistant"
                      ? "Coach"
                      : m.role === "system"
                        ? "System"
                        : "Error"}
                </span>
              </div>

              {/* Bubble */}
              <div
                className={cn(
                  "max-w-[85%] rounded-xl px-3.5 py-2.5 text-xs leading-relaxed",
                  m.role === "user" &&
                    "bg-primary text-primary-foreground rounded-tr-sm",
                  m.role === "assistant" &&
                    "bg-card border border-border text-card-foreground rounded-tl-sm shadow-sm",
                  m.role === "system" &&
                    "bg-accent/10 text-accent-foreground border border-accent/20 rounded-lg",
                  m.role === "error" &&
                    "bg-destructive/10 text-destructive border border-destructive/20 rounded-lg",
                )}
              >
                <span className="whitespace-pre-wrap">{m.content}</span>
              </div>
            </div>
          ))}

          {/* Analyzing position indicator */}
          {isAnalyzingFen && !isStreaming && (
            <div className="flex justify-center">
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-muted/50 text-muted-foreground text-xs">
                <div className="h-1.5 w-1.5 rounded-full bg-accent animate-pulse" />
                Analyzing position with Stockfish...
              </div>
            </div>
          )}

          {/* Live streaming text */}
          {isStreaming && (
            <div className="flex flex-col items-start">
              <div className="flex items-center gap-1 mb-1">
                <Bot className="h-3 w-3 text-muted-foreground" />
                <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
                  Coach
                </span>
              </div>
              <div className="max-w-[85%] rounded-xl rounded-tl-sm px-3.5 py-2.5 text-xs leading-relaxed bg-card border border-border text-card-foreground shadow-sm">
                {streamingTokens && (
                  <span className="whitespace-pre-wrap">{streamingTokens}</span>
                )}
                <span className="inline-block w-0.5 h-4 bg-primary ml-0.5 align-middle animate-blink" />
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Input area */}
      <div className="shrink-0 px-4 py-3 border-t border-border bg-card/30">
        {positionActive && (
          <div className="flex items-center gap-1.5 mb-2">
            <div
              className={cn(
                "h-1.5 w-1.5 rounded-full",
                isAnalyzingFen ? "bg-accent animate-pulse" : "bg-success",
              )}
            />
            <span className="text-[10px] text-muted-foreground">
              Position context active
            </span>
          </div>
        )}
        <div className="flex gap-2">
          <Input
            ref={inputRef}
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              positionActive
                ? "Ask about this position..."
                : "Ask a chess question..."
            }
            disabled={loading || isStreaming}
            className="flex-1"
          />
          <Button
            onClick={askWithFen}
            disabled={loading || isStreaming || !inputValue.trim()}
            size="sm"
            icon={<Send className="h-4 w-4" />}
          >
            Send
          </Button>
        </div>
      </div>
    </div>
  );
}
