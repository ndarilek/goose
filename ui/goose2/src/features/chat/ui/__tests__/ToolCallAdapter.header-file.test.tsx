import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ToolCardDisplay } from "@/features/chat/hooks/ArtifactPolicyContext";
import type { ArtifactPathCandidate } from "@/features/chat/lib/artifactPathPolicy";
import { ToolCallAdapter } from "../ToolCallAdapter";

const mockResolveToolCardDisplay =
  vi.fn<
    (
      args: Record<string, unknown>,
      name: string,
      result?: string,
    ) => ToolCardDisplay
  >();
const mockResolveMarkdownHref =
  vi.fn<(href: string) => ArtifactPathCandidate | null>();
const mockPathExists = vi.fn<(path: string) => Promise<boolean>>();
const mockOpenResolvedPath = vi.fn<(path: string) => Promise<void>>();

vi.mock("@/features/chat/hooks/ArtifactPolicyContext", () => ({
  useArtifactPolicyContext: () => ({
    resolveToolCardDisplay: mockResolveToolCardDisplay,
    resolveMarkdownHref: mockResolveMarkdownHref,
    pathExists: mockPathExists,
    openResolvedPath: mockOpenResolvedPath,
  }),
}));

const EMPTY_DISPLAY: ToolCardDisplay = {
  role: "none",
  primaryCandidate: null,
  secondaryCandidates: [],
};

function makeCandidate(
  overrides: Partial<ArtifactPathCandidate> = {},
): ArtifactPathCandidate {
  return {
    id: "c-1",
    rawPath: "/project/Sources/main.swift",
    resolvedPath: "/project/Sources/main.swift",
    source: "arg_key",
    confidence: "high",
    kind: "file",
    allowed: true,
    blockedReason: null,
    toolCallId: "tool-1",
    toolName: "edit_file",
    toolCallIndex: 0,
    appearanceIndex: 0,
    ...overrides,
  };
}

function renderAdapter(
  overrides: Partial<Parameters<typeof ToolCallAdapter>[0]> = {},
) {
  return render(
    <ToolCallAdapter
      name="Edit main.swift"
      arguments={{ path: "/project/Sources/main.swift", line: 1 }}
      kind="edit"
      status="completed"
      result="Updated /project/Sources/main.swift"
      open={undefined}
      {...overrides}
    />,
  );
}

beforeEach(() => {
  mockResolveToolCardDisplay.mockReset();
  mockResolveToolCardDisplay.mockReturnValue(EMPTY_DISPLAY);
  mockResolveMarkdownHref.mockReset();
  mockResolveMarkdownHref.mockImplementation((href) =>
    makeCandidate({
      rawPath: href,
      resolvedPath: href,
    }),
  );
  mockPathExists.mockReset();
  mockPathExists.mockResolvedValue(true);
  mockOpenResolvedPath.mockReset();
  mockOpenResolvedPath.mockResolvedValue(undefined);
});

describe("ToolCallAdapter header file link", () => {
  it("opens the file from the header filename without expanding the accordion", async () => {
    const user = userEvent.setup();

    renderAdapter();

    await user.click(screen.getByRole("button", { name: /open main\.swift/i }));

    expect(mockOpenResolvedPath).toHaveBeenCalledWith(
      "/project/Sources/main.swift",
    );
    expect(screen.queryByText("Path")).not.toBeInTheDocument();
  });

  it("does not expose a clickable header filename outside allowed roots", async () => {
    const user = userEvent.setup();
    mockResolveMarkdownHref.mockReturnValue(
      makeCandidate({
        rawPath: "/etc/passwd",
        resolvedPath: "/etc/passwd",
        allowed: false,
        blockedReason: "Path is outside allowed project/artifacts roots.",
      }),
    );

    renderAdapter({
      name: "Edit passwd",
      arguments: { path: "/etc/passwd", line: 1 },
      result: "Updated /etc/passwd",
    });

    expect(
      screen.queryByRole("button", { name: /open passwd/i }),
    ).not.toBeInTheDocument();

    await user.click(screen.getByText("passwd"));

    expect(mockOpenResolvedPath).not.toHaveBeenCalled();
    expect(screen.getByText("Path")).toBeInTheDocument();
  });

  it("opens the accordion when clicking the non-file title text", async () => {
    const user = userEvent.setup();

    const { container } = renderAdapter();

    const titlePrefix = container.querySelector("[data-tool-title-prefix]");
    expect(titlePrefix).toBeTruthy();

    if (!titlePrefix) {
      throw new Error("Expected non-file title text");
    }

    await user.click(titlePrefix);

    expect(mockOpenResolvedPath).not.toHaveBeenCalled();
    expect(screen.getByText("Path")).toBeInTheDocument();
  });
});
