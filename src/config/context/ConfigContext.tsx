import React, { createContext, useContext, useState, useEffect } from 'react';

interface ConfigContextType {
  systemName: string;
  setSystemName: (name: string) => void;
  logo: string | null;
  setLogo: (logo: string | null) => void;
}

const ConfigContext = createContext<ConfigContextType | undefined>(undefined);

export function ConfigProvider({ children }: { children: React.ReactNode }) {
  const [systemName, setSystemName] = useState(() => {
    return localStorage.getItem('systemName') || 'Ferre-POS';
  });
  
  const [logo, setLogo] = useState<string | null>(() => {
    return localStorage.getItem('systemLogo') || null;
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

  return (
    <ConfigContext.Provider value={{ systemName, setSystemName, logo, setLogo }}>
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
