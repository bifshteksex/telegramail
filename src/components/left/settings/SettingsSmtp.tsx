import { memo, useState } from '../../../lib/teact/teact';
import { getActions, withGlobal } from '../../../global';

import type { AccountSettings } from '../../../types';

import { IS_TAURI } from '../../../util/browser/globalEnvironment';
import useHistoryBack from '../../../hooks/useHistoryBack';
import useLang from '../../../hooks/useLang';
import useLastCallback from '../../../hooks/useLastCallback';

import { SMTP_PROVIDERS } from '../../../config/smtpProviders';

import Button from '../../ui/Button';
import InputText from '../../ui/InputText';
import Select from '../../ui/Select';

async function checkAndSaveSmtpCredentials(
  host: string, ports: number[], email: string, password: string,
): Promise<{ port: number; vpnBlocked: false } | { vpnBlocked: true }> {
  if (!IS_TAURI) throw new Error('Email transport requires the desktop app');
  const checkResult = await window.tauri.smtpCheck({
    host, ports, email, password,
  });
  if (checkResult.allPortsTimedOut) return { vpnBlocked: true };
  if (!checkResult.ok) throw new Error(checkResult.error ?? 'Unknown error');
  const saveResult = await window.tauri.smtpSaveCredentials({ email, password });
  if (!saveResult.ok) throw new Error(saveResult.error ?? 'Failed to save credentials');
  return { port: checkResult.port!, vpnBlocked: false };
}

type OwnProps = {
  isActive?: boolean;
  onReset: () => void;
};

type StateProps = Pick<AccountSettings, 'smtpProvider' | 'smtpEmail' | 'smtpPort'>;

type SmtpStatus = 'idle' | 'loading' | 'success' | 'error' | 'vpn-blocked';

const SettingsSmtp = ({
  isActive,
  smtpProvider,
  smtpEmail,
  smtpPort,
  onReset,
}: OwnProps & StateProps) => {
  const { setSettingOption } = getActions();
  const lang = useLang();

  const [provider, setProvider] = useState(smtpProvider ?? 'gmail');
  const [email, setEmail] = useState(smtpEmail ?? '');
  const [password, setPassword] = useState('');
  const [status, setStatus] = useState<SmtpStatus>('idle');
  const [errorText, setErrorText] = useState('');

  useHistoryBack({
    isActive,
    onBack: onReset,
  });

  const handleConnect = useLastCallback(async () => {
    const config = SMTP_PROVIDERS[provider];
    if (!config) return;

    setStatus('loading');
    setErrorText('');

    try {
      const result = await checkAndSaveSmtpCredentials(config.host, config.ports, email, password);
      if (result.vpnBlocked) {
        setStatus('vpn-blocked');
        return;
      }
      setSettingOption({ smtpProvider: provider, smtpEmail: email, smtpPort: result.port });
      setStatus('success');
      void window.tauri.imapStartWatch({
        email,
        imapHost: config.imapHost,
        imapPort: config.imapPort,
      });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setErrorText(msg);
      setStatus('error');
    }
  });

  const isConnectDisabled = !email || !password || status === 'loading';
  const isAlreadyConnected = Boolean(smtpEmail && smtpPort);

  return (
    <div className="settings-content custom-scroll">
      <div className="settings-item">
        <Select
          id="smtp-provider"
          label={lang('SmtpProvider')}
          value={provider}
          hasArrow
          onChange={(e) => setProvider(e.target.value)}
        >
          <option value="gmail">{lang('SmtpProviderGmail')}</option>
          <option value="yandex">{lang('SmtpProviderYandex')}</option>
          <option value="mailru">{lang('SmtpProviderMailRu')}</option>
          <option value="outlook">{lang('SmtpProviderOutlook')}</option>
          <option value="custom">{lang('SmtpProviderCustom')}</option>
        </Select>
        <InputText
          value={email}
          label={lang('SmtpEmailAddress')}
          inputMode="email"
          onChange={(e) => setEmail(e.target.value)}
        />
        <div className={`input-group${password ? ' touched' : ''} with-label`}>
          <input
            className="form-control"
            type="password"
            value={password}
            placeholder={lang('SmtpPassword')}
            autoComplete="new-password"
            onChange={(e) => setPassword(e.target.value)}
            aria-label={lang('SmtpPassword')}
          />
          <label>{lang('SmtpPassword')}</label>
        </div>
        {isAlreadyConnected && status === 'idle' && (
          <p className="settings-item-description success">
            {lang('SmtpConnectSuccess')}
          </p>
        )}
        {status === 'success' && (
          <p className="settings-item-description success">{lang('SmtpConnectSuccess')}</p>
        )}
        {status === 'error' && (
          <p className="settings-item-description error">{lang('SmtpConnectError', { error: errorText })}</p>
        )}
        {status === 'vpn-blocked' && (
          <p className="settings-item-description error">{lang('SmtpConnectErrorVpn')}</p>
        )}
      </div>
      <div className="settings-item">
        <Button
          onClick={handleConnect}
          disabled={isConnectDisabled}
          isLoading={status === 'loading'}
        >
          {lang('SmtpConnect')}
        </Button>
      </div>
    </div>
  );
};

export default memo(withGlobal<OwnProps>(
  (global): Complete<StateProps> => {
    const { smtpProvider, smtpEmail, smtpPort } = global.settings.byKey;
    return { smtpProvider, smtpEmail, smtpPort };
  },
)(SettingsSmtp));
