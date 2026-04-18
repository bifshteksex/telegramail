import { memo, useState } from '../../../lib/teact/teact';
import { getActions, withGlobal } from '../../../global';

import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import Icon from '../../common/icons/Icon';
import SmtpChatList from './SmtpChatList';
import SmtpChatScreen from './SmtpChatScreen';
import SmtpContacts from './SmtpContacts';

import styles from './SmtpMode.module.scss';

// eslint-disable-next-line @typescript-eslint/no-empty-interface
interface OwnProps {}

type StateProps = {
  emailConnectionStatus?: string;
  pendingInCount: number;
};

type View = { type: 'list' } | { type: 'chat'; chatId: string } | { type: 'contacts' };

const SmtpMode = ({ emailConnectionStatus, pendingInCount }: OwnProps & StateProps) => {
  const { closeSmtpMode } = getActions();
  const lang = useLang();

  const [view, setView] = useState<View>({ type: 'list' });

  const handleChatSelect = useLastCallback((chatId: string) => {
    setView({ type: 'chat', chatId });
  });

  const handleAddContact = useLastCallback(() => {
    setView({ type: 'contacts' });
  });

  const handleBack = useLastCallback(() => {
    setView({ type: 'list' });
  });

  return (
    <div className={styles.root}>
      {view.type === 'list' && (
        <>
          <div className={styles.header}>
            <button className={styles.closeButton} onClick={closeSmtpMode}>
              <Icon name="close" />
            </button>
            <span className={styles.title}>{lang('SmtpModeTitle')}</span>
            <div className={styles.headerRight}>
              {pendingInCount > 0 && (
                <span className={styles.pendingBadge}>{pendingInCount}</span>
              )}
              <button className={styles.contactsButton} onClick={handleAddContact}>
                <Icon name="add-user" />
              </button>
            </div>
          </div>
          {emailConnectionStatus && emailConnectionStatus !== 'connected' && (
            <div className={styles.statusBanner}>
              {emailConnectionStatus === 'reconnecting'
                ? lang('SmtpReconnecting')
                : lang('SmtpDisconnected')}
            </div>
          )}
          <SmtpChatList
            onChatSelect={handleChatSelect}
            onAddContact={handleAddContact}
          />
        </>
      )}

      {view.type === 'chat' && (
        <SmtpChatScreen chatId={view.chatId} onBack={handleBack} />
      )}

      {view.type === 'contacts' && (
        <SmtpContacts onBack={handleBack} />
      )}
    </div>
  );
};

export default memo(withGlobal<OwnProps>((global): Complete<StateProps> => {
  const contacts = Object.values(global.emailContacts.byEmail);
  return {
    emailConnectionStatus: global.settings.byKey.emailConnectionStatus,
    pendingInCount: contacts.filter((c) => c.status === 'pending_in').length,
  };
})(SmtpMode));
