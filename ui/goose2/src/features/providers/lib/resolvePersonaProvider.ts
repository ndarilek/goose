import type { AcpProvider } from "@/shared/api/acp";

function normalizeProviderText(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[\s_-]+/g, "");
}

export function resolvePersonaProvider(
  providers: AcpProvider[],
  personaProvider?: string | null,
): AcpProvider | undefined {
  const rawProvider = personaProvider?.trim();
  if (!rawProvider) {
    return undefined;
  }

  const normalizedProvider = normalizeProviderText(rawProvider);
  const exactMatch = providers.find(
    (provider) =>
      normalizeProviderText(provider.id) === normalizedProvider ||
      normalizeProviderText(provider.label) === normalizedProvider,
  );
  if (exactMatch) {
    return exactMatch;
  }

  if (normalizedProvider.length < 3) {
    return undefined;
  }

  return providers.find((provider) =>
    normalizeProviderText(provider.label).includes(normalizedProvider),
  );
}
