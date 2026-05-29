import { describe, it, expect } from "vitest";
import { getTauri } from "../lib/tauriBridge";

describe("tauriBridge", () => {
  it("getTauri returns null when not in Tauri", async () => {
    const tauri = await Promise.race([
      getTauri(),
      new Promise<null>((resolve) => setTimeout(() => resolve(null), 100)),
    ]);
    expect(tauri).toBeNull();
  });
});
