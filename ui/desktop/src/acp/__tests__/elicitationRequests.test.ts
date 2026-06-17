import type { CreateElicitationRequest } from '@agentclientprotocol/sdk';
import { describe, expect, it } from 'vitest';
import {
  cancelAcpElicitationRequestsForSession,
  requestAcpElicitation,
  resolveAcpElicitationRequest,
  subscribeToAcpElicitationRequests,
  type AcpElicitationRequest,
} from '../elicitationRequests';

const SESSION_ID = 'session-1';

function formRequest(): CreateElicitationRequest {
  return {
    mode: 'form',
    sessionId: SESSION_ID,
    message: 'Choose a project',
    requestedSchema: {
      type: 'object',
      properties: {
        project: {
          type: 'string',
        },
      },
      required: ['project'],
    },
  };
}

function waitForRequestRouting(): Promise<void> {
  return Promise.resolve();
}

describe('ACP elicitation requests', () => {
  it('routes form requests and resolves accepted responses', async () => {
    let routedRequest: AcpElicitationRequest | undefined;
    const unsubscribe = subscribeToAcpElicitationRequests(SESSION_ID, (request) => {
      routedRequest = request;
    });

    try {
      const responsePromise = requestAcpElicitation(formRequest());
      await waitForRequestRouting();

      expect(routedRequest).toBeDefined();
      expect(routedRequest?.id).toMatch(/^acp_elicitation_/);
      expect(routedRequest?.request.message).toBe('Choose a project');

      expect(
        resolveAcpElicitationRequest(SESSION_ID, routedRequest!.id, {
          project: 'goose',
        })
      ).toBe(true);

      await expect(responsePromise).resolves.toEqual({
        action: 'accept',
        content: {
          project: 'goose',
        },
      });
    } finally {
      unsubscribe();
    }
  });

  it('cancels form requests without a session subscriber', async () => {
    await expect(requestAcpElicitation(formRequest())).resolves.toEqual({ action: 'cancel' });
  });

  it('cancels pending requests for a session', async () => {
    let routedRequest: AcpElicitationRequest | undefined;
    const unsubscribe = subscribeToAcpElicitationRequests(SESSION_ID, (request) => {
      routedRequest = request;
    });

    try {
      const responsePromise = requestAcpElicitation(formRequest());
      await waitForRequestRouting();

      expect(routedRequest).toBeDefined();
      cancelAcpElicitationRequestsForSession(SESSION_ID);

      await expect(responsePromise).resolves.toEqual({ action: 'cancel' });
      expect(resolveAcpElicitationRequest(SESSION_ID, routedRequest!.id, {})).toBe(false);
    } finally {
      unsubscribe();
    }
  });
});
