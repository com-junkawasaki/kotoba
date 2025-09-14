import React, { useEffect, useState } from 'react';
import { KotobaApp } from './utils/componentBuilder';
import { kotobaParser, KotobaConfig } from './utils/kotobaParser';
import './styles.css';

const App: React.FC = () => {
  const [config, setConfig] = useState<KotobaConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadKotobaConfig = async () => {
      try {
        // .kotobaファイルの読み込み
        const response = await fetch('/app.kotoba');
        const text = await response.text();

        // パースして設定を取得
        const parsedConfig = kotobaParser.parse(text);
        setConfig(parsedConfig);
        setLoading(false);
      } catch (err) {
        console.error('Failed to load kotoba config:', err);
        setError('Failed to load application configuration');
        setLoading(false);
      }
    };

    loadKotobaConfig();
  }, []);

  if (loading) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '100vh',
        flexDirection: 'column'
      }}>
        <div className="loading" style={{ marginBottom: '1rem' }}></div>
        <p>Loading Kotoba Application...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '100vh',
        flexDirection: 'column',
        padding: '2rem'
      }}>
        <div className="error">
          <h2>Configuration Error</h2>
          <p>{error}</p>
          <p>Please check that app.kotoba file exists and is properly formatted.</p>
        </div>
      </div>
    );
  }

  if (!config) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '100vh'
      }}>
        <p>No configuration loaded</p>
      </div>
    );
  }

  return (
    <div className="app">
      <KotobaApp config={config} />
    </div>
  );
};

export default App;
