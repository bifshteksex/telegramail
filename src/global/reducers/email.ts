import type { EmailChat, EmailContact, EmailMessage } from '../../types/email';
import type { GlobalState } from '../types';

export function upsertEmailContact<T extends GlobalState>(
  global: T,
  contact: EmailContact,
): T {
  return {
    ...global,
    emailContacts: {
      byEmail: {
        ...global.emailContacts.byEmail,
        [contact.email]: contact,
      },
    },
  };
}

export function removeEmailContact<T extends GlobalState>(
  global: T,
  email: string,
): T {
  const byEmail = { ...global.emailContacts.byEmail };
  delete byEmail[email];
  return {
    ...global,
    emailContacts: { byEmail },
  };
}

export function upsertEmailChat<T extends GlobalState>(
  global: T,
  chat: EmailChat,
): T {
  return {
    ...global,
    emailChats: {
      byId: {
        ...global.emailChats.byId,
        [chat.id]: chat,
      },
    },
  };
}

export function addEmailMessage<T extends GlobalState>(
  global: T,
  message: EmailMessage,
): T {
  const existing = global.emailMessages.byChatId[message.chatId] ?? {};
  return {
    ...global,
    emailMessages: {
      byChatId: {
        ...global.emailMessages.byChatId,
        [message.chatId]: {
          ...existing,
          [message.id]: message,
        },
      },
    },
  };
}

export function markEmailChatRead<T extends GlobalState>(
  global: T,
  chatId: string,
): T {
  const chat = global.emailChats.byId[chatId];
  if (!chat) return global;
  return upsertEmailChat(global, { ...chat, unreadCount: 0 });
}
