import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { Persona } from "@/shared/types/agents";
import { AgentDetailPage } from "../AgentDetailPage";

function makePersona(overrides: Partial<Persona> = {}): Persona {
  return {
    id: "p1",
    displayName: "Code Reviewer",
    systemPrompt: "Review code for bugs.",
    isBuiltin: false,
    sourcePath: "/Users/test/.goose/agents/code-review.md",
    createdAt: "2026-04-01T00:00:00.000Z",
    updatedAt: "2026-04-02T00:00:00.000Z",
    ...overrides,
  };
}

function renderDetail(
  persona = makePersona(),
  overrides: Partial<ComponentProps<typeof AgentDetailPage>> = {},
) {
  const props: ComponentProps<typeof AgentDetailPage> = {
    persona,
    onBack: vi.fn(),
    onEdit: vi.fn(),
    onReveal: vi.fn(),
    onStartChat: vi.fn(),
    onCopyFile: vi.fn(),
    onSaveCopy: vi.fn(),
    onDuplicate: vi.fn(),
    onDelete: vi.fn(),
    ...overrides,
  };

  render(<AgentDetailPage {...props} />);
  return props;
}

describe("AgentDetailPage", () => {
  it("shows the skills-style action rail for file-backed agents", async () => {
    const user = userEvent.setup();
    const persona = makePersona();
    const props = renderDetail(persona);

    await user.click(screen.getByRole("button", { name: "Start chat" }));
    await user.click(screen.getByRole("button", { name: "Edit" }));
    await user.click(screen.getByRole("button", { name: "Show in folder" }));

    expect(props.onStartChat).toHaveBeenCalledWith(persona);
    expect(props.onEdit).toHaveBeenCalledWith(persona);
    expect(props.onReveal).toHaveBeenCalledWith(persona);
  });

  it("keeps file sharing actions in the overflow menu", () => {
    renderDetail();

    expect(screen.getByRole("button", { name: "More" })).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Copy file" }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "Save a copy..." }),
    ).not.toBeInTheDocument();
  });
});
