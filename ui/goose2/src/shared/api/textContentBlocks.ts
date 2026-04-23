import type {
  ContentAnnotations,
  MessageContent,
  TextContent,
} from "@/shared/types/messages";

export function normalizeTextAnnotations(
  annotations: unknown,
): ContentAnnotations | undefined {
  if (!annotations || typeof annotations !== "object") {
    return undefined;
  }

  const audience = "audience" in annotations ? annotations.audience : undefined;
  const normalizedAudience = Array.isArray(audience)
    ? audience.filter(
        (role): role is "user" | "assistant" =>
          role === "user" || role === "assistant",
      )
    : undefined;

  return normalizedAudience?.length
    ? { audience: normalizedAudience }
    : undefined;
}

export function isUserVisibleTextAnnotations(
  annotations?: ContentAnnotations,
): boolean {
  const audience = annotations?.audience;
  return !audience || audience.length === 0 || audience.includes("user");
}

function sameTextAnnotations(
  left?: ContentAnnotations,
  right?: ContentAnnotations,
): boolean {
  const leftAudience = left?.audience ?? [];
  const rightAudience = right?.audience ?? [];

  return (
    leftAudience.length === rightAudience.length &&
    leftAudience.every((role, index) => role === rightAudience[index])
  );
}

export function appendTextContent(
  content: MessageContent[],
  text: string,
  annotations?: ContentAnnotations,
): MessageContent[] {
  const nextBlock: TextContent = { type: "text", text, annotations };
  const lastContent = content[content.length - 1];

  if (
    lastContent?.type === "text" &&
    sameTextAnnotations(lastContent.annotations, annotations)
  ) {
    return [
      ...content.slice(0, -1),
      {
        ...lastContent,
        text: lastContent.text + text,
      },
    ];
  }

  return [...content, nextBlock];
}
