import type {
  CreateElicitationRequest,
  CreateElicitationResponse,
  ElicitationContentValue,
  ElicitationSchema,
} from '@agentclientprotocol/sdk';
import { v7 as uuidv7 } from 'uuid';
import { createSessionScopedNotificationRouter } from './sessionScopedNotificationRouter';

type SessionScopedFormElicitationRequest = CreateElicitationRequest & {
  mode: 'form';
  sessionId: string;
  requestedSchema: ElicitationSchema;
};

export interface AcpElicitationRequest {
  id: string;
  sessionId: string;
  request: SessionScopedFormElicitationRequest;
}

interface PendingElicitationRequest {
  request: AcpElicitationRequest;
  resolve: (response: CreateElicitationResponse) => void;
}

const elicitationRequestRouter = createSessionScopedNotificationRouter<AcpElicitationRequest>();
const pendingRequests = new Map<string, PendingElicitationRequest>();

export const subscribeToAcpElicitationRequests = elicitationRequestRouter.subscribe;

export async function requestAcpElicitation(
  request: CreateElicitationRequest
): Promise<CreateElicitationResponse> {
  if (!isSessionScopedFormElicitation(request)) {
    return cancelledElicitationResponse();
  }

  const elicitationRequest: AcpElicitationRequest = {
    id: `acp_elicitation_${uuidv7()}`,
    sessionId: request.sessionId,
    request,
  };
  const key = elicitationRequestKey(elicitationRequest.sessionId, elicitationRequest.id);

  return new Promise<CreateElicitationResponse>((resolve) => {
    pendingRequests.set(key, { request: elicitationRequest, resolve });

    elicitationRequestRouter
      .route(elicitationRequest)
      .then((routed) => {
        if (!routed) {
          const pending = pendingRequests.get(key);
          if (pending?.resolve === resolve) {
            pendingRequests.delete(key);
            resolve(cancelledElicitationResponse());
          }
        }
      })
      .catch((error) => {
        console.warn('Failed to route ACP elicitation request:', error);
        const pending = pendingRequests.get(key);
        if (pending?.resolve === resolve) {
          pendingRequests.delete(key);
          resolve(cancelledElicitationResponse());
        }
      });
  });
}

export function resolveAcpElicitationRequest(
  sessionId: string,
  elicitationId: string,
  userData: Record<string, unknown>
): boolean {
  const key = elicitationRequestKey(sessionId, elicitationId);
  const pending = pendingRequests.get(key);
  if (!pending) {
    return false;
  }

  pendingRequests.delete(key);
  pending.resolve(acceptedElicitationResponse(userData));
  return true;
}

export function cancelAcpElicitationRequestsForSession(sessionId: string): void {
  for (const [key, pending] of pendingRequests) {
    if (pending.request.sessionId === sessionId) {
      pendingRequests.delete(key);
      pending.resolve(cancelledElicitationResponse());
    }
  }
}

function isSessionScopedFormElicitation(
  request: CreateElicitationRequest
): request is SessionScopedFormElicitationRequest {
  return request.mode === 'form' && 'sessionId' in request && typeof request.sessionId === 'string';
}

function acceptedElicitationResponse(userData: Record<string, unknown>): CreateElicitationResponse {
  return {
    action: 'accept',
    content: userData as Record<string, ElicitationContentValue>,
  };
}

function cancelledElicitationResponse(): CreateElicitationResponse {
  return { action: 'cancel' };
}

function elicitationRequestKey(sessionId: string, elicitationId: string): string {
  return `${sessionId}\u0000${elicitationId}`;
}
