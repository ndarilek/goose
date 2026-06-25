import { describe, it, expect, vi } from 'vitest';
import type { KeyboardEvent } from 'react';
import { activateOnKey } from './a11y';

const keyEvent = (key: string): KeyboardEvent => {
  return { key, preventDefault: vi.fn() } as unknown as KeyboardEvent;
};

describe('activateOnKey', () => {
  it('activates on Enter and prevents default', () => {
    const handler = vi.fn();
    const event = keyEvent('Enter');

    activateOnKey(handler)(event);

    expect(handler).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
  });

  it('activates on Space and prevents default', () => {
    const handler = vi.fn();
    const event = keyEvent(' ');

    activateOnKey(handler)(event);

    expect(handler).toHaveBeenCalledTimes(1);
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
  });

  it.each(['Tab', 'ArrowDown', 'Escape', 'a'])(
    'ignores "%s" without activating or preventing default',
    (key) => {
      const handler = vi.fn();
      const event = keyEvent(key);

      activateOnKey(handler)(event);

      expect(handler).not.toHaveBeenCalled();
      expect(event.preventDefault).not.toHaveBeenCalled();
    }
  );
});
