// ─── Board + Chat View ───
// Clean board + coaching wrapper styled with the new design system.

import { useState } from "react";
import { Board } from "./Board";
import { analyzePosition } from "../../lib/tauriBridge";
import { useAppStore } from "../../stores";
import { Card, CardHeader, CardTitle, CardContent } from "../ui/Card";
import { Button } from "../ui/Button";
import { Input } from "../ui/Input";
import { ScrollArea } from "../ui/ScrollArea";
import { Separator } from "../ui/Separator";
import { cn } from "../../lib/utils";
import { MessageSquare, Send, Sparkles } from "lucide-react";

export function BoardChatView() {
  const { currentFen, setAnalysisResult } = useAppStore();
  const boardOrientation = useAppStore((s) => s.boardOrientation);
  const [chatInput, setChatInput] = useState("");
  const [chatMessages, setChatMessages] = useState<
    Array<{ role: string; content: string }>
  >([]);
  const [analyzing, setAnalyzing] = useState(false);

  const handleAnalyze = async () => {
    setAnalyzing(true);
    try {
      const result = await analyzePosition({
        fen: currentFen,
        depth: 18,
        pipeline_type: "PostGame",
      });
      setAnalysisResult(result.explanation.text, result.engine_eval);
      setChatMessages((prev) => [
        ...prev,
        {
          role: "system",
          content: `Engine: ${result.engine_eval > 0 ? "+" : ""}${(result.engine_eval / 100).toFixed(1)} | Best: ${result.best_move || "N/A"}`,
        },
        { role: "assistant", content: result.explanation.text },
      ]);
    } catch (e) {
      setChatMessages((prev) => [
        ...prev,
        { role: "error", content: `Analysis failed: ${e}` },
      ]);
    } finally {
      setAnalyzing(false);
    }
  };

  const handleSendChat = async () => {
    if (!chatInput.trim()) return;
    const msg = chatInput.trim();
    setChatMessages((prev) => [...prev, { role: "user", content: msg }]);
    setChatInput("");
    // TODO: send to conversational pipeline
  };

  const msgClasses = (role: string) => {
    switch (role) {
      case "user":
        return "bg-primary/10 border-l-2 border-primary text-foreground";
      case "system":
        return "bg-accent/10 text-accent-foreground";
      case "error":
        return "bg-destructive/10 text-destructive";
      default:
        return "bg-muted text-foreground";
    }
  };

  return (
    <div className="flex h-full gap-4 p-4 bg-background">
      {/* Board panel */}
      <div className="flex-1 flex items-center justify-center min-w-0 overflow-hidden">
        <Board
          fen={currentFen}
          orientation={boardOrientation}
          className="h-full aspect-square max-w-full"
        />
      </div>

      {/* Chat / Analysis panel */}
      <Card className="w-80 flex flex-col shrink-0 shadow-subtle border-border">
        <CardHeader className="pb-2 flex flex-row items-center justify-between">
          <CardTitle className="text-sm flex items-center gap-2">
            <MessageSquare className="w-4 h-4 text-primary" />
            Coach
          </CardTitle>
          <Button
            variant="secondary"
            size="sm"
            onClick={handleAnalyze}
            disabled={analyzing}
            icon={<Sparkles className="w-3.5 h-3.5" />}
          >
            {analyzing ? "Analyzing..." : "Analyze"}
          </Button>
        </CardHeader>

        <CardContent className="flex-1 min-h-0 p-0 flex flex-col">
          <ScrollArea className="flex-1 p-3">
            <div className="space-y-3 text-xs">
              {chatMessages.map((msg, i) => (
                <div
                  key={i}
                  className={cn("p-2.5 rounded-md", msgClasses(msg.role))}
                >
                  {msg.content}
                </div>
              ))}
              {chatMessages.length === 0 && (
                <p className="text-muted-foreground text-center py-8 text-xs">
                  Click{" "}
                  <span className="font-semibold text-primary">Analyze</span> to
                  get coaching on the current position.
                </p>
              )}
            </div>
          </ScrollArea>

          <Separator />

          <div className="p-3 flex gap-2">
            <Input
              value={chatInput}
              onChange={(e) => setChatInput(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && handleSendChat()}
              placeholder="Ask the coach..."
              className="flex-1 text-xs"
            />
            <Button
              variant="default"
              size="sm"
              onClick={handleSendChat}
              disabled={!chatInput.trim()}
              icon={<Send className="w-3.5 h-3.5" />}
            >
              Send
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
