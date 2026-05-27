// ─── App Root ───
// Thin router: delegates navigation and rendering to AppLayout.
// Also syncs the theme from the store to the <html> element.

import { useEffect } from "react";
import { AppLayout } from "./components/Layout/AppLayout";
import { useAppStore } from "./stores";
import "@lichess-org/chessground/assets/chessground.base.css";
import "./chessground.metal.css";
import "./chessground.alpha.css";

export default function App() {
  const theme = useAppStore((s) => s.theme);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    if (theme === "dark") {
      document.documentElement.classList.add("dark");
    } else {
      document.documentElement.classList.remove("dark");
    }
  }, [theme]);

  return <AppLayout />;
}
