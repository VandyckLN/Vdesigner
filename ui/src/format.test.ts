import { describe, expect, it } from "vitest";
import { formatBytes } from "./format";

describe("formatBytes", () => {
  it("mostra bytes crus abaixo de um kilobyte", () => {
    expect(formatBytes(512)).toBe("512 B");
  });

  it("usa kilobytes binários com uma casa decimal", () => {
    expect(formatBytes(2048)).toBe("2.0 KB");
  });

  it("passa para megabytes a partir de 1024 KB", () => {
    expect(formatBytes(1024 * 1024 * 3.5)).toBe("3.5 MB");
  });

  it("devolve um traço para um valor impossível", () => {
    expect(formatBytes(-1)).toBe("—");
  });
});
