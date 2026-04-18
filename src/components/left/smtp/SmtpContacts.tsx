import { memo, useEffect, useRef, useState } from '../../../lib/teact/teact';
import { getActions, withGlobal } from '../../../global';

import type { EmailContact } from '../../../types/email';

import { IS_TAURI } from '../../../util/browser/globalEnvironment';
import { SMTP_PROVIDERS } from '../../../config/smtpProviders';
import buildClassName from '../../../util/buildClassName';

import useFlag from '../../../hooks/useFlag';
import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import Icon from '../../common/icons/Icon';

import styles from './SmtpContacts.module.scss';

const QR_SIZE = 220;

let qrCodeStylingPromise: Promise<typeof import('qr-code-styling')> | undefined;
function ensureQrCodeStyling() {
  if (!qrCodeStylingPromise) {
    qrCodeStylingPromise = import('qr-code-styling');
  }
  return qrCodeStylingPromise;
}

type OwnProps = {
  onBack: NoneToVoidFunction;
};

type StateProps = {
  ownEmail?: string;
  smtpProvider?: string;
  displayName?: string;
  pendingIn: EmailContact[];
  confirmed: EmailContact[];
};

const SmtpContacts = ({
  onBack,
  ownEmail,
  smtpProvider,
  displayName,
  pendingIn,
  confirmed,
}: OwnProps & StateProps) => {
  const { sendEmailContactRequest, acceptEmailContactRequest } = getActions();
  const lang = useLang();

  const [newEmail, setNewEmail] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [isQrVisible, showQr, hideQr] = useFlag(false);
  const qrContainerRef = useRef<HTMLDivElement>();

  const smtpHost = smtpProvider ? SMTP_PROVIDERS[smtpProvider]?.host : undefined;

  const handleSendRequest = useLastCallback(async () => {
    const trimmed = newEmail.trim();
    if (!trimmed || !smtpHost || !displayName) return;
    setIsSending(true);
    sendEmailContactRequest({ toEmail: trimmed, smtpHost, displayName });
    setNewEmail('');
    setIsSending(false);
  });

  const handleAccept = useLastCallback((contact: EmailContact) => {
    if (!smtpHost || !displayName) return;
    acceptEmailContactRequest({
      fromEmail: contact.email,
      fromDisplayName: contact.displayName,
      smtpHost,
      displayName,
    });
  });

  const handleShowQr = useLastCallback(() => {
    showQr();
  });

  useEffect(() => {
    if (!isQrVisible || !ownEmail || !qrContainerRef.current) return;

    const container = qrContainerRef.current;
    container.innerHTML = '';

    async function renderQr() {
      let deepLink = `telegramail://handshake?email=${encodeURIComponent(ownEmail!)}`;
      if (displayName) deepLink += `&name=${encodeURIComponent(displayName)}`;

      if (IS_TAURI) {
        try {
          const result = await window.tauri.smtpGetPublicKey({ email: ownEmail! });
          if (result.ok && result.data) {
            deepLink += `&pgp=${encodeURIComponent(result.data)}`;
          }
        } catch {
          // proceed without public key
        }
      }

      const QrCodeStyling = (await ensureQrCodeStyling()).default;
      const qr = new QrCodeStyling({
        width: QR_SIZE,
        height: QR_SIZE,
        data: deepLink,
        margin: 10,
        type: 'svg',
        dotsOptions: { type: 'rounded' },
        cornersSquareOptions: { type: 'extra-rounded' },
        qrOptions: { errorCorrectionLevel: 'M' },
      });

      if (container) {
        qr.append(container);
      }
    }

    void renderQr();
  }, [isQrVisible, ownEmail, displayName]);

  const isAddDisabled = !newEmail.trim() || !smtpHost || isSending;

  return (
    <div className={styles.root}>
      <div className={styles.header}>
        <button className={styles.backButton} onClick={onBack}>
          <Icon name="arrow-left" />
        </button>
        <span className={styles.title}>{lang('SmtpContacts')}</span>
      </div>

      <div className={styles.content}>
        <div className={styles.section}>
          <h3 className={styles.sectionTitle}>{lang('SmtpAddContact')}</h3>
          <p className={styles.sectionHint}>{lang('SmtpAddContactHint')}</p>
          <div className={styles.addRow}>
            <input
              className={styles.emailInput}
              type="email"
              placeholder={lang('SmtpContactEmailPlaceholder')}
              value={newEmail}
              onInput={(e) => setNewEmail((e.target as HTMLInputElement).value)}
            />
            <button
              className={buildClassName(styles.sendBtn, isAddDisabled && styles.disabled)}
              onClick={handleSendRequest}
              disabled={isAddDisabled}
            >
              {lang('SmtpSendRequest')}
            </button>
          </div>
        </div>

        {ownEmail && (
          <div className={styles.section}>
            <h3 className={styles.sectionTitle}>{lang('SmtpShareContact')}</h3>
            <p className={styles.sectionHint}>{lang('SmtpShareContactHint')}</p>
            {!isQrVisible ? (
              <button className={styles.sendBtn} onClick={handleShowQr}>
                {lang('SmtpShowQr')}
              </button>
            ) : (
              <div className={styles.qrWrapper}>
                <div ref={qrContainerRef} className={styles.qrContainer} />
                <button className={buildClassName(styles.sendBtn, styles.qrClose)} onClick={hideQr}>
                  {lang('SmtpHideQr')}
                </button>
              </div>
            )}
          </div>
        )}

        {pendingIn.length > 0 && (
          <div className={styles.section}>
            <h3 className={styles.sectionTitle}>{lang('SmtpPendingRequests')}</h3>
            {pendingIn.map((contact) => (
              <div key={contact.email} className={styles.contactRow}>
                <div className={styles.contactInfo}>
                  <span className={styles.contactName}>{contact.displayName}</span>
                  <span className={styles.contactEmail}>{contact.email}</span>
                </div>
                <button className={styles.acceptBtn} onClick={() => handleAccept(contact)}>
                  {lang('SmtpAccept')}
                </button>
              </div>
            ))}
          </div>
        )}

        {confirmed.length > 0 && (
          <div className={styles.section}>
            <h3 className={styles.sectionTitle}>{lang('SmtpConfirmedContacts')}</h3>
            {confirmed.map((contact) => (
              <div key={contact.email} className={styles.contactRow}>
                <div className={styles.contactInfo}>
                  <span className={styles.contactName}>{contact.displayName}</span>
                  <span className={styles.contactEmail}>{contact.email}</span>
                </div>
                <Icon name="check-bold" className={styles.checkIcon} />
              </div>
            ))}
          </div>
        )}

        {pendingIn.length === 0 && confirmed.length === 0 && (
          <p className={styles.noContacts}>{lang('SmtpNoContacts')}</p>
        )}
      </div>
    </div>
  );
};

export default memo(withGlobal<OwnProps>((global): Complete<StateProps> => {
  const contacts = Object.values(global.emailContacts.byEmail);
  return {
    ownEmail: global.settings.byKey.smtpEmail,
    smtpProvider: global.settings.byKey.smtpProvider,
    displayName: global.currentUserId ? global.users.byId[global.currentUserId]?.firstName : undefined,
    pendingIn: contacts.filter((c) => c.status === 'pending_in'),
    confirmed: contacts.filter((c) => c.status === 'confirmed'),
  };
})(SmtpContacts));
