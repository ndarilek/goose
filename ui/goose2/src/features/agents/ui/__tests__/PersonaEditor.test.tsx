import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Persona } from "@/shared/types/agents";
import { PersonaEditor } from "../PersonaEditor";

vi.mock("@/shared/api/acp", () => ({
  discoverAcpProviders: vi.fn().mockResolvedValue([]),
}));

vi.mock("@/features/providers/api/inventory", () => ({
  getProviderInventory: vi.fn().mockResolvedValue([]),
}));

vi.mock("../AvatarDropZone", () => ({
  AvatarDropZone: () => <div data-testid="avatar-drop-zone" />,
}));

function makePersona(overrides: Partial<Persona> = {}): Persona {
  return {
    id: "agent-1",
    displayName: "Original Agent",
    systemPrompt: "Original prompt",
    isBuiltin: false,
    isFromDisk: true,
    sourcePath: "/mock/.agents/agents/original-agent.md",
    createdAt: "2026-04-01T00:00:00.000Z",
    updatedAt: "2026-04-01T00:00:00.000Z",
    ...overrides,
  };
}

const defaultProps = {
  isOpen: true,
  mode: "edit" as const,
  onClose: vi.fn(),
  onSave: vi.fn(),
};

describe("PersonaEditor", () => {
  it("does not overwrite in-progress edits when the same persona refreshes", async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <PersonaEditor {...defaultProps} persona={makePersona()} />,
    );
    const nameInput = screen.getByPlaceholderText("e.g. Code Reviewer");

    await user.clear(nameInput);
    await user.type(nameInput, "Draft Agent");

    rerender(
      <PersonaEditor
        {...defaultProps}
        persona={makePersona({
          displayName: "Refreshed Agent",
          systemPrompt: "Refreshed prompt",
          updatedAt: "2026-04-01T00:01:00.000Z",
        })}
      />,
    );

    expect(nameInput).toHaveValue("Draft Agent");
  });

  it("starts from a fresh snapshot when editing a different persona", async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <PersonaEditor {...defaultProps} persona={makePersona()} />,
    );
    const nameInput = screen.getByPlaceholderText("e.g. Code Reviewer");

    await user.clear(nameInput);
    await user.type(nameInput, "Draft Agent");

    rerender(
      <PersonaEditor
        {...defaultProps}
        persona={makePersona({
          id: "agent-2",
          displayName: "Second Agent",
          systemPrompt: "Second prompt",
        })}
      />,
    );

    expect(nameInput).toHaveValue("Second Agent");
  });

  it("starts from a fresh snapshot when the dialog is reopened", async () => {
    const user = userEvent.setup();
    const persona = makePersona();
    const { rerender } = render(
      <PersonaEditor {...defaultProps} persona={persona} />,
    );
    const nameInput = screen.getByPlaceholderText("e.g. Code Reviewer");

    await user.clear(nameInput);
    await user.type(nameInput, "Draft Agent");

    rerender(
      <PersonaEditor {...defaultProps} persona={persona} isOpen={false} />,
    );
    rerender(<PersonaEditor {...defaultProps} persona={persona} />);

    expect(screen.getByPlaceholderText("e.g. Code Reviewer")).toHaveValue(
      "Original Agent",
    );
  });
});
