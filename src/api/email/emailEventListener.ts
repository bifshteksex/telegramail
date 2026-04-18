import type { EmailHandshakePayload, EmailMessagePayload, EmailStatusPayload } from '../../types/email';

import { IS_TAURI } from '../../util/browser/globalEnvironment';
import { getActions } from '../../global';

export function initEmailEventListener() {
  if (!IS_TAURI) return;

  import('@tauri-apps/api/event').then(({ listen }) => {
    listen<EmailMessagePayload>('email:message', ({ payload }) => {
      getActions().receiveEmailMessage({
        from: payload.from,
        text: payload.text,
        messageId: payload.messageId,
      });
    });

    listen<EmailHandshakePayload>('email:handshake', ({ payload }) => {
      getActions().receiveEmailHandshake({
        kind: payload.kind,
        from: payload.from,
        displayName: payload.displayName,
      });
    });

    listen<EmailStatusPayload>('email:status', ({ payload }) => {
      getActions().setEmailConnectionStatus({
        status: payload.status,
        error: payload.error,
      });
    });
  }).catch((err) => {
    // eslint-disable-next-line no-console
    console.error('[email] Failed to set up event listeners:', err);
  });
}
