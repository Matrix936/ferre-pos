import React, { useState } from "react";
import { Box } from "@mui/material";
import { Sidebar } from "./Sidebar";
import { Topbar } from "./Topbar";

export function DashboardLayout({ children }: { children: React.ReactNode }) {
  const [sidebarOpen, setSidebarOpen] = useState(true);

  const closeSidebar = () => {
    setSidebarOpen(false);
  };

  const toggleSidebar = () => {
    setSidebarOpen((prev) => !prev);
  };

  return (
    <Box sx={{ display: 'flex', height: '100vh', overflow: 'hidden', bgcolor: 'background.default', fontFamily: (theme) => theme.typography.fontFamily }}>
      <Sidebar isOpen={sidebarOpen} onClose={closeSidebar} />
      <Box sx={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <Topbar onToggleSidebar={toggleSidebar} />
        <Box component="main" sx={{ flex: 1, overflowX: 'hidden', overflowY: 'auto' }}>
          <Box sx={{ width: '100%', minHeight: '100%', p: { xs: 2, md: 3 } }}>
            {children}
          </Box>
        </Box>
      </Box>
    </Box>
  );
}
