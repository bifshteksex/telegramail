import { IS_MAC_OS } from '../browser/windowEnvironment';

export default function initTauriApi() {
  const corePromise = import('@tauri-apps/api/core');
  async function markTitleBarOverlay(isOverlay: boolean) {
    if (!IS_MAC_OS) return;
    const core = await corePromise;
    return core.invoke<void>('mark_title_bar_overlay', { isOverlay });
  }

  async function setNotificationsCount(amount: number, isMuted = false) {
    const core = await corePromise;
    return core.invoke<void>('set_notifications_count', { amount, isMuted });
  }

  async function openNewWindow(url: string) {
    const core = await corePromise;
    return core.invoke<boolean>('open_new_window_cmd', { url });
  }

  async function setWindowTitle(title: string) {
    const core = await corePromise;
    return core.invoke<void>('set_window_title', { title });
  }

  async function smtpCheck(params: { host: string; ports: number[]; email: string; password: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string; port?: number }>('smtp_check', params);
  }

  async function smtpSend(params: {
    host: string;
    email: string;
    toEmail: string;
    chatName: string;
    chatId: string;
    localRef: string;
    text: string;
    inReplyTo?: string;
  }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('smtp_send', params);
  }

  async function smtpSaveCredentials(params: { email: string; password: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('smtp_save_credentials', params);
  }

  async function smtpLoadCredentials(params: { email: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('smtp_load_credentials', params);
  }

  async function smtpDeleteCredentials(params: { email: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('smtp_delete_credentials', params);
  }

  async function smtpGetPublicKey(params: { email: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string; data?: string }>('smtp_get_public_key', params);
  }

  async function smtpImportAutocrypt(params: { autocryptValue: string }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('smtp_import_autocrypt', params);
  }

  async function imapStartWatch(params: { email: string; imapHost: string; imapPort: number }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('imap_start_watch', params);
  }

  async function imapStopWatch() {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('imap_stop_watch');
  }

  async function handshakeSendRequest(params: {
    ownEmail: string; smtpHost: string; displayName: string; toEmail: string;
  }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('handshake_send_request', params);
  }

  async function handshakeAccept(params: {
    ownEmail: string; smtpHost: string; displayName: string; toEmail: string;
  }) {
    const core = await corePromise;
    return core.invoke<{ ok: boolean; error?: string }>('handshake_accept', params);
  }

  // @ts-expect-error
  window.tauri ??= {};
  Object.assign(window.tauri, {
    markTitleBarOverlay,
    setNotificationsCount,
    openNewWindow,
    relaunch: () => import('@tauri-apps/plugin-process').then(({ relaunch }) => relaunch()),
    checkUpdate: () => import('@tauri-apps/plugin-updater').then(({ check }) => check()),
    getCurrentWindow: () => import('@tauri-apps/api/window').then(({ getCurrentWindow }) => getCurrentWindow()),
    setWindowTitle,
    smtpCheck,
    smtpSend,
    smtpSaveCredentials,
    smtpLoadCredentials,
    smtpDeleteCredentials,
    smtpGetPublicKey,
    smtpImportAutocrypt,
    imapStartWatch,
    imapStopWatch,
    handshakeSendRequest,
    handshakeAccept,
  });
}
