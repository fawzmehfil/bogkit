import { describe, expect, it } from "vitest";
import { clampAxis, formatTime } from "./gameMath";

describe("game math", () => {
  it("formats the five minute run clock", () => {
    expect(formatTime(300_000)).toBe("5:00");
    expect(formatTime(59_001)).toBe("1:00");
    expect(formatTime(0)).toBe("0:00");
  });

  it("never sends invalid movement axes", () => {
    expect(clampAxis(2)).toBe(1);
    expect(clampAxis(-2)).toBe(-1);
    expect(clampAxis(Number.NaN)).toBe(0);
  });
});
