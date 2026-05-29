import { describe, it, expect } from "vitest";
import { cn } from "../lib/utils";

describe("cn utility", () => {
  it("merges class names", () => {
    const result = cn("text-red-500", "text-blue-500");
    expect(result).toBe("text-blue-500");
  });

  it("handles conditional classes", () => {
    const hidden = false;
    const result = cn("base", hidden && "hidden", "end");
    expect(result).toBe("base end");
  });

  it("handles undefined and null", () => {
    const result = cn("base", undefined, null, "end");
    expect(result).toBe("base end");
  });
});
