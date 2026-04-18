import type { Window as TauriWindow } from '@tauri-apps/api/window';
import type { Update } from '@tauri-apps/plugin-updater';

type SmtpCheckParams = {
  host: string;
  ports: number[];
  email: string;
  password: string;
};

type SmtpSendParams = {
  host: string;
  email: string;
  toEmail: string;
  chatName: string;
  chatId: string;
  localRef: string;
  text: string;
  inReplyTo?: string;
};

type SmtpCredentialsParams = {
  email: string;
  password: string;
};

type SmtpResult = {
  ok: boolean;
  error?: string;
  // Returned by smtp_check: the port that succeeded, saved for smtp_send
  port?: number;
  // All ports timed out — likely blocked by VPN in TUN mode
  allPortsTimedOut?: boolean;
  // Optional payload (e.g. armored public key from smtp_get_public_key)
  data?: string;
};

type TauriApi = {
  version: string;
  markTitleBarOverlay: (isOverlay: boolean) => Promise<void>;
  setNotificationsCount: (amount: number, isMuted?: boolean) => Promise<void>;
  openNewWindow: (url: string) => Promise<void>;
  relaunch: () => Promise<void>;
  checkUpdate: () => Promise<Update | null>;
  getCurrentWindow: () => Promise<TauriWindow>;
  setWindowTitle: (title: string) => Promise<void>;
  smtpCheck: (params: SmtpCheckParams) => Promise<SmtpResult>;
  smtpSend: (params: SmtpSendParams) => Promise<SmtpResult>;
  smtpSaveCredentials: (params: SmtpCredentialsParams) => Promise<SmtpResult>;
  smtpLoadCredentials: (params: { email: string }) => Promise<SmtpResult>;
  smtpDeleteCredentials: (params: { email: string }) => Promise<SmtpResult>;
  smtpGetPublicKey: (params: { email: string }) => Promise<SmtpResult>;
  smtpImportAutocrypt: (params: { autocryptValue: string }) => Promise<SmtpResult>;
  imapStartWatch: (params: { email: string; imapHost: string; imapPort: number }) => Promise<SmtpResult>;
  imapStopWatch: () => Promise<SmtpResult>;
  handshakeSendRequest: (params: { ownEmail: string; smtpHost: string; displayName: string; toEmail: string }) => Promise<SmtpResult>;
  handshakeAccept: (params: { ownEmail: string; smtpHost: string; displayName: string; toEmail: string }) => Promise<SmtpResult>;
};

declare global {
  interface Window {
    tauri: TauriApi;
  }
}

export {};
