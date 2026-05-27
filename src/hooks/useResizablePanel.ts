import { useState, useCallback, useRef, useEffect } from "react";

interface UseResizablePanelOptions {
  /** Initial panel width in pixels */
  initialWidth: number;
  /** Minimum panel width in pixels */
  minWidth?: number;
  /** Maximum panel width in pixels */
  maxWidth?: number;
  /** Direction from which the panel is resized: "left" or "right" */
  side: "left" | "right";
}

interface UseResizablePanelReturn {
  /** Current panel width */
  width: number;
  /** Ref callback to attach to the resize handle element */
  handleRef: (node: HTMLDivElement | null) => void;
  /** Ref callback to attach to the panel element */
  panelRef: (node: HTMLDivElement | null) => void;
  /** Whether the panel is currently being resized */
  isResizing: boolean;
  /** Reset to initial width */
  reset: () => void;
}

export function useResizablePanel({
  initialWidth,
  minWidth = 160,
  maxWidth = 600,
  side,
}: UseResizablePanelOptions): UseResizablePanelReturn {
  const [width, setWidth] = useState(initialWidth);
  const [isResizing, setIsResizing] = useState(false);
  const handleNodeRef = useRef<HTMLDivElement | null>(null);
  const panelNodeRef = useRef<HTMLDivElement | null>(null);
  const startXRef = useRef(0);
  const startWidthRef = useRef(0);

  const onMouseDown = useCallback(
    (e: MouseEvent) => {
      e.preventDefault();
      setIsResizing(true);
      startXRef.current = e.clientX;
      startWidthRef.current = width;
    },
    [width],
  );

  const onMouseMove = useCallback(
    (e: MouseEvent) => {
      if (!isResizing) return;
      const delta = e.clientX - startXRef.current;
      const newWidth =
        side === "left"
          ? startWidthRef.current - delta
          : startWidthRef.current + delta;
      setWidth(Math.max(minWidth, Math.min(maxWidth, newWidth)));
    },
    [isResizing, side, minWidth, maxWidth],
  );

  const onMouseUp = useCallback(() => {
    setIsResizing(false);
  }, []);

  // Callback ref for the handle — manages adding/removing the mousedown listener
  const setHandleRef = useCallback(
    (node: HTMLDivElement | null) => {
      if (handleNodeRef.current) {
        handleNodeRef.current.removeEventListener("mousedown", onMouseDown);
      }
      handleNodeRef.current = node;
      if (node) {
        node.addEventListener("mousedown", onMouseDown);
      }
    },
    [onMouseDown],
  );

  // Callback ref for the panel — just stores the reference
  const setPanelRef = useCallback((node: HTMLDivElement | null) => {
    panelNodeRef.current = node;
  }, []);

  // Global mouse events during resize
  useEffect(() => {
    if (!isResizing) return;
    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "ew-resize";
    document.body.style.userSelect = "none";
    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing, onMouseMove, onMouseUp]);

  const reset = useCallback(() => setWidth(initialWidth), [initialWidth]);

  return {
    width,
    handleRef: setHandleRef,
    panelRef: setPanelRef,
    isResizing,
    reset,
  };
}
