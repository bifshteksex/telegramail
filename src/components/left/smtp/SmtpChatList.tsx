import { memo, useMemo } from '../../../lib/teact/teact';
import { getActions, withGlobal } from '../../../global';

import type { EmailChat } from '../../../types/email';

import buildClassName from '../../../util/buildClassName';
import { formatDateTimeToString } from '../../../util/dates/oldDateFormat';

import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import styles from './SmtpChatList.module.scss';

type OwnProps = {
  onChatSelect: (chatId: string) => void;
  onAddContact: NoneToVoidFunction;
  selectedChatId?: string;
};

type StateProps = {
  chats: EmailChat[];
};

const SmtpChatList = ({
  onChatSelect,
  onAddContact,
  selectedChatId,
  chats,
}: OwnProps & StateProps) => {
  const lang = useLang();

  const handleChatClick = useLastCallback((chatId: string) => {
    onChatSelect(chatId);
  });

  if (chats.length === 0) {
    return (
      <div className={styles.empty}>
        <p className={styles.emptyText}>{lang('SmtpNoChats')}</p>
        <button className={styles.addButton} onClick={onAddContact}>
          {lang('SmtpAddContact')}
        </button>
      </div>
    );
  }

  return (
    <div className={styles.list}>
      {chats.map((chat) => (
        <div
          key={chat.id}
          className={buildClassName(styles.chatItem, selectedChatId === chat.id && styles.selected)}
          onClick={() => handleChatClick(chat.id)}
          role="button"
          tabIndex={0}
        >
          <div className={styles.avatar}>
            {chat.displayName.charAt(0).toUpperCase()}
          </div>
          <div className={styles.info}>
            <div className={styles.row}>
              <span className={styles.name}>{chat.displayName}</span>
              {chat.lastMessageDate !== undefined && (
                <span className={styles.date}>
                  {formatDateTimeToString(chat.lastMessageDate * 1000, lang.code)}
                </span>
              )}
            </div>
            <div className={styles.row}>
              <span className={styles.lastMessage}>{chat.lastMessageText ?? chat.email}</span>
              {(chat.unreadCount ?? 0) > 0 && (
                <span className={styles.badge}>{chat.unreadCount}</span>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
};

export default memo(withGlobal<OwnProps>((global): Complete<StateProps> => {
  const chats = Object.values(global.emailChats.byId).sort((a, b) => {
    const aDate = a.lastMessageDate ?? 0;
    const bDate = b.lastMessageDate ?? 0;
    return bDate - aDate;
  });
  return { chats };
})(SmtpChatList));
