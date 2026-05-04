import { describe, expect, it } from "vitest";
import { resolvePersonaProvider } from "./resolvePersonaProvider";

const providers = [
  { id: "goose", label: "Goose" },
  { id: "openai", label: "OpenAI" },
  { id: "claude-code", label: "Claude Code" },
];

describe("resolvePersonaProvider", () => {
  it("matches provider ids and labels despite case or spacing", () => {
    expect(resolvePersonaProvider(providers, "Open AI")?.id).toBe("openai");
    expect(resolvePersonaProvider(providers, "CLAUDE_CODE")?.id).toBe(
      "claude-code",
    );
  });

  it("does not return raw stale provider text as a fallback", () => {
    expect(
      resolvePersonaProvider(providers, "retired-provider"),
    ).toBeUndefined();
  });

  it("avoids broad short substring matches", () => {
    expect(resolvePersonaProvider(providers, "ai")).toBeUndefined();
  });
});
