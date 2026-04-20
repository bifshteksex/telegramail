import { memo, useState } from '../../../lib/teact/teact';
import { withGlobal } from '../../../global';

import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import SmtpChatList from './SmtpChatList';
import SmtpChatScreen from './SmtpChatScreen';
import SmtpContacts from './SmtpContacts';

import styles from './SmtpMode.module.scss';

interface OwnProps {
  isContactsOpen?: boolean;
  onContactsClose?: NoneToVoidFunction;
}

type StateProps = {
  emailConnectionStatus?: string;
};

type View = { type: 'list' } | { type: 'chat'; chatId: string } | { type: 'contacts' };

const SmtpMode = ({
  emailConnectionStatus, isContactsOpen, onContactsClose,
}: OwnProps & StateProps) => {
  const lang = useLang();

  const [view, setView] = useState<View>({ type: 'list' });

  const handleChatSelect = useLastCallback((chatId: string) => {
    setView({ type: 'chat', chatId });
  });

  const handleShowContacts = useLastCallback(() => {
    setView({ type: 'contacts' });
  });

  const handleBack = useLastCallback(() => {
    setView({ type: 'list' });
    onContactsClose?.();
  });

  const resolvedView: View = isContactsOpen ? { type: 'contacts' } : view;

  return (
    <div className={styles.root}>
      {resolvedView.type === 'list' && (
        <>
          {emailConnectionStatus && emailConnectionStatus !== 'connected' && (
            <div className={styles.statusBanner}>
              {emailConnectionStatus === 'reconnecting'
                ? lang('SmtpReconnecting')
                : lang('SmtpDisconnected')}
            </div>
          )}
          <SmtpChatList
            onChatSelect={handleChatSelect}
            onAddContact={handleShowContacts}
          />
        </>
      )}

      {resolvedView.type === 'chat' && (
        <SmtpChatScreen chatId={resolvedView.chatId} onBack={handleBack} />
      )}

      {resolvedView.type === 'contacts' && (
        <SmtpContacts onBack={handleBack} />
      )}
    </div>
  );
};

export default memo(withGlobal<OwnProps>((global): Complete<StateProps> => {
  return {
    emailConnectionStatus: global.settings.byKey.emailConnectionStatus,
  };
})(SmtpMode));
