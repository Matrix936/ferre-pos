/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        industrial: {
          dark: "#1e293b",
          primary: "#3b82f6",
          accent: "#f59e0b",
        }
      }
    },
  },
  plugins: [],
}