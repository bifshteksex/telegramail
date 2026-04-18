type SmtpProviderConfig = {
  host: string;
  // Ports tried in order: first success wins. 465 = implicit TLS, 587 = STARTTLS.
  ports: [number, ...number[]];
  imapHost: string;
  imapPort: number;
  requiresAppPassword?: boolean;
};

export const SMTP_PROVIDERS: Record<string, SmtpProviderConfig> = {
  gmail: {
    host: 'smtp.gmail.com',
    ports: [465, 587],
    imapHost: 'imap.gmail.com',
    imapPort: 993,
    requiresAppPassword: true,
  },
  yandex: {
    host: 'smtp.yandex.ru',
    ports: [465, 587],
    imapHost: 'imap.yandex.ru',
    imapPort: 993,
  },
  mailru: {
    host: 'smtp.mail.ru',
    ports: [465, 587],
    imapHost: 'imap.mail.ru',
    imapPort: 993,
  },
  outlook: {
    host: 'smtp.office365.com',
    ports: [587, 465],
    imapHost: 'outlook.office365.com',
    imapPort: 993,
  },
  custom: {
    host: '',
    ports: [465, 587],
    imapHost: '',
    imapPort: 993,
  },
};
