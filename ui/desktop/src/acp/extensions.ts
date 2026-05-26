import type { ExtensionEntry, ExtensionConfig } from '../api';
import { getAcpClient } from './acpConnection';
import { nameToKey } from '../components/settings/extensions/utils';

export interface ConfiguredExtensionsResponse {
  extensions: ExtensionEntry[];
  warnings: string[];
}

/**
 * Fetch all configured extensions via ACP (`_goose/config/extensions`).
 */
export async function getConfiguredExtensions(): Promise<ConfiguredExtensionsResponse> {
  const client = await getAcpClient();
  const response = await client.goose.GooseConfigExtensions({});
  return {
    extensions: response.extensions as ExtensionEntry[],
    warnings: response.warnings ?? [],
  };
}

/**
 * Add (or update) an extension in the user's global goose config via ACP
 * (`_goose/config/extensions/add`).
 */
export async function addConfiguredExtension(
  name: string,
  config: ExtensionConfig,
  enabled: boolean
): Promise<void> {
  const client = await getAcpClient();
  // Server expects a JSON object matching one of the ExtensionConfig variants,
  // and injects `name` itself. We strip `name` from the body to match that shape.
  const extensionConfig = { ...config } as Record<string, unknown>;
  delete extensionConfig.name;

  await client.goose.GooseConfigExtensionsAdd({
    name,
    extensionConfig,
    enabled,
  });
}

/**
 * Remove an extension from the user's global goose config via ACP
 * (`_goose/config/extensions/remove`). The server identifies the entry by
 * `configKey`, which is derived from the extension name.
 */
export async function removeConfiguredExtension(name: string): Promise<void> {
  const client = await getAcpClient();
  await client.goose.GooseConfigExtensionsRemove({
    configKey: nameToKey(name),
  });
}

/**
 * Add an extension to a running session's agent via ACP
 * (`_goose/extensions/add`).
 */
export async function addSessionExtension(
  sessionId: string,
  config: ExtensionConfig
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.GooseExtensionsAdd({
    sessionId,
    config,
  });
}

/**
 * Remove an extension from a running session's agent via ACP
 * (`_goose/extensions/remove`).
 */
export async function removeSessionExtension(
  sessionId: string,
  name: string
): Promise<void> {
  const client = await getAcpClient();
  await client.goose.GooseExtensionsRemove({
    sessionId,
    name,
  });
}

/**
 * Fetch the list of extensions associated with a given session via ACP
 * (`_goose/session/extensions`).
 */
export async function getSessionExtensions(
  sessionId: string
): Promise<ExtensionEntry[]> {
  const client = await getAcpClient();
  const response = await client.goose.GooseSessionExtensions({ sessionId });
  return response.extensions as ExtensionEntry[];
}
