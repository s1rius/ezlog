/** @type {import('tailwindcss').Config} */
export default {
  darkMode: 'media', // This will follow system theme
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      dropShadow: {
        '3xl': '0 35px 35px rgba(0, 0, 0, 0.25)',
      }
    },
    container: {
    },
  },
  plugins: [],
}


