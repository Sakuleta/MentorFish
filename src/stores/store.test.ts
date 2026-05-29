import { describe, it, expect, beforeEach } from "vitest";
import { useAppStore } from "../stores";

describe("useAppStore", () => {
  beforeEach(() => {
    useAppStore.setState({
      currentFen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
      boardOrientation: "white",
      activeView: "dashboard",
    });
  });

  it("has correct initial FEN", () => {
    const { currentFen } = useAppStore.getState();
    expect(currentFen).toBe(
      "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    );
  });

  it("can set FEN", () => {
    const newFen =
      "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    useAppStore.getState().setCurrentFen(newFen);
    expect(useAppStore.getState().currentFen).toBe(newFen);
  });

  it("can toggle board orientation", () => {
    expect(useAppStore.getState().boardOrientation).toBe("white");
    useAppStore.getState().setBoardOrientation("black");
    expect(useAppStore.getState().boardOrientation).toBe("black");
  });

  it("can set analysis result", () => {
    useAppStore
      .getState()
      .setAnalysisResult("Good move! You controlled the center.", 35);
    expect(useAppStore.getState().lastExplanation).toBe(
      "Good move! You controlled the center.",
    );
    expect(useAppStore.getState().engineEval).toBe(35);
  });

  it("can set navigation view", () => {
    const { setActiveView } = useAppStore.getState();
    setActiveView("analysis");
    expect(useAppStore.getState().activeView).toBe("analysis");
    setActiveView("dashboard");
    expect(useAppStore.getState().activeView).toBe("dashboard");
  });

  it("can set streaming state", () => {
    const { setStreaming } = useAppStore.getState();
    setStreaming(true);
    expect(useAppStore.getState().isStreaming).toBe(true);
    setStreaming(false);
    expect(useAppStore.getState().isStreaming).toBe(false);
  });

  it("can set persona", () => {
    const { setPersona } = useAppStore.getState();
    setPersona("SovietCoach");
    expect(useAppStore.getState().persona).toBe("SovietCoach");
  });

  it("can set skill level", () => {
    const { setUserSkillLevel } = useAppStore.getState();
    setUserSkillLevel("advanced");
    expect(useAppStore.getState().userSkillLevel).toBe("advanced");
  });

  it("can set play mode", () => {
    const { setPlayMode } = useAppStore.getState();
    setPlayMode("training");
    expect(useAppStore.getState().playMode).toBe("training");
  });

  it("can append streaming tokens", () => {
    const { appendStreamToken, setStreaming } = useAppStore.getState();
    setStreaming(true);
    appendStreamToken("Hello ");
    appendStreamToken("world");
    expect(useAppStore.getState().streamingTokens).toBe("Hello world");
  });

  it("can reset analysis", () => {
    useAppStore
      .getState()
      .setAnalysisResult("Some explanation", 50);
    expect(useAppStore.getState().lastExplanation).toBe("Some explanation");
    useAppStore.getState().resetAnalysis();
    expect(useAppStore.getState().lastExplanation).toBeNull();
    expect(useAppStore.getState().engineEval).toBeNull();
  });
});
