import { useTranslation } from 'react-i18next';

const STELLAR_EXPERT_BASE = 'https://stellar.expert/explorer/testnet/tx';

const overlay = {
  position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.7)',
  display: 'flex', alignItems: 'center', justifyContent: 'center',
  zIndex: 1000, padding: '1rem',
};
const modal = {
  background: '#0f172a', border: '1px solid #334155', borderRadius: 16,
  padding: '2rem', width: '100%', maxWidth: 520, color: '#e2e8f0', position: 'relative',
};
const row = { marginBottom: '1rem' };
const labelStyle = { fontSize: '0.75rem', color: '#64748b', marginBottom: '0.2rem' };
const valueStyle = { fontSize: '0.95rem', color: '#e2e8f0', wordBreak: 'break-all' };
const closeBtn = {
  position: 'absolute', top: '1rem', right: '1rem',
  background: 'none', border: 'none', color: '#94a3b8',
  fontSize: '1.25rem', cursor: 'pointer', lineHeight: 1,
};

export default function RecordDetailModal({ record, onClose }) {
  const { t } = useTranslation();

  if (!record) return null;

  const handleOverlayClick = (e) => { if (e.target === e.currentTarget) onClose(); };
  const handleKeyDown = (e) => { if (e.key === 'Escape') onClose(); };
  const explorerUrl = record.tx_hash ? `${STELLAR_EXPERT_BASE}/${record.tx_hash}` : null;

  const fields = [
    { label: t('modal.vaccineName'), value: record.vaccine_name },
    { label: t('modal.dateAdministered'), value: record.date_administered },
    { label: t('modal.tokenId'), value: `#${record.token_id}` },
    { label: t('modal.issuerAddress'), value: record.issuer },
    ...(record.tx_hash ? [{ label: t('modal.txHash'), value: record.tx_hash }] : []),
  ];

  return (
    <div
      style={overlay}
      onClick={handleOverlayClick}
      onKeyDown={handleKeyDown}
      role="dialog"
      aria-modal="true"
      aria-label={t('modal.ariaLabel')}
      tabIndex={-1}
      ref={(el) => el?.focus()}
    >
      <div style={modal}>
        <button style={closeBtn} onClick={onClose} aria-label={t('modal.close')}>✕</button>
        <h2 style={{ marginBottom: '1.5rem', color: '#38bdf8', fontSize: '1.2rem' }}>
          {t('modal.title')}
        </h2>
        {fields.map(({ label, value }) => (
          <div key={label} style={row}>
            <p style={labelStyle}>{label}</p>
            <p style={valueStyle}>{value}</p>
          </div>
        ))}
        <div style={{ marginTop: '1.5rem' }}>
          {explorerUrl ? (
            <a
              href={explorerUrl}
              target="_blank"
              rel="noopener noreferrer"
              style={{ display: 'inline-block', padding: '0.6rem 1.25rem', background: '#0ea5e9', color: '#fff', borderRadius: 8, textDecoration: 'none', fontSize: '0.9rem' }}
            >
              {t('modal.viewExplorer')}
            </a>
          ) : (
            <p style={{ color: '#475569', fontSize: '0.85rem' }}>{t('modal.noTxHash')}</p>
          )}
        </div>
      </div>
    </div>
  );
}
