import type React from '../../../lib/teact/teact';
import { memo, useRef, useState } from '../../../lib/teact/teact';
import { getActions, withGlobal } from '../../../global';

import type { EmailChat, EmailMessage } from '../../../types/email';

import buildClassName from '../../../util/buildClassName';

import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import Icon from '../../common/icons/Icon';

import styles from './SmtpChatScreen.module.scss';

type OwnProps = {
  chatId: string;
  onBack: NoneToVoidFunction;
};

type StateProps = {
  chat?: EmailChat;
  messages: EmailMessage[];
};

const SmtpChatScreen = ({
  chatId,
  onBack,
  chat,
  messages,
}: OwnProps & StateProps) => {
  const { sendEmailMessage, markEmailChatRead } = getActions();
  const [text, setText] = useState('');
  const inputRef = useRef<HTMLTextAreaElement>();
  const lang = useLang();

  const handleSend = useLastCallback(() => {
    const trimmed = text.trim();
    if (!trimmed) return;
    sendEmailMessage({ chatId, text: trimmed });
    setText('');
    inputRef.current?.focus();
  });

  const handleKeyDown = useLastCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  });

  const handleFocus = useLastCallback(() => {
    if (chat && (chat.unreadCount ?? 0) > 0) {
      markEmailChatRead({ chatId });
    }
  });

  if (!chat) return undefined;

  return (
    <div className={styles.root}>
      <div className={styles.header}>
        <button className={styles.backButton} onClick={onBack}>
          <Icon name="arrow-left" />
        </button>
        <div className={styles.headerInfo}>
          <span className={styles.headerName}>{chat.displayName}</span>
          <span className={styles.headerEmail}>{chat.email}</span>
        </div>
      </div>

      <div className={styles.messages}>
        {messages.length === 0 && (
          <div className={styles.noMessages}>{lang('SmtpNoMessages')}</div>
        )}
        {messages.map((msg) => (
          <div
            key={msg.id}
            className={buildClassName(styles.message, msg.isOutgoing && styles.outgoing)}
          >
            <div className={styles.bubble}>{msg.text}</div>
          </div>
        ))}
      </div>

      <div className={styles.composer}>
        <textarea
          ref={inputRef}
          className={styles.input}
          value={text}
          placeholder={lang('SmtpTypeMessage')}
          onInput={(e) => setText((e.target as HTMLTextAreaElement).value)}
          onKeyDown={handleKeyDown}
          onFocus={handleFocus}
          rows={1}
        />
        <button
          className={buildClassName(styles.sendButton, !text.trim() && styles.disabled)}
          onClick={handleSend}
          disabled={!text.trim()}
        >
          <Icon name="send" />
        </button>
      </div>
    </div>
  );
};

export default memo(withGlobal<OwnProps>((global, { chatId }): Complete<StateProps> => {
  const chat = global.emailChats.byId[chatId];
  const msgMap = global.emailMessages.byChatId[chatId] ?? {};
  const messages = Object.values(msgMap).sort((a, b) => a.date - b.date);
  return { chat, messages };
})(SmtpChatScreen));
