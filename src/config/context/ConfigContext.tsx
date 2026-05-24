import React, { createContext, useContext, useState, useEffect } from 'react';

interface ConfigContextType {
  systemName: string;
  setSystemName: (name: string) => void;
  logo: string | null;
  setLogo: (logo: string | null) => void;
  logoAnimationEnabled: boolean;
  setLogoAnimationEnabled: (enabled: boolean) => void;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export function ConfigProvider({ children }: { children: React.ReactNode }) {
  const [systemName, setSystemName] = useState(() => {
    return localStorage.getItem('systemName') || 'Ferre-POS';
  });
  
  const [logo, setLogo] = useState<string | null>(() => {
    return localStorage.getItem('systemLogo') || null;
  });

  const [logoAnimationEnabled, setLogoAnimationEnabled] = useState(() => {
    return localStorage.getItem('logoAnimationEnabled') !== 'false';
  });

  useEffect(() => {
    localStorage.setItem('systemName', systemName);
  }, [systemName]);

  useEffect(() => {
    if (logo) {
      localStorage.setItem('systemLogo', logo);
    } else {
      localStorage.removeItem('systemLogo');
    }
  }, [logo]);

  useEffect(() => {
    localStorage.setItem('logoAnimationEnabled', String(logoAnimationEnabled));
  }, [logoAnimationEnabled]);

  return (
    <ConfigContext.Provider value={{ systemName, setSystemName, logo, setLogo, logoAnimationEnabled, setLogoAnimationEnabled }}>
      {children}
    </ConfigContext.Provider>
  );
}

export function useConfig() {
  const context = useContext(ConfigContext);
  if (context === undefined) {
    throw new Error('useConfig must be used within a ConfigProvider');
  }
  return context;
}
