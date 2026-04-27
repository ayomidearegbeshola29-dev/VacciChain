import { Routes, Route, Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import Landing from './pages/Landing';
import PatientDashboard from './pages/PatientDashboard';
import IssuerDashboard from './pages/IssuerDashboard';
import VerifyPage from './pages/VerifyPage';
import { AuthProvider } from './hooks/useFreighter';
import LanguageSelector from './components/LanguageSelector';

export default function App() {
  const { t } = useTranslation();

  return (
    <AuthProvider>
      <nav style={{ padding: '1rem 2rem', background: '#1e293b', display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
        <strong style={{ color: '#38bdf8', fontSize: '1.2rem' }}>💉 VacciChain</strong>
        <Link to="/">{t('nav.home')}</Link>
        <Link to="/patient">{t('nav.myRecords')}</Link>
        <Link to="/issuer">{t('nav.issue')}</Link>
        <Link to="/verify">{t('nav.verify')}</Link>
        <LanguageSelector />
      </nav>
      <Routes>
        <Route path="/" element={<Landing />} />
        <Route path="/patient" element={<PatientDashboard />} />
        <Route path="/issuer" element={<IssuerDashboard />} />
        <Route path="/verify" element={<VerifyPage />} />
      </Routes>
    </AuthProvider>
  );
}
