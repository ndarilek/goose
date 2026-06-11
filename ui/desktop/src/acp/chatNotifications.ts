import type { GooseSessionNotification_unstable } from '@aaif/goose-sdk';
import type { SessionNotification } from '@agentclientprotocol/sdk';

export type AcpSessionUpdateNotification =
  | { type: 'standard'; notification: SessionNotification }
  | { type: 'goose'; notification: GooseSessionNotification_unstable };

export type AcpSessionUpdateListener = (notification: AcpSessionUpdateNotification) => void;

const sessionListeners = new Map<string, Set<AcpSessionUpdateListener>>();

export function subscribeToAcpSessionUpdates(
  sessionId: string,
  listener: AcpSessionUpdateListener
): () => void {
  let listeners = sessionListeners.get(sessionId);
  if (!listeners) {
    listeners = new Set();
    sessionListeners.set(sessionId, listeners);
  }
  listeners.add(listener);

  return () => {
    const currentListeners = sessionListeners.get(sessionId);
    if (!currentListeners) {
      return;
    }
    currentListeners.delete(listener);
    if (currentListeners.size === 0) {
      sessionListeners.delete(sessionId);
    }
  };
}

export function publishAcpSessionUpdate(notification: SessionNotification): void {
  const sessionId = notification.sessionId;
  sessionListeners.get(sessionId)?.forEach((listener) => {
    listener({ type: 'standard', notification });
  });
}

export function publishGooseSessionUpdate(notification: GooseSessionNotification_unstable): void {
  const sessionId = notification.sessionId;
  sessionListeners.get(sessionId)?.forEach((listener) => {
    listener({ type: 'goose', notification });
  });
}
