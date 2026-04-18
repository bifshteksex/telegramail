import type { EmailChat, EmailContact, EmailMessage } from '../../../types/email';
import type { ActionReturnType, GlobalState, TabArgs } from '../../types';

import { IS_TAURI } from '../../../util/browser/globalEnvironment';
import { getCurrentTabId } from '../../../util/establishMultitabRole';
import { SMTP_PROVIDERS } from '../../../config/smtpProviders';
import { addActionHandler, getActions, getGlobal, setGlobal } from '../../index';
import { addEmailMessage, markEmailChatRead, upsertEmailChat, upsertEmailContact } from '../../reducers/email';
import { replaceSettings } from '../../reducers/settings';
import { updateTabState } from '../../reducers/tabs';
import { selectTabState } from '../../selectors';

function emailToChatId(email: string): string {
  return `email:${email}`;
}

addActionHandler('openSmtpMode', (global, actions, _payload, ...[tabId = getCurrentTabId()]): ActionReturnType => {
  return updateTabState(global, { isSmtpMode: true }, tabId);
});

addActionHandler('closeSmtpMode', (global, actions, _payload, ...[tabId = getCurrentTabId()]): ActionReturnType => {
  return updateTabState(global, { isSmtpMode: false }, tabId);
});

addActionHandler('setEmailConnectionStatus', (global, actions, payload): ActionReturnType => {
  const { status } = payload;
  return replaceSettings(global, { emailConnectionStatus: status });
});

addActionHandler('sendEmailContactRequest', async (global, actions, payload): Promise<void> => {
  if (!IS_TAURI) return;

  const { toEmail, smtpHost, displayName } = payload;

  global = upsertEmailContact(getGlobal(), {
    email: toEmail,
    displayName: toEmail,
    status: 'pending_out',
  });
  setGlobal(global);

  try {
    await window.tauri.handshakeSendRequest({ ownEmail: global.settings.byKey.smtpEmail!, smtpHost, displayName, toEmail });
  } catch {
    // Fire-and-forget
  }
});

addActionHandler('acceptEmailContactRequest', async (global, actions, payload): Promise<void> => {
  if (!IS_TAURI) return;

  const { fromEmail, fromDisplayName, smtpHost, displayName } = payload;

  // Update contact status to confirmed.
  global = upsertEmailContact(getGlobal(), {
    email: fromEmail,
    displayName: fromDisplayName,
    status: 'confirmed',
    confirmedAt: Math.floor(Date.now() / 1000),
  });

  // Create email chat for this contact.
  const chatId = emailToChatId(fromEmail);
  global = upsertEmailChat(global, {
    id: chatId,
    email: fromEmail,
    displayName: fromDisplayName,
    unreadCount: 0,
  });
  setGlobal(global);

  try {
    await window.tauri.handshakeAccept({ ownEmail: global.settings.byKey.smtpEmail!, smtpHost, displayName, toEmail: fromEmail });
  } catch {
    // Fire-and-forget
  }
});

addActionHandler('receiveEmailHandshake', (global, actions, payload): ActionReturnType => {
  const { kind, from, displayName } = payload;
  const name = displayName ?? from;

  if (kind === 'request') {
    // Store as pending_in — user will see a notification and can accept.
    global = upsertEmailContact(global, {
      email: from,
      displayName: name,
      status: 'pending_in',
    });
    return global;
  }

  if (kind === 'accept') {
    // The other side accepted our request — mark confirmed and create chat.
    global = upsertEmailContact(global, {
      email: from,
      displayName: name,
      status: 'confirmed',
      confirmedAt: Math.floor(Date.now() / 1000),
    });

    const chatId = emailToChatId(from);
    global = upsertEmailChat(global, {
      id: chatId,
      email: from,
      displayName: name,
      unreadCount: 0,
    });
    return global;
  }

  return undefined;
});

addActionHandler('receiveEmailMessage', (global, actions, payload): ActionReturnType => {
  const { from, text, messageId } = payload;

  const contact = global.emailContacts.byEmail[from];
  if (!contact || contact.status !== 'confirmed') {
    return undefined;
  }

  const chatId = emailToChatId(from);

  // Ensure chat exists.
  if (!global.emailChats.byId[chatId]) {
    global = upsertEmailChat(global, {
      id: chatId,
      email: from,
      displayName: contact.displayName,
      unreadCount: 0,
    });
  }

  const message: EmailMessage = {
    id: messageId || `email-${Date.now()}`,
    chatId,
    fromEmail: from,
    text,
    date: Math.floor(Date.now() / 1000),
    isOutgoing: false,
    isEncrypted: false,
    emailMessageId: messageId,
  };

  global = addEmailMessage(global, message);
  global = upsertEmailChat(global, {
    ...global.emailChats.byId[chatId],
    lastMessageText: text,
    lastMessageDate: message.date,
    unreadCount: (global.emailChats.byId[chatId]?.unreadCount ?? 0) + 1,
  });

  return global;
});

addActionHandler('sendEmailMessage', async (global, actions, payload): Promise<void> => {
  if (!IS_TAURI) return;

  const { chatId, text } = payload;
  const chat = global.emailChats.byId[chatId];
  if (!chat) return;

  const { smtpProvider, smtpEmail } = global.settings.byKey;
  if (!smtpEmail || !smtpProvider) return;

  const providerConfig = SMTP_PROVIDERS[smtpProvider];
  if (!providerConfig) return;

  const messageId = `email-out-${Date.now()}`;

  // Optimistically add outgoing message to state.
  const outgoing: EmailMessage = {
    id: messageId,
    chatId,
    fromEmail: smtpEmail,
    text,
    date: Math.floor(Date.now() / 1000),
    isOutgoing: true,
    isEncrypted: false,
    emailMessageId: messageId,
  };

  global = getGlobal();
  global = addEmailMessage(global, outgoing);
  global = upsertEmailChat(global, {
    ...global.emailChats.byId[chatId],
    lastMessageText: text,
    lastMessageDate: outgoing.date,
  });
  setGlobal(global);

  try {
    const result = await window.tauri.smtpSend({
      host: providerConfig.host,
      email: smtpEmail,
      toEmail: chat.email,
      chatName: chat.displayName,
      chatId,
      localRef: messageId,
      text,
    });
    if (!result.ok) {
      // eslint-disable-next-line no-console
      console.error('[sendEmailMessage] smtp_send error:', result.error);
    }
  } catch (e) {
    // eslint-disable-next-line no-console
    console.error('[sendEmailMessage] invoke error:', e);
  }
});

addActionHandler('markEmailChatRead', (global, actions, payload): ActionReturnType => {
  return markEmailChatRead(global, payload.chatId);
});
