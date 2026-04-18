export type EmailContactStatus = 'pending_out' | 'pending_in' | 'confirmed';

export type EmailContact = {
  email: string;
  displayName: string;
  status: EmailContactStatus;
  confirmedAt?: number;
};

export type EmailMessage = {
  id: string;
  chatId: string;
  fromEmail: string;
  text: string;
  date: number;
  isOutgoing: boolean;
  isEncrypted: boolean;
  emailMessageId: string;
};

export type EmailChat = {
  id: string;
  email: string;
  displayName: string;
  lastMessageText?: string;
  lastMessageDate?: number;
  unreadCount: number;
};

// Payloads emitted from Rust via Tauri events

export type EmailMessagePayload = {
  from: string;
  text: string;
  telegramRef?: string;
  messageId: string;
  chatGroupName?: string;
  inReplyTo?: string;
  autocrypt?: string;
};

export type EmailHandshakePayload = {
  kind: 'request' | 'accept';
  from: string;
  displayName?: string;
};

export type EmailStatusPayload = {
  status: 'connected' | 'reconnecting' | 'disconnected';
  error?: string;
};
