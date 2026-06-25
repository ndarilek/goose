import type { KeyboardEvent } from 'react';

/**
 * onKeyDown handler that activates a control on Enter or Space, matching native
 * button behavior. For elements given `role="button"` that aren't native buttons.
 */
export const activateOnKey = (handler: () => void) => (e: KeyboardEvent) => {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    handler();
  }
};
